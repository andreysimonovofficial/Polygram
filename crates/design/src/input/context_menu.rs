//! Контекстное меню правой кнопкой мыши.

use crate::free_canvas::FreeCanvas;
use egui::{Pos2, Vec2};

impl FreeCanvas {
    /// Обработка триггера контекстного меню (правый клик).
    pub(crate) fn handle_context_menu_trigger(&mut self, response: &egui::Response) {
        if response.secondary_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let world_pos = self.screen_to_world(pos);
                if let Some(idx) = self.hit_shape(world_pos) {
                    if !self.selected.contains(&idx) {
                        self.selected = vec![idx];
                    }
                    self.context_menu = Some((pos, Some(idx)));
                    self.touch();
                } else {
                    self.selected.clear();
                    self.context_menu = Some((pos, None));
                    self.touch();
                }
            }
        }
    }

    /// Показать контекстное меню, если оно открыто.
    pub(crate) fn show_context_menu_if_needed(&mut self, ctx: &egui::Context) {
        if let Some((menu_pos, target_idx)) = self.context_menu.clone() {
            self.show_context_menu(ctx, menu_pos, target_idx);
        }
    }

    /// Закрыть контекстное меню при левом клике вне меню.
    pub(crate) fn handle_close_menu(&mut self, response: &egui::Response) {
        let menu_was_open = self.context_menu.is_some();
        if response.clicked() && menu_was_open {
            self.context_menu = None;
        }
    }

    /// Показать контекстное меню у заданной позиции на экране.
    pub(crate) fn show_context_menu(&mut self, ctx: &egui::Context, menu_pos: Pos2, target_idx: Option<usize>) {
        let has_group = target_idx
            .and_then(|i| self.doc.shapes.get(i))
            .and_then(|s| s.group_id.as_ref())
            .is_some();
        let all_grouped = self.selected.iter()
            .all(|&i| self.doc.shapes.get(i).and_then(|s| s.group_id.as_ref()).is_some());
        let any_grouped = self.selected.iter()
            .any(|&i| self.doc.shapes.get(i).and_then(|s| s.group_id.as_ref()).is_some());
        let has_selection = !self.selected.is_empty();

        let mut close_menu = false;

        egui::Window::new("Контекстное меню")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .default_pos(menu_pos)
            .show(ctx, |ui| {
                ui.set_min_width(160.0);

                if has_selection && ui.button("Удалить фигуру").clicked() {
                    close_menu = true;
                }

                if has_selection {
                    ui.separator();

                    if ui.button("На передний план").clicked() {
                        self.send_to_front();
                        close_menu = true;
                    }
                    if ui.button("На задний план").clicked() {
                        self.send_to_back();
                        close_menu = true;
                    }
                }

                ui.separator();

                if ui.button("Копировать").clicked() {
                    self.copy_selected();
                    close_menu = true;
                }
                if self.clipboard.is_some() && ui.button("Вставить").clicked() {
                    self.paste_from_clipboard();
                    close_menu = true;
                }

                ui.separator();

                if target_idx.is_some() {
                    if has_group {
                        if ui.button("Разгруппировать").clicked() {
                            self.ungroup_selected();
                            close_menu = true;
                        }
                    } else if self.selected.len() > 1 {
                        if ui.button("Сгруппировать").clicked() {
                            self.group_selected();
                            close_menu = true;
                        }
                    } else if !all_grouped && !any_grouped {
                        if ui.button("Сгруппировать").clicked() {
                            self.group_selected();
                            close_menu = true;
                        }
                    }
                }

                if close_menu {
                    self.context_menu = None;
                }
            });

        let _ = Vec2::ZERO;
    }
}
