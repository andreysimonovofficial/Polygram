//! Журнал событий. Каждое успешное изменение получает номер ревизии.

use crate::types::StatusCode;
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    ObjectAdded {
        object: u32,
    },
    ObjectRemoved {
        object: u32,
    },
    ObjectRenamed {
        object: u32,
    },
    ObjectMoved {
        object: u32,
        gx: i32,
        gy: i32,
    },
    MetricDefAdded {
        metric_def: u32,
    },
    MetricDefRemoved {
        metric_def: u32,
    },
    MetricDefRenamed {
        metric_def: u32,
    },
    MetricAdded {
        metric: u32,
    },
    MetricRemoved {
        metric: u32,
    },
    MetricChanged {
        metric: u32,
    },
    EdgeAdded {
        edge: u32,
    },
    EdgeRemoved {
        edge: u32,
    },
    EdgeRouted {
        edge: u32,
    },
    RoutesChanged,
    LayoutChanged,
    StatusChanged {
        metric: u32,
        from: StatusCode,
        to: StatusCode,
    },
}

impl EventKind {
    pub fn is_structural(&self) -> bool {
        !matches!(
            self,
            Self::MetricChanged { .. } | Self::StatusChanged { .. }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Event {
    pub rev: u64,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventGap {
    pub oldest_available: u64,
}

#[derive(Clone, Debug)]
pub struct EventLog {
    buf: VecDeque<Event>,
    cap: usize,
    dropped_up_to: u64,
}

impl EventLog {
    pub fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::new(),
            cap: cap.max(1),
            dropped_up_to: 0,
        }
    }

    pub fn push(&mut self, e: Event) {
        self.buf.push_back(e);
        while self.buf.len() > self.cap {
            if let Some(old) = self.buf.pop_front() {
                self.dropped_up_to = self.dropped_up_to.max(old.rev);
            }
        }
    }

    pub fn since(&self, rev: u64) -> Result<Vec<Event>, EventGap> {
        if rev < self.dropped_up_to {
            return Err(EventGap {
                oldest_available: self.dropped_up_to + 1,
            });
        }
        Ok(self.buf.iter().filter(|e| e.rev > rev).cloned().collect())
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.buf.iter()
    }
}
