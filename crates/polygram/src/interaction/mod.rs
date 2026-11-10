//! Взаимодействие с пользователем на холсте: контекстное меню, удаление,
//! перетаскивание, диалоги показателей и переименования.
//! Разбито на узкие модули по одной ответственности каждому.

mod add_metric;
mod canvas;
mod context_menu;
mod container_menu;
mod delete;
mod rename;

pub use canvas::handle_canvas_interaction;
pub use context_menu::context_menu;
pub use delete::handle_delete_confirm;
pub use rename::{handle_rename_dialog, inline_rename_editor};
