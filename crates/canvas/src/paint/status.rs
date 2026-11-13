//! Ранжирование и агрегация статусов показателей.

use polygram_core::geometry::EntryGeom;
use polygram_core::{DynamicState, StatusCode};

/// Ранг статуса (чем выше — тем хуже).
pub fn status_rank(s: StatusCode) -> u8 {
    match s {
        StatusCode::Ok => 0,
        StatusCode::Unknown | StatusCode::Disabled => 1,
        StatusCode::Unconnected => 2,
        StatusCode::Warning => 3,
        StatusCode::Deficit => 4,
        StatusCode::Surplus => 5,
        StatusCode::Invalid => 6,
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
        4 => StatusCode::Deficit,
        5 => StatusCode::Surplus,
        3 => StatusCode::Warning,
        2 => StatusCode::Unconnected,
        6 => StatusCode::Invalid,
        _ => StatusCode::Ok,
    }
}
