//! Взаимодействие с пользователем на холсте: контекстное меню, удаление,
//! перетаскивание, диалоги показателей и переименования.
//! Разбито на узкие модули по одной ответственности каждому.

mod add_metric;
mod api_settings;
pub mod camera;
mod canvas;

#[cfg(test)]
mod canvas_tests;
pub mod container_menu;
mod context_menu;
mod create_subdepartment;
mod delete;
mod drag_object;
mod rename;
mod reparent_confirm;
mod selection;

pub use canvas::handle_canvas_interaction;
pub use context_menu::context_menu;
pub use create_subdepartment::handle_create_subdepartment;
pub use delete::handle_delete_confirm;
pub use rename::{handle_rename_dialog, inline_rename_editor};
pub use reparent_confirm::handle_reparent_confirm;
