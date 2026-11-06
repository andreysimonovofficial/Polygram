use crate::engine::Engine;
use crate::event::EventKind;

/// Завершить пачку: пересчитать помеченное, переложить линии, записать события.
pub fn finish(engine: &mut Engine) {
    const ROUTE_EVENT_BURST: usize = 64;

    let mut changes = std::mem::take(&mut engine.scratch_changes);
    changes.clear();
    engine.vals.settle(&engine.hier, &mut changes);
    let routed = engine.layout.flush_routes(&engine.hier);
    for &(e, from, to) in &changes {
        engine.pending.push(EventKind::StatusChanged {
            metric: e.0,
            from,
            to,
        });
    }
    engine.scratch_changes = changes;
    if routed.len() > ROUTE_EVENT_BURST {
        engine.pending.push(EventKind::RoutesChanged);
    } else {
        for edge in routed {
            engine.pending.push(EventKind::EdgeRouted { edge: edge.0 });
        }
    }
    if !engine.pending.is_empty() {
        engine.rev += 1;
        if engine.pending.iter().any(|k| k.is_structural()) {
            engine.geo_rev = engine.rev;
        }
        for kind in engine.pending.drain(..) {
            engine.log.push(crate::event::Event {
                rev: engine.rev,
                kind,
            });
        }
    }
}
