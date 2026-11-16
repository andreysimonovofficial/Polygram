//! Ранжирование и агрегация статусов показателей.

use polygram_core::geometry::EntryGeom;
use polygram_core::{DynamicState, StatusCode};

/// Ранг статуса (чем выше — тем выше приоритет показа / хуже состояние).
/// Ожидается > В процессе > В избытке > Обеспечен / Удовлетворён.
pub fn status_rank(s: StatusCode) -> u8 {
    match s {
        StatusCode::Ok => 0,
        StatusCode::Surplus => 1,
        StatusCode::InProgress => 2,
        StatusCode::Expected => 3,
    }
}

/// Худший (высокоприоритетнейший) статус среди записей.
pub fn worst_status(entries: &[EntryGeom], dynamic: &DynamicState) -> StatusCode {
    let mut worst: u8 = 0;
    for e in entries {
        let r = status_rank(dynamic.status[e.ix as usize]);
        if r > worst {
            worst = r;
        }
    }
    match worst {
        3 => StatusCode::Expected,
        2 => StatusCode::InProgress,
        1 => StatusCode::Surplus,
        _ => StatusCode::Ok,
    }
}
