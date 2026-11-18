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
    views::MetricDefView,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{mpsc, Arc, RwLock},
    thread,
    time::Duration,
};

/// Состояние виртуального ввода (мышь + клавиатура), управляемое извне через MCP/API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualInputState {
    /// Целевая координата X виртуального курсора (устанавливается командами).
    pub target_x: f32,
    /// Целевая координата Y виртуального курсора (устанавливается командами).
    pub target_y: f32,
    /// Контрольная точка X для кривой Безье (если 0,0 — движение линейное).
    pub curve_control_x: f32,
    /// Контрольная точка Y для кривой Безье (если 0,0 — движение линейное).
    pub curve_control_y: f32,
    /// Текущая отрисовываемая координата X (интерполируется в GUI).
    pub mouse_x: f32,
    /// Текущая отрисовываемая координата Y (интерполируется в GUI).
    pub mouse_y: f32,
    /// Левая кнопка мыши зажата.
    pub mouse_left_pressed: bool,
    /// Правая кнопка мыши зажата.
    pub mouse_right_pressed: bool,
    /// Средняя кнопка мыши зажата.
    pub mouse_middle_pressed: bool,
    /// Текст для ввода виртуальной клавиатуры (очищается после подачи).
    pub keyboard_text: String,
    /// Нажатые клавиши (имена клавиш, например "Control", "C", "V") — для комбинаций клавиш.
    pub keyboard_keys: Vec<String>,
    /// Дельта прокрутки колёсика мыши (x, y).
    pub wheel_delta: (f32, f32),
    /// Фигура для рисования курсором: Some(kind, x2, y2) — рисовать от точки нажатия до (x2, y2).
    pub draw_shape: Option<(u8, f32, f32)>,
    /// Цель перетаскивания: Some(object_id) — объект, который перетаскивается.
    pub drag_target: Option<String>,
    /// Целевые координаты перетаскивания: Some((x, y)) — куда перетаскивать объект.
    pub drag_to: Option<(f32, f32)>,
    /// Точки скруглённой кривой для анимации (устанавливается командой VirtualMousePath).
    pub path_points: Vec<(f32, f32)>,
    /// Индекс текущей точки на кривой (для анимации в GUI).
    pub path_index: usize,
}

impl Default for VirtualInputState {
    fn default() -> Self {
        Self {
            target_x: 0.0,
            target_y: 0.0,
            curve_control_x: 0.0,
            curve_control_y: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left_pressed: false,
            mouse_right_pressed: false,
            mouse_middle_pressed: false,
            keyboard_text: String::new(),
            keyboard_keys: Vec::new(),
            wheel_delta: (0.0, 0.0),
            draw_shape: None,
            drag_target: None,
            drag_to: None,
            path_points: Vec::new(),
            path_index: 0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Published {
    pub rev: u64,
    pub dynamic: DynamicState,
    pub geometry: Geometry,
    pub metric_defs: Vec<MetricDefView>,
    pub camera_fit_requested: bool,
    pub auto_screenshot: bool,
    pub virtual_input: VirtualInputState,
}

type BatchResult = std::result::Result<Vec<Applied>, (usize, EngineError)>;

enum Msg {
    Apply {
        scope: Scope,
        cmd: Command,
        reply: mpsc::Sender<Result<Applied>>,
    },
    Batch {
        scope: Scope,
        cmds: Vec<Command>,
        reply: mpsc::Sender<BatchResult>,
    },
    Query(Box<dyn FnOnce(&mut Engine) + Send>),
    ReplaceEngine(Engine),
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

// `JoinHandle` is `Send` but not `Sync`, but `join` is private and never
// exposed through shared references, so `EngineRuntime` can safely be `Sync`.
unsafe impl Sync for EngineRuntime {}

fn publish(
    engine: &Engine,
    slot: &RwLock<Arc<Published>>,
    prev: &Arc<Published>,
) -> Arc<Published> {
    let geometry =
        if prev.geometry.rev != engine.geometry_rev() || prev.rev == 0 && engine.rev() == 0 {
            engine.geometry()
        } else {
            prev.geometry.clone()
        };
    let metric_defs = engine.metric_def_views();
    let next = Arc::new(Published {
        rev: engine.rev(),
        dynamic: engine.dynamic_state(),
        geometry,
        metric_defs,
        camera_fit_requested: engine.camera_fit_requested,
        auto_screenshot: engine.auto_screenshot,
        virtual_input: engine.virtual_input.clone(),
    });
    *slot.write().expect("замок публикации") = next.clone();
    next
}

impl EngineRuntime {
    pub fn spawn(engine: Engine) -> EngineRuntime {
        let (tx, rx) = mpsc::channel::<Msg>();
        let first = Arc::new(Published {
            rev: engine.rev(),
            dynamic: engine.dynamic_state(),
            geometry: engine.geometry(),
            metric_defs: engine.metric_def_views(),
            camera_fit_requested: engine.camera_fit_requested,
            auto_screenshot: engine.auto_screenshot,
            virtual_input: VirtualInputState::default(),
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
                                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.apply(scope, cmd)))
                                    .unwrap_or_else(|_| Err(EngineError::Stopped));
                                replies.push(Box::new(move || {
                                    let _ = reply.send(r);
                                }));
                            }
                            Msg::Batch { scope, cmds, reply } => {
                                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.apply_batch(scope, cmds)))
                                    .unwrap_or_else(|_| Err((0, EngineError::Stopped)));
                                replies.push(Box::new(move || {
                                    let _ = reply.send(r);
                                }));
                            }
                            Msg::Query(f) => {
                                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut engine)));
                            }
                            Msg::ReplaceEngine(new_engine) => {
                                engine = new_engine;
                                last = publish(&engine, &slot, &last);
                            }
                            Msg::Stop => {
                                stop = true;
                                break;
                            }
                        }
                    }
                    if engine.rev() != last.rev {
                        last = publish(&engine, &slot, &last);
                        engine.camera_fit_requested = false;
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
        EngineRuntime {
            handle: EngineHandle { tx, published },
            join: Some(join),
        }
    }

    pub fn handle(&self) -> EngineHandle {
        self.handle.clone()
    }

    /// Остановить поток и получить движок назад.
    pub fn shutdown(mut self) -> Option<Engine> {
        let _ = self.handle.tx.send(Msg::Stop);
        self.join.take().and_then(|j| j.join().ok())
    }

    /// Заменить движок в рантайме на новый (используется при открытии проекта из файла).
    pub fn replace_engine(&self, engine: Engine) {
        let _ = self.handle.tx.send(Msg::ReplaceEngine(engine));
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
        self.tx
            .send(Msg::Apply { scope, cmd, reply })
            .map_err(|_| EngineError::Stopped)?;
        rx.recv().map_err(|_| EngineError::Stopped)?
    }

    pub fn apply_batch(&self, cmds: Vec<Command>) -> BatchResult {
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(Msg::Batch {
                scope: Scope::Root,
                cmds,
                reply,
            })
            .map_err(|_| (0, EngineError::Stopped))?;
        rx.recv().map_err(|_| (0, EngineError::Stopped))?
    }

    /// Выполнить чтение на потоке движка и вернуть результат.
    pub fn query<R: Send + 'static>(
        &self,
        f: impl FnOnce(&Engine) -> R + Send + 'static,
    ) -> Option<R> {
        let (reply, rx) = mpsc::channel();
        let job = Box::new(move |e: &mut Engine| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(e)));
            let _ = reply.send(result.ok());
        });
        self.tx.send(Msg::Query(job)).ok()?;
        rx.recv_timeout(Duration::from_secs(5)).ok()?
    }

    /// Последний опубликованный снимок (дёшево: клонируется только указатель).
    pub fn latest(&self) -> Arc<Published> {
        self.published.read().expect("замок публикации").clone()
    }
}
