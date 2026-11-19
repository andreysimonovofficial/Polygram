//! События движка.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    ObjectAdded { object: u32 },
    ObjectRemoved { object: u32 },
    ObjectRenamed { object: u32 },
    ObjectMoved { object: u32, gx: i32, gy: i32 },
    MetricDefAdded { metric_def: u32 },
    MetricDefRemoved { metric_def: u32 },
    MetricDefRenamed { metric_def: u32 },
    MetricAdded { metric: u32 },
    MetricRemoved { metric: u32 },
    MetricChanged { metric: u32 },
    EdgeAdded,
    EdgeRemoved,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub rev: u64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventGap {
    Missing,
}

#[derive(Clone, Debug)]
pub struct EventLog {
    events: Vec<Event>,
    capacity: usize,
}

impl EventLog {
    pub fn new(capacity: usize) -> Self {
        Self { events: Vec::new(), capacity }
    }

    pub fn push(&mut self, rev: u64, kind: EventKind) {
        self.events.push(Event { rev, kind });
        if self.events.len() > self.capacity {
            self.events.remove(0);
        }
    }

    pub fn since(&self, rev: u64) -> std::result::Result<Vec<Event>, EventGap> {
        let events: Vec<Event> = self.events.iter().filter(|e| e.rev > rev).cloned().collect();
        if events.is_empty() && rev != 0 {
            Err(EventGap::Missing)
        } else {
            Ok(events)
        }
    }
}
