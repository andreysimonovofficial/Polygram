//! Финализация ревизии: пересчёт значений и статусов.

use crate::engine::Engine;

pub fn finish(engine: &mut Engine) {
    let mut changes = Vec::new();
    engine.vals.recompute_all(&engine.hier, &mut changes);
    engine.vals.settle(&engine.hier, &mut changes);
    engine.rev += 1;
    engine.geo_rev += 1;
}
