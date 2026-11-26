//! Единая тёмная тема холста Polygram.
//! Источник истины для цветов и геометрии холста.

use egui::{Color32, CornerRadius, Margin, epaint::Shadow};
use polygram_core::types::{Direction, StatusCode};

/// Тёмная тема холста Polygram.
/// Содержит цвета и размеры, используемые при отрисовке фона и точечной сетки.
#[derive(Clone, Copy, Debug, Default)]
pub struct Theme {
    /// Фон холста.
    pub bg: Color32,
    /// Цвет точек сетки.
    pub grid_color: Color32,
    /// Размер одной клетки в мировых координатах.
    pub cell_size: f32,
    /// Радиус точки сетки в долях от размера клетки.
    pub dot_radius_world: f32,
    /// Тень окон и контекстных меню. По умолчанию без тени.
    pub shadow: Shadow,
    /// Скругление углов внешней рамки контекстного меню.
    pub menu_frame_rounding: CornerRadius,
    /// Скругление углов пунктов (кнопок) контекстного меню.
    pub menu_item_rounding: CornerRadius,
    /// Отступ внешней рамки контекстного меню от пунктов внутри.
    pub menu_frame_margin: Margin,
    /// Фон внешней рамки контекстного меню.
    pub menu_frame_fill: Color32,
    /// Цвет контура внешней рамки контекстного меню.
    pub menu_frame_stroke_color: Color32,
    /// Толщина контура внешней рамки контекстного меню.
    pub menu_frame_stroke_width: f32,
    /// Фон пунктов контекстного меню.
    pub menu_item_bg_fill: Color32,

    // ========== Объекты холста (процессы, отделы) ==========
    /// Заливка фона карточки процесса.
    pub card_bg: Color32,
    /// Граница карточки процесса.
    pub card_border: Color32,
    /// Заливка фона отдела.
    pub object_bg: Color32,
    /// Граница отдела.
    pub object_border: Color32,
    /// Акцентный цвет (рамка выделения, активные элементы).
    pub accent: Color32,
    /// Вторичный акцентный цвет.
    pub accent_secondary: Color32,
    /// Скругление углов отдела (в клетках).
    pub object_corner_radius: f32,
    /// Толщина границы отдела (в клетках).
    pub object_border_width: f32,
    /// Толщина границы отдела при наведении или выделении (в клетках).
    pub object_border_hover_width: f32,
    /// Высота шапки отдела (в клетках).
    pub object_header_height_cells: f32,
    /// Размер текста отдела.
    pub text_size: f32,
    /// Цвет основного текста отдела.
    pub text: Color32,
    /// Цвет приглушённого текста отдела (id).
    pub text_muted: Color32,
    /// Радиус порта точки сцепления.
    pub port_radius: f32,
    /// Толщина линии связи (пиксели при zoom = 1).
    pub edge_width: f32,
    /// Радиус скругления углов линии связи (пиксели при zoom = 1); 0 — острые углы.
    /// По умолчанию 6 px; на коротких коленах адаптируется под размер ступеньки.
    pub line_corner_radius: f32,

    // ========== Статусы ==========
    /// Цвет точки статуса «Ок» (зелёный).
    pub status_ok: Color32,
    /// Цвет точки статуса «Избыток» (жёлтый).
    pub status_warning: Color32,
    /// Цвет точки статуса «В процессе / Ожидается» (красный).
    pub status_deficit: Color32,

    // ========== Заливка фона отдела по агрегированному статусу ==========
    /// Заливка фона отдела со статусом «Ок».
    pub status_fill_ok: Color32,
    /// Заливка фона отдела со статусом «Избыток».
    pub status_fill_surplus: Color32,
    /// Заливка фона отдела со статусом «В процессе».
    pub status_fill_inprogress: Color32,
    /// Заливка фона отдела со статусом «Ожидается».
    pub status_fill_expected: Color32,
}

impl Theme {
    /// Создаёт тёмную тему по умолчанию.
    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(18, 18, 22),
            grid_color: Color32::from_rgb(49, 49, 58),
            cell_size: 16.0,
            dot_radius_world: 0.04,
            shadow: Shadow::NONE,
            menu_frame_rounding: CornerRadius::same(8),
            menu_item_rounding: CornerRadius::same(2),
            menu_frame_margin: Margin::same(4),
            menu_frame_fill: Color32::from_rgb(30, 30, 30),
            menu_frame_stroke_color: Color32::from_rgb(56, 56, 56),
            menu_frame_stroke_width: 0.5,
            menu_item_bg_fill: Color32::from_rgb(30, 30, 30),

            object_bg: Color32::from_rgb(28, 28, 34),
            object_border: Color32::from_rgb(60, 60, 72),
            card_bg: Color32::from_rgb(28, 28, 34),
            card_border: Color32::from_rgb(60, 60, 72),
            accent: Color32::from_rgb(255, 255, 255),
            accent_secondary: Color32::from_rgb(240, 240, 240),
            object_corner_radius: 2.0,
            object_border_width: 1.0,
            object_border_hover_width: 2.0,
            object_header_height_cells: 1.0,
            text_size: 14.0,
            text: Color32::from_rgb(230, 230, 235),
            text_muted: Color32::from_rgb(150, 150, 160),
            port_radius: 3.0,
            edge_width: 1.5,
            line_corner_radius: 6.0,

            status_ok: Color32::from_rgb(76, 175, 80),
            status_warning: Color32::from_rgb(255, 152, 0),
            status_deficit: Color32::from_rgb(244, 67, 54),

            status_fill_ok: Color32::from_rgb(15, 80, 25),
            status_fill_surplus: Color32::from_rgb(90, 70, 0),
            status_fill_inprogress: Color32::from_rgb(100, 45, 0),
            status_fill_expected: Color32::from_rgb(90, 15, 15),
        }
    }

    /// Заливка фона отдела по агрегированному статусу.
    pub fn status_fill_color(&self, status: StatusCode) -> Color32 {
        match status {
            StatusCode::Ok => self.status_fill_ok,
            StatusCode::Surplus => self.status_fill_surplus,
            StatusCode::InProgress => self.status_fill_inprogress,
            StatusCode::Expected => self.status_fill_expected,
        }
    }
}

/// Цвет точки статуса и линии связи с учётом направления (Запрос / Результат).
pub fn status_dot_color(status: StatusCode, dir: Direction, theme: &Theme) -> Color32 {
    use polygram_core::types::Direction;
    match status {
        StatusCode::Ok => theme.status_ok,
        StatusCode::Surplus => match dir {
            Direction::Request => theme.text_muted,
            Direction::Result => theme.status_warning,
        },
        StatusCode::InProgress | StatusCode::Expected => theme.status_deficit,
    }
}

/// Метка статуса по-русски с учётом направления (всегда верхний регистр).
pub fn status_label_ru(status: StatusCode, dir: Direction) -> &'static str {
    use polygram_core::types::Direction;
    match status {
        StatusCode::Ok => match dir {
            Direction::Request => "УДОВЛЕТВОРЁН",
            Direction::Result => "ОБЕСПЕЧЕН",
        },
        StatusCode::Surplus => "В ИЗБЫТКЕ",
        StatusCode::InProgress => "В ПРОЦЕССЕ",
        StatusCode::Expected => "ОЖИДАЕТСЯ",
    }
}
