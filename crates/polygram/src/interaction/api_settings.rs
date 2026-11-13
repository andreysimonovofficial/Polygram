//! Настройки API-привязки для одной записи показателя
//! (разворачиваются по нажатию ⚙ в строке показателя контейнера).

use crate::app::PolygramApp;
use egui::Ui;
use polygram_core::types::Direction;
use polygram_core::geometry::EntryGeom;

/// Настройки API-привязки для одной записи показателя.
pub fn render_entry_api_settings(
    ui: &mut Ui,
    app: &mut PolygramApp,
    object_id: &str,
    entry: &EntryGeom,
    dir: Direction,
) {
    let binding_id = format!("{}:{}", object_id, entry.name);

    // Неизменяемые чтения из API (клонируем, заимствование освобождается).
    let connectors: Vec<String> = app.connectors.get_connectors();
    let bindings: Vec<serde_json::Value> = app.get_bindings();
    let current = bindings.iter().find(|b| {
        b.get("id").and_then(|v| v.as_str()) == Some(&binding_id)
    });
    let current_label = if let Some(b) = current {
        format!(
            "привязка: {} → {} (интервал {} сек)",
            b.get("connector").and_then(|v| v.as_str()).unwrap_or(""),
            b.get("cell").and_then(|v| v.as_str()).unwrap_or(""),
            b.get("interval_secs").and_then(|v| v.as_u64()).unwrap_or(0)
        )
    } else {
        "привязка: не привязано".to_owned()
    };

    let direction_text = match dir {
        Direction::Request => "запись в: необходимое (required)",
        Direction::Output => "запись в: значение (value)",
    };

    // Изменяемые настройки одной записи в области, чтобы заимствование
    // приложения было освобождено перед записями (post/delete_binding).
    let (do_bind, do_unbind, do_close, bind_connector, bind_cell, bind_interval) = {
        let settings = app
            .dialogs
            .container_menu_api_settings
            .entry(entry.name.clone())
            .or_default();
        let mut do_bind = false;
        let mut do_unbind = false;
        let mut do_close = false;

        ui.label(direction_text);

        ui.label("коннектор:");
        egui::ComboBox::new(format!("api_connector_{}", binding_id), "")
            .selected_text(if settings.connector.is_empty() {
                "коннектор".to_owned()
            } else {
                settings.connector.clone()
            })
            .show_ui(ui, |ui| {
                if connectors.is_empty() {
                    ui.disable();
                    ui.label("нет коннекторов (API недоступен)");
                } else {
                    for c in &connectors {
                        if ui.selectable_label(settings.connector == c.as_str(), c).clicked() {
                            settings.connector = c.clone();
                        }
                    }
                }
            });

        ui.label("ячейка / метрика:");
        ui.text_edit_singleline(&mut settings.cell);

        ui.label("интервал опроса (сек):");
        let mut interval_str = settings.interval_secs.to_string();
        if ui.text_edit_singleline(&mut interval_str).lost_focus() {
            if let Ok(interval) = interval_str.trim().parse::<u64>() {
                settings.interval_secs = interval.max(1);
            }
        }

        ui.label(current_label);

        ui.separator();
        ui.horizontal(|ui| {
            if !settings.connector.is_empty() && !settings.cell.is_empty() {
                if ui.button("Привязать").clicked() {
                    do_bind = true;
                }
            }
            if ui.button("Отвязать").clicked() {
                do_unbind = true;
            }
            if ui.button("Закрыть").clicked() {
                do_close = true;
            }
        });

        (
            do_bind,
            do_unbind,
            do_close,
            settings.connector.clone(),
            settings.cell.clone(),
            settings.interval_secs,
        )
    };

    // Записи в API после освобождения заимствования настроек.
    if do_bind {
        app.post_binding(
            object_id.to_owned(),
            entry.name.clone(),
            bind_connector.clone(),
            bind_cell.clone(),
            bind_interval,
        );
        app.add_event_log(
            "info",
            "command",
            &format!("Объект «{object_id}»: запись «{}» привязана к {}:{}", entry.name, bind_connector, bind_cell),
            None,
        );
    }
    if do_unbind {
        app.delete_binding(object_id.to_owned(), entry.name.clone());
        app.add_event_log(
            "warn",
            "command",
            &format!("Объект «{object_id}»: запись «{}» отвязана от API", entry.name),
            None,
        );
    }
    if do_close {
        app.dialogs.container_menu_api_expanded = None;
    }
}
