use egui::Color32;

/// Цвета темы.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Color32,
    pub grid: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub card_header: Color32,
    pub line: Color32,
    pub line_hover: Color32,
    pub status_ok: Color32,
    pub status_warning: Color32,
    pub status_deficit: Color32,
    pub status_surplus: Color32,
    pub status_unconnected: Color32,
    pub status_invalid: Color32,
    pub text: Color32,
    pub text_muted: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color32::from_rgb(18, 18, 22),
            grid: Color32::from_rgb(35, 35, 42),
            card_bg: Color32::from_rgb(28, 28, 34),
            card_border: Color32::from_rgb(60, 60, 72),
            card_header: Color32::from_rgb(45, 45, 54),
            line: Color32::from_rgb(120, 120, 135),
            line_hover: Color32::from_rgb(160, 160, 180),
            status_ok: Color32::from_rgb(76, 175, 80),
            status_warning: Color32::from_rgb(255, 193, 7),
            status_deficit: Color32::from_rgb(244, 67, 54),
            status_surplus: Color32::from_rgb(3, 169, 244),
            status_unconnected: Color32::from_rgb(100, 100, 110),
            status_invalid: Color32::from_rgb(158, 158, 158),
            text: Color32::from_rgb(230, 230, 235),
            text_muted: Color32::from_rgb(150, 150, 160),
        }
    }
}
