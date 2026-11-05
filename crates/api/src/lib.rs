//! API-слой Polygram: HTTP REST + MCP поверх ядра.
//! Предоставляет срезы контекста и выполнение команд.

pub mod http;
pub mod mcp;

use polygram_core::{
    runtime::EngineRuntime, Command, Event, ObjectView, Stats,
};
use std::sync::Arc;

/// Главный сервис API. Оборачивает EngineRuntime и предоставляет
/// срезы состояния и выполнение команд.
#[derive(Clone)]
pub struct ApiService {
    inner: Arc<ApiServiceInner>,
}

struct ApiServiceInner {
    runtime: EngineRuntime,
}

impl ApiService {
    pub fn new(engine: polygram_core::Engine) -> Self {
        let runtime = EngineRuntime::spawn(engine);
        Self { inner: Arc::new(ApiServiceInner { runtime }) }
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
        self.inner.runtime.handle().query(move |e| e.object_view(&id)).flatten()
    }

    /// Лента событий отдела: собственные события + события всех дочерних отделов.
    pub fn department_events(&self, dept: String) -> Option<Result<Vec<Event>, polygram_core::error::EngineError>> {
        self.inner.runtime.handle().query(move |e| e.department_events(&dept))
    }

    pub fn apply(&self, cmd: Command) -> Result<polygram_core::command::Applied, polygram_core::error::EngineError> {
        self.inner.runtime.handle().apply(cmd)
    }

    pub fn apply_batch(&self, cmds: Vec<Command>) -> Result<Vec<polygram_core::command::Applied>, (usize, polygram_core::error::EngineError)> {
        self.inner.runtime.handle().apply_batch(cmds)
    }
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new(polygram_core::Engine::new())
    }
}
