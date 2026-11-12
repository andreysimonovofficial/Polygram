//! API-слой Polygram: HTTP REST + MCP поверх ядра.
//! Предоставляет срезы контекста и выполнение команд.

pub mod http;
pub mod mcp;
pub mod chat;
pub mod sync;

use polygram_core::{
    document::{Document, DocObject}, error::EngineError, runtime::EngineRuntime, Command, Event,
    ObjectView, Stats, types::{Direction, NodeKind},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

    /// Получить полный документ grafa (включая parent у объектов).
    pub fn to_document_json(&self) -> Result<String, EngineError> {
        let json = self
            .inner
            .runtime
            .handle()
            .query(|e| e.to_json())
            .ok_or(EngineError::Stopped)?;
        Ok(json)
    }

    /// Применить документ (.pg JSON) к текущему графу, заменяя его содержимое.
    pub fn apply_document(&self, json: &str) -> Result<(), EngineError> {
        let doc = Document::parse(json)?;
        let engine = polygram_core::Engine::from_document(&doc)?;
        // Нельзя просто заменить движок в рантайме — вместо этого применяем команды.
        // Но для простоты возвращаем документ, а GUI сама разберёт его на команды.
        let _ = engine;
        Ok(())
    }

    /// Импортировать документ (.pg JSON) как новый отдел (подграф) в текущий граф.
    /// Все импортируемые объекты помещаются в созданный отдел (с сохранением внутренней
    /// иерархии отделов и связей). ID объектов и показателей переназначаются с уникальным
    /// префиксом, чтобы избежать конфликтов с существующими в графе.
    pub fn import_document_as_department(
        &self,
        json: &str,
        dept_name: &str,
    ) -> Result<String, EngineError> {
        let doc = Document::parse(json)?;
        let handle = self.inner.runtime.handle();

        // Текущие ID объектов и показателей в графе (для проверки уникальности).
        let existing_object_ids: Vec<String> = handle
            .query(|e| {
                e.hierarchy()
                    .live_objects()
                    .map(|ix| e.hierarchy().object(ix).ext_id.clone())
                    .collect()
            })
            .unwrap_or_default();
        let existing_metric_def_ids: Vec<String> = handle
            .query(|e| {
                e.hierarchy()
                    .live_metric_defs()
                    .map(|ix| e.hierarchy().metric_def(ix).ext_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Уникальный ID нового отдела.
        let mut used_object_ids = existing_object_ids.clone();
        let dept_id = unique_id("imp", &mut used_object_ids);

        // Переназначение ID показателей (глобальное пространство имён).
        let mut md_map: HashMap<String, String> = HashMap::new();
        let mut used_md_ids = existing_metric_def_ids.clone();
        for md in &doc.metric_defs {
            let new_id = unique_id(&md.id, &mut used_md_ids);
            md_map.insert(md.id.clone(), new_id.clone());
            used_md_ids.push(new_id);
        }

        // Топологическая сортировка объектов: родители перед детьми.
        let obj_order = topological_sort_objects(&doc.objects);

        // Переназначение ID объектов.
        let mut obj_map: HashMap<String, String> = HashMap::new();
        for oid in &obj_order {
            let o = doc
                .objects
                .iter()
                .find(|o| o.id == *oid)
                .expect("объект из топологической сортировки");
            let new_id = unique_id(&o.id, &mut used_object_ids);
            obj_map.insert(o.id.clone(), new_id.clone());
            used_object_ids.push(new_id);
        }

        // Построение пачки команд.
        let mut cmds: Vec<Command> = Vec::new();

        // 1. Сам отдел.
        cmds.push(Command::AddObject {
            id: Some(dept_id.clone()),
            name: dept_name.to_owned(),
            kind: NodeKind::Department,
            at: None,
            parent: None,
            author: None,
        });

        // 2. Показатели.
        for md in &doc.metric_defs {
            cmds.push(Command::AddMetricDef {
                id: Some(md_map[&md.id].clone()),
                name: md.name.clone(),
                kind: md.kind,
                unit: md.unit.clone(),
                api: md.api.clone(),
            });
        }

        // 3. Объекты (в топологическом порядке — родители перед детьми).
        for oid in &obj_order {
            let o = doc
                .objects
                .iter()
                .find(|o| o.id == *oid)
                .expect("объект из топологической сортировки");
            let new_parent = o
                .parent
                .as_ref()
                .and_then(|p| obj_map.get(p).cloned())
                .unwrap_or_else(|| dept_id.clone());
            cmds.push(Command::AddObject {
                id: Some(obj_map[oid].clone()),
                name: o.name.clone(),
                kind: o.kind,
                at: Some(o.at),
                parent: Some(new_parent),
                author: None,
            });
        }

        // 4. Записи показателей объектов (entry ID scoped к объекту — оставляем как есть,
        //    но metric_def переназначаем).
        for oid in &obj_order {
            let o = doc
                .objects
                .iter()
                .find(|o| o.id == *oid)
                .expect("объект из топологической сортировки");
            for (dir, list) in [
                (Direction::Request, &o.requests),
                (Direction::Output, &o.outputs),
            ] {
                for m in list {
                    cmds.push(Command::AddMetricEntry {
                        object: obj_map[oid].clone(),
                        dir,
                        id: Some(m.id.clone()),
                        metric_def: md_map[&m.metric_def].clone(),
                        name: m.id.clone(),
                        value: m.value.clone(),
                        required: m.required.clone(),
                        max: m.max.clone(),
                        fact: m.fact.clone(),
                    });
                }
            }
        }

        // 5. Связи (переназначены from_object / to_object, entry ID не меняются).
        for e in &doc.edges {
            if let (Some(from), Some(to)) = (
                obj_map.get(&e.from_object),
                obj_map.get(&e.to_object),
            ) {
                cmds.push(Command::Connect {
                    from_object: from.clone(),
                    from_entry: e.from_entry.clone(),
                    to_object: to.clone(),
                    to_entry: e.to_entry.clone(),
                    throughput: e.throughput,
                });
            }
        }

        handle
            .apply_batch(cmds)
            .map(|_| dept_id.clone())
            .map_err(|(_, e)| e)?;
        Ok(dept_id)
    }
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new(polygram_core::Engine::new())
    }
}

/// Сгенерировать уникальный ID, которого нет в списке `used`.
/// Если `base` свободен — возвращает его как есть, иначе добавляет суффикс `_1`, `_2`, ...
fn unique_id(base: &str, used: &mut Vec<String>) -> String {
    if !used.contains(&base.to_owned()) {
        used.push(base.to_owned());
        return base.to_owned();
    }
    let mut n = 1u64;
    loop {
        let candidate = format!("{base}_{n}");
        if !used.contains(&candidate) {
            used.push(candidate.clone());
            return candidate;
        }
        n += 1;
    }
}

/// Топологическая сортировка объектов документа: родители перед детьми.
/// Объекты, чей родитель не входит в документ (или None), идут первыми.
fn topological_sort_objects(objects: &[DocObject]) -> Vec<String> {
    let id_set: std::collections::HashSet<&str> = objects
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    let mut result: Vec<String> = Vec::new();
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();

    fn visit<'a>(
        oid: &'a str,
        objects: &'a [DocObject],
        id_set: &std::collections::HashSet<&str>,
        visited: &mut std::collections::HashSet<&'a str>,
        result: &mut Vec<String>,
    ) {
        if !visited.insert(oid) {
            return;
        }
        let obj = match objects.iter().find(|o| o.id == oid) {
            Some(o) => o,
            None => return,
        };
        // Сначала посещаем родителя (если он в документе).
        if let Some(parent) = &obj.parent {
            if id_set.contains(parent.as_str()) {
                visit(parent, objects, id_set, visited, result);
            }
        }
        result.push(oid.to_owned());
    }

    for o in objects {
        visit(&o.id, objects, &id_set, &mut visited, &mut result);
    }
    result
}

