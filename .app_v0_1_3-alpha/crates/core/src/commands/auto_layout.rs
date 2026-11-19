//! Автораскладка объектов на холсте.

use crate::engine::Engine;
use crate::{command::Applied, error::Result};

pub fn handle_auto_layout(_engine: &mut Engine) -> Result<Applied> {
    Ok(Applied::default())
}
