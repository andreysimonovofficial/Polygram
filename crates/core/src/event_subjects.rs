use crate::hierarchy::Hierarchy;
use crate::types::ObjectIx;

/// Объекты, к которым относится событие (для фильтрации ленты отдела).
pub fn event_subjects(kind: &crate::event::EventKind, hier: &Hierarchy) -> Vec<ObjectIx> {
    match kind {
        crate::event::EventKind::ObjectAdded { object }
        | crate::event::EventKind::ObjectRemoved { object }
        | crate::event::EventKind::ObjectRenamed { object }
        | crate::event::EventKind::ObjectMoved { object, .. } => vec![ObjectIx(*object)],
        crate::event::EventKind::MetricAdded { metric }
        | crate::event::EventKind::MetricRemoved { metric }
        | crate::event::EventKind::MetricChanged { metric }
        | crate::event::EventKind::StatusChanged { metric, .. } => {
            let e = crate::types::EntryIx(*metric);
            let om = hier.object_metric(e);
            vec![om.object]
        }
        crate::event::EventKind::EdgeAdded { edge }
        | crate::event::EventKind::EdgeRemoved { edge }
        | crate::event::EventKind::EdgeRouted { edge } => {
            let ed = hier.edge(crate::types::EdgeIx(*edge));
            let from = hier.object_metric(ed.from).object;
            let to = hier.object_metric(ed.to).object;
            if from == to {
                vec![from]
            } else {
                vec![from, to]
            }
        }
        crate::event::EventKind::RoutesChanged
        | crate::event::EventKind::LayoutChanged
        | crate::event::EventKind::MetricDefAdded { .. }
        | crate::event::EventKind::MetricDefRemoved { .. }
        | crate::event::EventKind::MetricDefRenamed { .. } => Vec::new(),
    }
}
