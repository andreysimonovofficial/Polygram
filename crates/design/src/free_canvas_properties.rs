//! Панель свойств выделенной фигуры (справа).

use crate::design_format::{DrawTool, FreeShape, FontFamily, FontWeight, Rgba, StateStyle, StrokePosition, TextAlign, TextAlignV, ChildReactMode};
use crate::free_canvas::FreeCanvas;
use egui::{Color32, DragValue};

impl FreeCanvas {
    /// Панель свойств выделенной фигуры (справа).
    pub(crate) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Свойства");
        ui.separator();

        ui.label("Цвет фона холста:");
        let old_bg = self.doc.bg;
        Self::color_picker(ui, &mut self.doc.bg);
        if old_bg != self.doc.bg {
            self.save_undo_state();
            self.touch();
        }
        ui.separator();

        ui.checkbox(&mut self.magnetic, "Магнитное выравнивание");
        ui.separator();

        ui.label(format!("Фигуры: {}", self.doc.shapes.len()));
        ui.label(format!("Выбрано: {}", self.selected.len()));
        if ui.button("Удалить выделенное").clicked() {
            self.delete_selected();
            self.touch();
        }
        if ui.button("Очистить").clicked() {
            self.save_undo_state();
            self.doc.shapes.clear();
            self.selected.clear();
            self.touch();
        }

        ui.separator();
        ui.heading("Свойства фигуры");
        ui.separator();

        match self.selected.first().copied() {
            Some(idx) => {
                let prev_doc = self.doc.clone();
                let (changed, delete) = Self::edit_shape(ui, &mut self.doc.shapes[idx]);
                if changed {
                    self.push_undo_state(prev_doc);
                    self.touch();
                    self.reindex_containment();
                } else if delete {
                    self.doc.shapes.remove(idx);
                    self.selected.clear();
                    self.touch();
                }

                ui.separator();
                if ui.button("На передний план").clicked() {
                    self.send_to_front();
                }
                if ui.button("На задний план").clicked() {
                    self.send_to_back();
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
            let mut color = Color32::from_rgba_unmultiplied(fill.r, fill.g, fill.b, fill.a);
            if ui.color_edit_button_srgba(&mut color).changed() {
                fill.r = color.r();
                fill.g = color.g();
                fill.b = color.b();
                fill.a = color.a();
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
        ui.horizontal(|ui| {
            ui.label("Позиция обводки:");
            let mut position = shape.stroke_position;
            egui::ComboBox::new("stroke_position", "")
                .selected_text(match position {
                    StrokePosition::Center => "По центру",
                    StrokePosition::Inside => "Внутри",
                    StrokePosition::Outside => "Снаружи",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut position, StrokePosition::Center, "По центру").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut position, StrokePosition::Inside, "Внутри").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut position, StrokePosition::Outside, "Снаружи").changed() {
                        changed = true;
                    }
                });
            shape.stroke_position = position;
        });
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
            ui.label("Шрифт:");
            let mut family = shape.font_family;
            egui::ComboBox::new("text_font_family", "")
                .selected_text(match family {
                    FontFamily::Arial => "Arial",
                    FontFamily::Inter => "Inter",
                    FontFamily::SegoeUI => "Segoe UI",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut family, FontFamily::Arial, "Arial").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut family, FontFamily::Inter, "Inter").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut family, FontFamily::SegoeUI, "Segoe UI").changed() {
                        changed = true;
                    }
                });
            shape.font_family = family;
        });
        ui.horizontal(|ui| {
            ui.label("Насыщенность:");
            let mut weight = shape.font_weight;
            egui::ComboBox::new("text_font_weight", "")
                .selected_text(match weight {
                    FontWeight::Thin => "Thin",
                    FontWeight::ExtraLight => "Extra Light",
                    FontWeight::Light => "Light",
                    FontWeight::Regular => "Regular",
                    FontWeight::Medium => "Medium",
                    FontWeight::SemiBold => "Semi Bold",
                    FontWeight::Bold => "Bold",
                    FontWeight::ExtraBold => "Extra Bold",
                    FontWeight::Black => "Black",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut weight, FontWeight::Thin, "Thin").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::ExtraLight, "Extra Light").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::Light, "Light").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::Regular, "Regular").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::Medium, "Medium").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::SemiBold, "Semi Bold").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::Bold, "Bold").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::ExtraBold, "Extra Bold").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut weight, FontWeight::Black, "Black").changed() {
                        changed = true;
                    }
                });
            shape.font_weight = weight;
        });
        ui.label("Выравнивание по горизонтали:");
            let mut align = shape.text_align.clone();
            egui::ComboBox::new("text_align_h", "")
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
            ui.label("Выравнивание по вертикали:");
            let mut align_v = shape.text_align_v;
            egui::ComboBox::new("text_align_v", "")
                .selected_text(match align_v {
                    TextAlignV::Top => "По верхнему",
                    TextAlignV::Middle => "По центру",
                    TextAlignV::Bottom => "По нижнему",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut align_v, TextAlignV::Top, "По верхнему").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut align_v, TextAlignV::Middle, "По центру").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut align_v, TextAlignV::Bottom, "По нижнему").changed() {
                        changed = true;
                    }
                });
            shape.text_align_v = align_v;
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

        }

        ui.separator();
        ui.heading("Состояния (прототип)");
        ui.separator();

        // Наведение
        ui.label("Наведение (Hover):");
        let mut hover_enabled = shape.states.hover.is_some();
        if ui.checkbox(&mut hover_enabled, "Переопределить стиль при наведении").changed() {
            if hover_enabled {
                shape.states.hover = Some(StateStyle::default());
            } else {
                shape.states.hover = None;
            }
            changed = true;
        }
        if let Some(st) = shape.states.hover.as_mut() {
            ui.horizontal(|ui| {
                ui.label("  Заливка:");
                let mut color = Color32::from_rgb(st.fill.map_or(0, |c| c.r), st.fill.map_or(0, |c| c.g), st.fill.map_or(0, |c| c.b));
                if ui.color_edit_button_srgba(&mut color).changed() {
                    st.fill = Some(Rgba { r: color.r(), g: color.g(), b: color.b(), a: 255 });
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("  Обводка:");
                let mut color = Color32::from_rgb(st.stroke.map_or(0, |c| c.r), st.stroke.map_or(0, |c| c.g), st.stroke.map_or(0, |c| c.b));
                if ui.color_edit_button_srgba(&mut color).changed() {
                    st.stroke = Some(Rgba { r: color.r(), g: color.g(), b: color.b(), a: 255 });
                    changed = true;
                }
            });
            changed |= ui
                .add(
                    DragValue::new(&mut st.stroke_width.unwrap_or(shape.stroke_width))
                        .prefix("  Толщина обводки: ")
                        .speed(0.1)
                        .range(0.0..=20.0),
                )
                .changed();
            if st.stroke_width.is_none() {
                st.stroke_width = Some(shape.stroke_width);
            }
        }

        // Нажатие
        ui.label("Нажатие (Pressed):");
        let mut pressed_enabled = shape.states.pressed.is_some();
        if ui.checkbox(&mut pressed_enabled, "Переопределить стиль при нажатии").changed() {
            if pressed_enabled {
                shape.states.pressed = Some(StateStyle::default());
            } else {
                shape.states.pressed = None;
            }
            changed = true;
        }
        if let Some(st) = shape.states.pressed.as_mut() {
            ui.horizontal(|ui| {
                ui.label("  Заливка:");
                let mut color = Color32::from_rgb(st.fill.map_or(0, |c| c.r), st.fill.map_or(0, |c| c.g), st.fill.map_or(0, |c| c.b));
                if ui.color_edit_button_srgba(&mut color).changed() {
                    st.fill = Some(Rgba { r: color.r(), g: color.g(), b: color.b(), a: 255 });
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("  Обводка:");
                let mut color = Color32::from_rgb(st.stroke.map_or(0, |c| c.r), st.stroke.map_or(0, |c| c.g), st.stroke.map_or(0, |c| c.b));
                if ui.color_edit_button_srgba(&mut color).changed() {
                    st.stroke = Some(Rgba { r: color.r(), g: color.g(), b: color.b(), a: 255 });
                    changed = true;
                }
            });
            changed |= ui
                .add(
                    DragValue::new(&mut st.stroke_width.unwrap_or(shape.stroke_width))
                        .prefix("  Толщина обводки: ")
                        .speed(0.1)
                        .range(0.0..=20.0),
                )
                .changed();
            if st.stroke_width.is_none() {
                st.stroke_width = Some(shape.stroke_width);
            }
        }

        // Группа
        ui.separator();
        ui.label("Группа (прототип):");
        let mut group_id = shape.group_id.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut group_id).changed() {
            if group_id.is_empty() {
                shape.group_id = None;
            } else {
                shape.group_id = Some(group_id);
            }
            changed = true;
        }
        if shape.group_id.is_some() {
            if ui.button("Убрать из группы").clicked() {
                shape.group_id = None;
                changed = true;
            }
        }

        ui.separator();
        ui.label("Иерархия (контейнеры):");
        ui.separator();

        // is_container
        let mut is_container = shape.is_container;
        if ui.checkbox(&mut is_container, "Рамка-контейнер (распространяет наведение/нажатие на детей)").changed() {
            shape.is_container = is_container;
            changed = true;
        }

        // parent_id
        let mut parent_id = shape.parent_id.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut parent_id).changed() {
            shape.parent_id = if parent_id.is_empty() { None } else { Some(parent_id) };
            changed = true;
        }
        if shape.parent_id.is_some() {
            if ui.button("Убрать родителя").clicked() {
                shape.parent_id = None;
                changed = true;
            }
            // reacts_to_parent
            let mut reacts = shape.reacts_to_parent;
            if ui.checkbox(&mut reacts, "Реагировать на состояние родителя").changed() {
                shape.reacts_to_parent = reacts;
                changed = true;
            }
        }

        // child_react_mode (only for containers)
        if shape.is_container {
            ui.label("Режим реакции детей:");
            let mut mode = shape.child_react_mode;
            egui::ComboBox::from_label("")
                .selected_text(match mode {
                    ChildReactMode::All => "Все дети",
                    ChildReactMode::List => "Список детей",
                    ChildReactMode::None => "Никто",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut mode, ChildReactMode::All, "Все дети").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut mode, ChildReactMode::List, "Список детей").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut mode, ChildReactMode::None, "Никто").changed() {
                        changed = true;
                    }
                });
            shape.child_react_mode = mode;

            if mode == ChildReactMode::List {
                let mut allow = shape.child_react_allow.join(", ");
                if ui.text_edit_singleline(&mut allow).changed() {
                    shape.child_react_allow = allow
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect();
                    changed = true;
                }
                ui.label("id детей через запятую, которые горят с родителем.");
            }
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
