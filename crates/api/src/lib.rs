//! API-слой Polygram: HTTP REST + MCP поверх ядра.
//! Предоставляет срезы контекста и выполнение команд.

pub mod http;
pub mod mcp;

use polygram_core::{
    document::Document, error::EngineError, runtime::EngineRuntime, Command, Event, ObjectView,
    Stats,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Канал управления жизненным циклом API-сервера.
#[derive(Clone)]
pub struct AdminTx {
    pub restart: mpsc::UnboundedSender<()>,
    pub change_port: mpsc::UnboundedSender<u16>,
    pub shutdown: mpsc::UnboundedSender<()>,
}

pub struct AdminRx {
    pub restart: mpsc::UnboundedReceiver<()>,
    pub change_port: mpsc::UnboundedReceiver<u16>,
    pub shutdown: mpsc::UnboundedReceiver<()>,
}

/// Главный сервис API. Оборачивает EngineRuntime и предоставляет
/// срезы состояния и выполнение команд.
#[derive(Clone)]
pub struct ApiService {
    inner: Arc<ApiServiceInner>,
}

struct ApiServiceInner {
    runtime: EngineRuntime,
    /// Путь к файлу автосохранения (None — отключено).
    auto_save_path: Mutex<Option<PathBuf>>,
    /// Канал управления сервером (устанавливается из main.rs).
    admin_tx: Mutex<Option<AdminTx>>,
}

/// Пара админ-каналов: Tx уходит в ApiService, Rx — в основной цикл main.rs.
pub fn admin_channels() -> (AdminTx, AdminRx) {
    let (restart_tx, restart_rx) = mpsc::unbounded_channel();
    let (port_tx, port_rx) = mpsc::unbounded_channel();
    let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
    (
        AdminTx {
            restart: restart_tx,
            change_port: port_tx,
            shutdown: shutdown_tx,
        },
        AdminRx {
            restart: restart_rx,
            change_port: port_rx,
            shutdown: shutdown_rx,
        },
    )
}

impl ApiService {
    pub fn new(engine: polygram_core::Engine) -> Self {
        let runtime = EngineRuntime::spawn(engine);
        Self {
            inner: Arc::new(ApiServiceInner {
                runtime,
                auto_save_path: Mutex::new(None),
                admin_tx: Mutex::new(None),
            }),
        }
    }

    /// Установить канал управления сервером (вызывается из main.rs перед стартом HTTP).
    pub fn set_admin_tx(&self, admin_tx: AdminTx) {
        *self.inner.admin_tx.lock().unwrap() = Some(admin_tx);
    }

    pub fn handle(&self) -> polygram_core::runtime::EngineHandle {
        self.inner.runtime.handle()
    }

    pub fn snapshot(&self) -> Arc<polygram_core::runtime::Published> {
        self.inner.runtime.handle().latest()
    }

    pub fn stats(&self) -> Option<Stats> {
        self.inner.runtime.handle().query(|e| e.stats())
    }

    pub fn objects(&self) -> Vec<ObjectView> {
        let handle = self.inner.runtime.handle();
        let ids = handle.query(|e| e.object_ids()).unwrap_or_default();
        ids.into_iter()
            .filter_map(|id| handle.query(move |e| e.object_view(&id)))
            .flatten()
            .collect()
    }

    pub fn object(&self, id: String) -> Option<ObjectView> {
        self.inner
            .runtime
            .handle()
            .query(move |e| e.object_view(&id))
            .flatten()
    }

    /// Лента событий отдела: собственные события + события всех дочерних отделов.
    pub fn department_events(
        &self,
        dept: String,
    ) -> Option<Result<Vec<Event>, polygram_core::error::EngineError>> {
        self.inner
            .runtime
            .handle()
            .query(move |e| e.department_events(&dept))
    }

    pub fn apply(
        &self,
        cmd: Command,
    ) -> Result<polygram_core::command::Applied, polygram_core::error::EngineError> {
        // Админ-команды не модифицируют ядро — они управляют жизненным циклом сервера.
        match &cmd {
            Command::RestartServer => {
                if let Some(tx) = &*self.inner.admin_tx.lock().unwrap() {
                    let _ = tx.restart.send(());
                }
                return Ok(polygram_core::command::Applied {
                    rev: self.snapshot().rev,
                    created: None,
                    at: None,
                });
            }
            Command::ChangePort { port } => {
                if let Some(tx) = &*self.inner.admin_tx.lock().unwrap() {
                    let _ = tx.change_port.send(*port);
                }
                return Ok(polygram_core::command::Applied {
                    rev: self.snapshot().rev,
                    created: None,
                    at: None,
                });
            }
            Command::Shutdown => {
                if let Some(tx) = &*self.inner.admin_tx.lock().unwrap() {
                    let _ = tx.shutdown.send(());
                }
                return Ok(polygram_core::command::Applied {
                    rev: self.snapshot().rev,
                    created: None,
                    at: None,
                });
            }
            Command::ReloadConfig => {
                return Ok(polygram_core::command::Applied {
                    rev: self.snapshot().rev,
                    created: None,
                    at: None,
                });
            }
            _ => {}
        }

        let result = self.inner.runtime.handle().apply(cmd);
        if result.is_ok() {
            let _ = self.maybe_save();
        }
        result
    }

    pub fn apply_batch(
        &self,
        cmds: Vec<Command>,
    ) -> Result<Vec<polygram_core::command::Applied>, (usize, polygram_core::error::EngineError)>
    {
        let result = self.inner.runtime.handle().apply_batch(cmds);
        if result.is_ok() {
            let _ = self.maybe_save();
        }
        result
    }

    fn maybe_save(&self) {
        let path = self.inner.auto_save_path.lock().unwrap().clone();
        if let Some(path) = path {
            let _ = self.save_to_file(&path);
        }
    }

    /// Установить путь к файлу автосохранения.
    pub fn set_auto_save_path(&self, path: PathBuf) {
        *self.inner.auto_save_path.lock().unwrap() = Some(path);
    }

    /// Экспортировать состояние ядра в JSON-строку (формат .pg).
    pub fn to_json(&self) -> String {
        self.inner
            .runtime
            .handle()
            .query(|e| e.to_json())
            .unwrap_or_default()
    }

    /// Сохранить состояние ядра в .pg файл.
    pub fn save_to_file(&self, path: &Path) -> Result<(), EngineError> {
        let json = self
            .inner
            .runtime
            .handle()
            .query(|e| e.to_json())
            .ok_or(EngineError::Stopped)?;
        std::fs::write(path, json).map_err(|e| EngineError::Document(e.to_string()))
    }

    /// Загрузить состояние ядра из .pg файла (создаёт новый ApiService с загруженным движком).
    pub fn load_from_file(path: &Path) -> Result<Self, EngineError> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| EngineError::Document(e.to_string()))?;
        let doc = Document::parse(&json)?;
        let engine = polygram_core::Engine::from_document(&doc)?;
        Ok(Self::new(engine))
    }
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new(polygram_core::Engine::new())
    }
}
