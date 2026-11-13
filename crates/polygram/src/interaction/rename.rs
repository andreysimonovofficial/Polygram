use crate::app::PolygramApp;
use polygram_core::Published;
use egui::{Context, Key, Pos2, Rect, Vec2, Window};

/// Диалог переименования объекта (модальное окно).
pub fn handle_rename_dialog(app: &mut PolygramApp, ctx: &Context) {
    let rename_target = app.dialogs.rename_target.clone();
    let mut do_rename = false;
    if let Some(target) = &rename_target {
        Window::new("Переименовать объект")
            .default_pos(egui::pos2(400.0, 300.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!("Объект: {target}"));
                ui.text_edit_singleline(&mut app.dialogs.rename_input);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Отмена").clicked() {
                        app.dialogs.rename_target = None;
                        app.dialogs.rename_input.clear();
                    }
                });
            });
    }
    if do_rename && !app.dialogs.rename_input.is_empty() {
        app.apply_cmd(polygram_core::Command::RenameObject {
            object: rename_target.unwrap(),
            name: app.dialogs.rename_input.clone(),
        });
        app.add_event_log(
            "info",
            "command",
            &format!("Объект переименован в «{}»", app.dialogs.rename_input),
            None,
        );
        app.dialogs.rename_target = None;
        app.dialogs.rename_input.clear();
    }
}

/// Инлайн-редактирование названия объекта прямо на холсте (Figma-стиль).
/// Поле ввода рисуется точно поверх текста названия (shape "ui-1").
pub fn inline_rename_editor(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
) {
    let Some(target) = &app.dialogs.edit_target else {
        return;
    };
    let geometry = app.geometry_with_predictions(published);
    let Some(object) = geometry.objects.iter().find(|o| o.id == target.as_str()) else {
        app.dialogs.edit_target = None;
        return;
    };

    let cell_px = app.canvas.cell_px();
    let zoom = app.canvas.camera.zoom;
    let theme = &app.canvas.theme;
    let screen_min = app.canvas.camera.world_to_screen(
        Pos2::new(object.rect.x as f32, object.rect.y as f32),
        app.canvas.cell_size,
    );
    let header_h_px = cell_px * 1.0; // OBJECT_HEADER_HEIGHT_CELLS = 1.0
    let edit_rect = Rect::from_min_size(
        Pos2::new(
            screen_min.x + 16.0 * zoom,
            screen_min.y + header_h_px / 2.0 - (theme.text_size * zoom) / 2.0,
        ),
        Vec2::new(
            (object.rect.w as f32 * cell_px - 32.0 * zoom).max(60.0),
            theme.text_size * zoom,
        ),
    );

    let mut commit = false;
    let mut cancel = false;

    egui::Area::new(egui::Id::new("inline_rename"))
        .order(egui::Order::Foreground)
        .fixed_pos(edit_rect.min)
        .show(ctx, |ui| {
            ui.set_min_width(edit_rect.width().max(40.0));
            ui.set_min_height(edit_rect.height().max(16.0));
            egui::Frame::none().show(ui, |ui| {
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
            app.apply_cmd(polygram_core::Command::RenameObject {
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
