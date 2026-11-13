//! API-слой Polygram: HTTP REST + MCP поверх ядра.
//!
//! Ответственность разнесена по файлам:
//! - `service.rs` — `ApiService` и `ApiServiceInner`: жизненный цикл, рантайм, чат, синхронизация, команды.
//! - `document.rs` — JSON-документ (.pg): экспорт/импорт/сохранение/загрузка.
//! - `import.rs` — импорт документа как отдела, уникальные ID, топологическая сортировка.
//! - `backup.rs` — ротация резервных копий и атомарная запись.

pub mod backup;
pub mod chat;
pub mod document;
pub mod http;
pub mod import;
pub mod mcp;
pub mod sync;

use polygram_core::{
    runtime::EngineRuntime, Command, Event, ObjectView, Stats,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use crate::chat::ChatService;

/// Главный сервис API. Оборачивает EngineRuntime и предоставляет
/// срезы состояния и выполнение команд.
#[derive(Clone)]
pub struct ApiService {
    inner: Arc<ApiServiceInner>,
}

struct ApiServiceInner {
    runtime: EngineRuntime,
    /// Лента чата (общая для всех участников).
    pub chat: ChatService,
    /// Сервис синхронизации показателей с внешними API.
    pub sync: sync::SyncService,
    /// Путь к файлу автосохранения (None — отключено).
    auto_save_path: Mutex<Option<PathBuf>>,
    /// Флаги управления жизненным циклом сервера.
    restart_flag: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
    port_cell: Arc<AtomicU16>,
}

impl ApiService {
    pub fn new(engine: polygram_core::Engine) -> Self {
        let runtime = EngineRuntime::spawn(engine);
        let handle = runtime.handle();
        let sync = sync::SyncService::new(handle);
        Self {
            inner: Arc::new(ApiServiceInner {
                runtime,
                chat: ChatService::new(),
                sync,
                auto_save_path: Mutex::new(None),
                restart_flag: Arc::new(AtomicBool::new(false)),
                shutdown_flag: Arc::new(AtomicBool::new(false)),
                port_cell: Arc::new(AtomicU16::new(8080)),
            }),
        }
    }

    /// Лента чата.
    pub fn chat(&self) -> ChatService {
        self.inner.chat.clone()
    }

    /// Флаг перезапуска сервера.
    pub fn restart_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.inner.restart_flag)
    }

    /// Флаг остановки сервера.
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.inner.shutdown_flag)
    }

    /// Ячейка текущего порта.
    pub fn port_cell(&self) -> Arc<AtomicU16> {
        Arc::clone(&self.inner.port_cell)
    }

    pub fn handle(&self) -> polygram_core::runtime::EngineHandle {
        self.inner.runtime.handle()
    }

    /// Сервис синхронизации.
    pub fn sync(&self) -> &sync::SyncService {
        &self.inner.sync
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
                std::process::exit(0);
            }
            Command::ChangePort { port } => {
                self.inner.port_cell.store(*port, Ordering::SeqCst);
                return Ok(polygram_core::command::Applied {
                    rev: self.snapshot().rev,
                    created: None,
                    at: None,
                });
            }
            Command::Shutdown => {
                std::process::exit(0);
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

        // Информация о создании объекта для ленты чата извлекается до перемещения команды.
        let object_note = match &cmd {
            Command::AddObject {
                author,
                name,
                kind,
                at,
                ..
            } => author.as_ref().map(|author| {
                (
                    author.clone(),
                    name.clone(),
                    *kind,
                    at.unwrap_or([0, 0]),
                )
            }),
            _ => None,
        };

        let result = self.inner.runtime.handle().apply(cmd);
        if let Ok(applied) = &result {
            let _ = self.maybe_save();
            if let Some((author, name, kind, at)) = object_note {
                let object_id = applied.created.as_deref().unwrap_or(&name).to_owned();
                self.inner
                    .chat
                    .note_object_created(author, name, object_id, kind, at);
            }
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
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new(polygram_core::Engine::new())
    }
}

pub(crate) use crate::backup::atomic_save_with_backup;
pub use crate::backup::{backup_path, MAX_BACKUPS};
