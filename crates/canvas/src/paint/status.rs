//! Ранжирование и агрегация статусов показателей.

use polygram_core::geometry::EntryGeom;
use polygram_core::{DynamicState, StatusCode};

/// Ранг статуса (чем выше — тем хуже).
pub fn status_rank(s: StatusCode) -> u8 {
    match s {
        StatusCode::Ok => 0,
        StatusCode::Deficit => 1,
        StatusCode::Surplus => 2,
    }
}

/// Худший статус среди записей.
pub fn worst_status(entries: &[EntryGeom], dynamic: &DynamicState) -> StatusCode {
    let mut worst: u8 = 0;
    for e in entries {
        let r = status_rank(dynamic.status[e.ix as usize]);
        if r > worst {
            worst = r;
        }
    }
    match worst {
        2 => StatusCode::Surplus,
        1 => StatusCode::Deficit,
        _ => StatusCode::Ok,
    }
}
