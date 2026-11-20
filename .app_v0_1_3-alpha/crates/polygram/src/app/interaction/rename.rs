//! Инлайн-редактирование названия объекта прямо на холсте (Figma-стиль).
//! Поле ввода рисуется поверх текста названия процесса.

use crate::app::PolygramApp;
use polygram_core::{Command, Published};
use egui::{Context, Key, Pos2, Rect, Vec2};

/// Инлайн-редактор названия объекта (процесса/отдела) прямо на холсте.
pub fn inline_rename_editor(app: &mut PolygramApp, ctx: &Context, _published: &Published) {
    let Some(target) = app.dialogs.edit_target.clone() else {
        return;
    };
    let object = match app.canvas.objects.iter().find(|o| o.id == target) {
        Some(o) => o.clone(),
        None => {
            app.dialogs.edit_target = None;
            return;
        }
    };

    let zoom = app.canvas.camera.zoom;
    let screen_min = app.canvas.cell_to_screen(object.rect.x, object.rect.y);
    // Название процесса рисуется в заголовке карточки: слева после алмаза,
    // по вертикали — центр шапки (header_h = 28px).
    let header_h = 28.0 * zoom;
    let header_cy = screen_min.y + 4.0 * zoom + header_h / 2.0;
    let card_w = 256.0 * zoom;
    let edit_rect = Rect::from_min_size(
        Pos2::new(screen_min.x + 24.0 * zoom, header_cy - (10.0 * zoom) / 2.0),
        Vec2::new((card_w - 48.0 * zoom).max(80.0), 14.0 * zoom),
    );

    let mut commit = false;
    let mut cancel = false;

    egui::Area::new(egui::Id::new("inline_rename_process"))
        .order(egui::Order::Foreground)
        .fixed_pos(edit_rect.min)
        .show(ctx, |ui| {
            ui.set_min_width(edit_rect.width().max(40.0));
            ui.set_min_height(edit_rect.height().max(12.0));
            egui::Frame::NONE.show(ui, |ui| {
                let response = ui.text_edit_singleline(&mut app.dialogs.edit_input);
                response.request_focus();

                if response.has_focus() {
                    if ctx.input(|i| i.key_pressed(Key::Enter)) {
                        commit = true;
                    }
                    if ctx.input(|i| i.key_pressed(Key::Escape)) {
                        cancel = true;
                    }
                }
                if response.lost_focus() {
                    commit = true;
                }
            });
        });

    if commit {
        if !app.dialogs.edit_input.is_empty() {
            app.apply_cmd(Command::RenameObject {
                object: target.clone(),
                name: app.dialogs.edit_input.clone(),
            });
        }
        app.dialogs.edit_target = None;
        app.dialogs.edit_input.clear();
    } else if cancel {
        app.dialogs.edit_target = None;
        app.dialogs.edit_input.clear();
    }
}
