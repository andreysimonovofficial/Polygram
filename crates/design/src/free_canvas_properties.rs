//! Панель свойств выделенной фигуры (справа).

use crate::design_format::{DrawTool, FreeShape, Rgba, TextAlign};
use crate::free_canvas::FreeCanvas;
use egui::{Color32, DragValue};

impl FreeCanvas {
    /// Панель свойств выделенной фигуры (справа).
    pub(crate) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Свойства");
        ui.separator();

        ui.label("Цвет фона холста:");
        Self::color_picker(ui, &mut self.doc.bg);
        self.touch();
        ui.separator();

        ui.checkbox(&mut self.magnetic, "Магнитное выравнивание");
        ui.separator();

        ui.label(format!("Фигуры: {}", self.doc.shapes.len()));
        if ui.button("Удалить выделенное").clicked() {
            self.delete_selected();
            self.touch();
        }
        if ui.button("Очистить").clicked() {
            self.doc.shapes.clear();
            self.selected = None;
            self.touch();
        }

        ui.separator();
        ui.heading("Свойства фигуры");
        ui.separator();

        match self.selected {
            Some(idx) => {
                let (changed, delete) = Self::edit_shape(ui, &mut self.doc.shapes[idx]);
                if delete {
                    self.doc.shapes.remove(idx);
                    self.selected = None;
                    self.touch();
                } else if changed {
                    self.touch();
                }
            }
            None => {
                ui.label("Ничего не выбрано.");
                ui.label("Нажмите на фигуру на холсте, чтобы выбрать.");
            }
        }
    }

    /// Редактировать свойства одной фигуры. Возвращает (изменено, удалить).
    pub(crate) fn edit_shape(ui: &mut egui::Ui, shape: &mut FreeShape) -> (bool, bool) {
        let mut changed = false;
        let mut delete = false;

        ui.label(format!("Фигура #{}", shape.id));
        ui.separator();

        ui.label("Инструмент:");
        let mut tool = shape.tool;
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", tool))
            .show_ui(ui, |ui| {
                for t in [DrawTool::Rect, DrawTool::Line, DrawTool::Ellipse, DrawTool::Text] {
                    if ui.selectable_value(&mut tool, t, format!("{:?}", t)).changed() {
                        changed = true;
                    }
                }
            });
        shape.tool = tool;

        ui.horizontal(|ui| {
            ui.label("X:");
            changed |= ui.add(DragValue::new(&mut shape.x).speed(1.0)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Y:");
            changed |= ui.add(DragValue::new(&mut shape.y).speed(1.0)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Ширина:");
            changed |= ui
                .add(
                    DragValue::new(&mut shape.w)
                        .speed(1.0)
                        .range(0.0..=10000.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Высота:");
            changed |= ui
                .add(
                    DragValue::new(&mut shape.h)
                        .speed(1.0)
                        .range(0.0..=10000.0),
                )
                .changed();
        });

        ui.separator();
        ui.label("Заливка:");
        if let Some(fill) = &mut shape.fill {
            let mut color = Color32::from_rgb(fill.r, fill.g, fill.b);
            if ui.color_edit_button_srgba(&mut color).changed() {
                fill.r = color.r();
                fill.g = color.g();
                fill.b = color.b();
                changed = true;
            }
        } else {
            if ui.button("Добавить заливку").clicked() {
                shape.fill = Some(Rgba { r: 0, g: 0, b: 0, a: 255 });
                changed = true;
            }
        }
        ui.horizontal(|ui| {
            ui.label("Обводка:");
            let mut stroke_color = Color32::from_rgb(shape.stroke.r, shape.stroke.g, shape.stroke.b);
            if ui.color_edit_button_srgba(&mut stroke_color).changed() {
                shape.stroke.r = stroke_color.r();
                shape.stroke.g = stroke_color.g();
                shape.stroke.b = stroke_color.b();
                changed = true;
            }
        });
        changed |= ui
            .add(
                DragValue::new(&mut shape.stroke_width)
                    .prefix("Толщина обводки: ")
                    .speed(0.1)
                    .range(0.0..=20.0),
            )
            .changed();

        ui.separator();
        ui.label("Скругление углов:");
        let mut all = shape.corner_radius;
        if ui
            .add(
                DragValue::new(&mut all)
                    .prefix("Все углы: ")
                    .speed(0.5)
                    .range(0.0..=100.0),
            )
            .changed()
        {
            shape.corner_tl = all;
            shape.corner_tr = all;
            shape.corner_bl = all;
            shape.corner_br = all;
            shape.corner_radius = all;
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.label("Левый верхний:");
            changed |= ui
                .add(
                    DragValue::new(&mut shape.corner_tl)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Правый верхний:");
            changed |= ui
                .add(
                    DragValue::new(&mut shape.corner_tr)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Левый нижний:");
            changed |= ui
                .add(
                    DragValue::new(&mut shape.corner_bl)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Правый нижний:");
            changed |= ui
                .add(
                    DragValue::new(&mut shape.corner_br)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });

        if shape.tool == DrawTool::Text {
            ui.separator();
            ui.label("Текст:");
            changed |= ui.text_edit_singleline(&mut shape.text).changed();
            changed |= ui
                .add(
                    DragValue::new(&mut shape.font_size)
                        .prefix("Размер шрифта: ")
                        .speed(0.5)
                        .range(4.0..=200.0),
                )
                .changed();
            ui.horizontal(|ui| {
                ui.label("Цвет текста:");
                let mut text_color = Color32::from_rgb(
                    shape.text_color.r,
                    shape.text_color.g,
                    shape.text_color.b,
                );
                if ui.color_edit_button_srgba(&mut text_color).changed() {
                    shape.text_color.r = text_color.r();
                    shape.text_color.g = text_color.g();
                    shape.text_color.b = text_color.b();
                    changed = true;
                }
            });
            ui.label("Выравнивание:");
            let mut align = shape.text_align.clone();
            egui::ComboBox::from_label("")
                .selected_text(match align {
                    TextAlign::Left => "По левому",
                    TextAlign::Center => "По центру",
                    TextAlign::Right => "По правому",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut align, TextAlign::Left, "По левому").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut align, TextAlign::Center, "По центру").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut align, TextAlign::Right, "По правому").changed() {
                        changed = true;
                    }
                });
            shape.text_align = align;
        }

        ui.separator();
        if ui.button("Удалить фигуру").clicked() {
            delete = true;
        }

        (changed, delete)
    }

    pub(crate) fn color_picker(ui: &mut egui::Ui, color: &mut Rgba) {
        let mut egui_color = Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a);
        ui.color_edit_button_srgba(&mut egui_color);
        color.r = egui_color.r();
        color.g = egui_color.g();
        color.b = egui_color.b();
        color.a = egui_color.a();
    }
}
