//! Обработка ввода на холсте: зум, панорамирование, перетаскивание, магнитное выравнивание,
//! создание фигур, предварительный просмотр, контекстное меню и группировка.

mod camera;
mod context_menu;
mod groups;
mod hierarchy;
mod selection;
mod shapes;

pub(crate) use camera::*;
pub(crate) use context_menu::*;
pub(crate) use groups::*;
pub(crate) use hierarchy::*;
pub(crate) use selection::*;
pub(crate) use shapes::*;

use egui::Sense;
use std::collections::HashSet;

/// Режим изменения размера фигуры за угол или сторону.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeMode {
    NW, N, NE,
    W,     E,
    SW, S, SE,
}

impl crate::free_canvas::FreeCanvas {
    /// Центральная область: холст с обработкой ввода.
    pub(crate) fn canvas_panel(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let canvas_rect = egui::Rect::from_min_size(ui.cursor().min, available);

        // Обработка ввода сначала (избегаем конфликта заимствований с painter)
        let response = ui.allocate_rect(canvas_rect, Sense::click_and_drag());

        self.handle_camera(ui, &response, canvas_rect);
        self.handle_prototype(ui, canvas_rect);
        self.draw_background(ui, canvas_rect);

        let camera = self.camera;
        let zoom = self.zoom.clamp(0.1, 10.0);

        let mut magnetic_lines: Vec<(f32, bool)> = Vec::new();

        self.draw_selection(ui, &response, zoom);
        self.handle_context_menu_trigger(&response);
        self.show_context_menu_if_needed(ui.ctx());
        self.handle_close_menu(&response);
        self.handle_text_click(ui, &response);
        self.handle_select_click(ui, &response);
        self.handle_drag_start(ui, &response);
        self.handle_drag(ui, &response, &mut magnetic_lines);
        self.handle_resize(ui, &response, &mut magnetic_lines);
        self.handle_drag_stopped(ui, &response, &mut magnetic_lines);
        self.draw_magnetic_lines(ui, canvas_rect, &magnetic_lines, zoom, camera);
        self.draw_preview(ui, &response, canvas_rect, zoom);

        ui.allocate_rect(canvas_rect, Sense::hover());
    }

    /// Определить визуальное состояние фигуры с учёта множеств наведения/нажатия.
    fn shape_state(
        hovered: &HashSet<usize>,
        pressed: &HashSet<usize>,
        idx: usize,
    ) -> crate::design_format::ShapeState {
        if pressed.contains(&idx) {
            crate::design_format::ShapeState::Pressed
        } else if hovered.contains(&idx) {
            crate::design_format::ShapeState::Hover
        } else {
            crate::design_format::ShapeState::Normal
        }
    }
}
