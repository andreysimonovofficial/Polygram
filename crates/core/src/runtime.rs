//! Рантайм: движок в собственном потоке. Окно, MCP и API — клиенты через `EngineHandle`.
//!
//! * Команды приходят по каналу и выполняются одним потоком (нет гонок, один писатель).
//! * После каждой пачки команд публикуется неизменяемый снимок (`Published`): окно берёт
//!   последний снимок за один клик мьютекса, не мешая расчётам.
//! * Быстро меняющиеся числа публикуются при каждой ревизии, геометрия (позиции, линии) —
//!   только когда она изменилась.

use crate::{
    command::{Applied, Command},
    engine::Engine,
    error::{EngineError, Result},
    geometry::Geometry,
    types::Scope,
    values::DynamicState,
};
use std::{
    sync::{mpsc, Arc, RwLock},
    thread,
};

#[derive(Debug)]
pub struct Published {
    pub rev: u64,
    pub dynamic: Arc<DynamicState>,
    pub geometry: Arc<Geometry>,
}

type BatchResult = std::result::Result<Vec<Applied>, (usize, EngineError)>;

enum Msg {
    Apply { scope: Scope, cmd: Command, reply: mpsc::Sender<Result<Applied>> },
    Batch { scope: Scope, cmds: Vec<Command>, reply: mpsc::Sender<BatchResult> },
    Query(Box<dyn FnOnce(&mut Engine) + Send>),
    Stop,
}

#[derive(Clone)]
pub struct EngineHandle {
    tx: mpsc::Sender<Msg>,
    published: Arc<RwLock<Arc<Published>>>,
}

pub struct EngineRuntime {
    handle: EngineHandle,
    join: Option<thread::JoinHandle<Engine>>,
}

fn publish(engine: &Engine, slot: &RwLock<Arc<Published>>, prev: &Arc<Published>) -> Arc<Published> {
    let geometry = if prev.geometry.rev != engine.geometry_rev() || prev.rev == 0 && engine.rev() == 0 {
        Arc::new(engine.geometry())
    } else {
        prev.geometry.clone()
    };
    let next = Arc::new(Published {
        rev: engine.rev(),
        dynamic: Arc::new(engine.dynamic_state()),
        geometry,
    });
    *slot.write().expect("замок публикации") = next.clone();
    next
}

impl EngineRuntime {
    pub fn spawn(engine: Engine) -> EngineRuntime {
        let (tx, rx) = mpsc::channel::<Msg>();
        let first = Arc::new(Published {
            rev: engine.rev(),
            dynamic: Arc::new(engine.dynamic_state()),
            geometry: Arc::new(engine.geometry()),
        });
        let published = Arc::new(RwLock::new(first.clone()));
        let slot = published.clone();
        let join = thread::Builder::new()
            .name("polygram-engine".to_owned())
            .spawn(move || {
                let mut engine = engine;
                let mut last = first;
                loop {
                    let Ok(first_msg) = rx.recv() else { break };
                    let mut batch = vec![first_msg];
                    while batch.len() < 512 {
                        match rx.try_recv() {
                            Ok(m) => batch.push(m),
                            Err(_) => break,
                        }
                    }
                    let mut replies: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
                    let mut stop = false;
                    for msg in batch {
                        match msg {
                            Msg::Apply { scope, cmd, reply } => {
                                let r = engine.apply(scope, cmd);
                                replies.push(Box::new(move || {
                                    let _ = reply.send(r);
                                }));
                            }
                            Msg::Batch { scope, cmds, reply } => {
                                let r = engine.apply_batch(scope, cmds);
                                replies.push(Box::new(move || {
                                    let _ = reply.send(r);
                                }));
                            }
                            Msg::Query(f) => f(&mut engine),
                            Msg::Stop => {
                                stop = true;
                                break;
                            }
                        }
                    }
                    if engine.rev() != last.rev {
                        last = publish(&engine, &slot, &last);
                    }
                    for r in replies {
                        r();
                    }
                    if stop {
                        break;
                    }
                }
                engine
            })
            .expect("не удалось запустить поток движка");
        EngineRuntime { handle: EngineHandle { tx, published }, join: Some(join) }
    }

    pub fn handle(&self) -> EngineHandle {
        self.handle.clone()
    }

    /// Остановить поток и получить движок назад.
    pub fn shutdown(mut self) -> Option<Engine> {
        let _ = self.handle.tx.send(Msg::Stop);
        self.join.take().and_then(|j| j.join().ok())
    }
}

impl Drop for EngineRuntime {
    fn drop(&mut self) {
        if let Some(j) = self.join.take() {
            let _ = self.handle.tx.send(Msg::Stop);
            let _ = j.join();
        }
    }
}

impl EngineHandle {
    pub fn apply(&self, cmd: Command) -> Result<Applied> {
        self.apply_scoped(Scope::Root, cmd)
    }

    pub fn apply_scoped(&self, scope: Scope, cmd: Command) -> Result<Applied> {
        let (reply, rx) = mpsc::channel();
        self.tx.send(Msg::Apply { scope, cmd, reply }).map_err(|_| EngineError::Stopped)?;
        rx.recv().map_err(|_| EngineError::Stopped)?
    }

    pub fn apply_batch(&self, cmds: Vec<Command>) -> BatchResult {
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(Msg::Batch { scope: Scope::Root, cmds, reply })
            .map_err(|_| (0, EngineError::Stopped))?;
        rx.recv().map_err(|_| (0, EngineError::Stopped))?
    }

    /// Выполнить чтение на потоке движка и вернуть результат.
    pub fn query<R: Send + 'static>(&self, f: impl FnOnce(&Engine) -> R + Send + 'static) -> Option<R> {
        let (reply, rx) = mpsc::channel();
        let job = Box::new(move |e: &mut Engine| {
            let _ = reply.send(f(e));
        });
        self.tx.send(Msg::Query(job)).ok()?;
        rx.recv().ok()
    }

    /// Последний опубликованный снимок (дёшево: клонируется только указатель).
    pub fn latest(&self) -> Arc<Published> {
        self.published.read().expect("замок публикации").clone()
    }
}
