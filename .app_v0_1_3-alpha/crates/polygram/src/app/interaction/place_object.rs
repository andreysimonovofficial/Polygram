//! Модальное окно «Поместить Объект в Объект».
//! Единое окно для размещения Процесса в Отделе и Отдела в Отделе.
//! Запрещено: Процесс в Процесс, Отдел в Процесс (обрабатывается на этапе вызова).
//! Спецификация: Документация/Продукт/Интерфейс/Сущности/Модальные окна/Модальные окна - Поместить Объект в Объект - Оформление.md

use crate::app::PolygramApp;
use polygram_canvas::modal::{
    footer, header, modal_frame, paint_overlay,
    MODAL_BG, MODAL_BORDER, MODAL_TEXT, MODAL_TEXT_MUTED,
};
use polygram_canvas::paint_lucide;
use polygram_core::{Command, NodeKind, runtime::Published};
use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Frame, Margin, Rect, Sense, Stroke,
    StrokeKind, Vec2, Window,
};
use egui_lucide::Lucide;

/// Размер окна: 540 × 201 px (строго по макету).
const MODAL_SIZE: [f32; 2] = [540.0, 201.0];

/// Данные диалога «Поместить Объект в Объект».
#[derive(Clone)]
pub struct PlaceObjectConfirmData {
    /// Объект #1 (источник): id.
    pub object_id: String,
    /// Объект #1 (источник): имя.
    pub object_name: String,
    /// Объект #1 (источник): вид.
    pub object_kind: NodeKind,
    /// Объект #2 (цель): id.
    pub target_id: String,
    /// Объект #2 (цель): имя.
    pub target_name: String,
    /// Объект #2 (цель): вид.
    pub target_kind: NodeKind,
    /// Клетка размещения цели.
    pub drop_cell: (i32, i32),
    /// Стартовая клетка источника (для расчёта смещения потомков отдела).
    pub start_cell: (i32, i32),
}

/// Иконка Lucide для вида объекта.
fn object_icon(kind: NodeKind) -> Lucide {
    match kind {
        NodeKind::Process => Lucide::SquareEqual,
        NodeKind::Department => Lucide::Group,
        _ => Lucide::Square,
    }
}

/// Модальное окно «Поместить Объект в Объект».
pub fn handle_place_object_confirm(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
) {
    let Some(data) = app.dialogs.place_object_confirm.clone() else {
        return;
    };

    let mut done = false;
    let mut cancelled = false;

    let title = format!(
        "Поместить {} #{} в {} #{}?",
        data.object_name, data.object_id, data.target_name, data.target_id
    );
    let subtitle = format!("{} #{}", data.target_name, data.target_id);

    // Затемнённый фон позади модального окна (50%).
    paint_overlay(ctx);

    // Центрирование модального окна по экрану (540 × 201 px).
    let screen_rect = ctx.viewport_rect();
    let center_pos = egui::pos2(
        screen_rect.center().x - MODAL_SIZE[0] / 2.0,
        screen_rect.center().y - MODAL_SIZE[1] / 2.0,
    );

    Window::new("")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size(MODAL_SIZE)
        .fixed_pos(center_pos)
        .frame(modal_frame())
        .show(ctx, |ui| {
            // ---- Шапка диалога (отступы 20/20/20/12, фон #1E1E1E) ----
            let closed = header(ui, &title, &subtitle, Some(object_icon(data.target_kind)));
            if closed {
                cancelled = true;
            }

            // ---- Тело диалога (отступы секции: сверху 4, слева 20, справа 20, снизу 16) ----
            Frame::new()
                .fill(MODAL_BG)
                .inner_margin(Margin {
                    left: 20,
                    top: 4,
                    right: 20,
                    bottom: 16,
                })
                .show(ui, |ui| {
                    // Информационная плашка объекта: 498×34 px, bg #252525, рамка 1 px #373839, radius 2.
                    let plate_size = Vec2::new(498.0, 34.0);
                    let (plate_rect, _) =
                        ui.allocate_exact_size(plate_size, Sense::hover());
                    ui.painter()
                        .rect_filled(plate_rect, CornerRadius::same(2), Color32::from_rgb(0x25, 0x25, 0x25));
                    ui.painter().rect_stroke(
                        plate_rect,
                        CornerRadius::same(2),
                        Stroke::new(1.0, MODAL_BORDER),
                        StrokeKind::Middle,
                    );

                    // Содержимое плашки: иконка 16×16, название (flex-grow), идентификатор.
                    // Элементы выровнены строго по вертикальному центру плашки.
                    let plate_center_y = plate_rect.center().y;
                    let padding_x = 10.0;
                    let icon_size = 16.0;
                    let gap = 8.0;

                    // Иконка объекта #1: 16×16 px, #FFFFFF.
                    let icon_y = plate_center_y - icon_size / 2.0;
                    let icon_rect = Rect::from_min_size(
                        egui::pos2(plate_rect.min.x + padding_x, icon_y),
                        Vec2::splat(icon_size),
                    );
                    paint_lucide(
                        ui,
                        icon_rect,
                        object_icon(data.object_kind),
                        MODAL_TEXT,
                    );

                    // Название объекта #1: Inter 14 px, medium 500, #FFFFFF, flex-grow.
                    let name_font = FontId::new(14.0, FontFamily::Name("inter_medium".into()));
                    let name_galley = ui
                        .fonts_mut(|f| f.layout_no_wrap(data.object_name.clone(), name_font, MODAL_TEXT));
                    let name_y = plate_center_y - name_galley.size().y / 2.0;

                    // Идентификатор объекта #1: Inter 14 px, medium 500, #A1A1AA.
                    let id_text = format!("#{}", data.object_id);
                    let id_font = FontId::new(14.0, FontFamily::Name("inter_medium".into()));
                    let id_galley = ui
                        .fonts_mut(|f| f.layout_no_wrap(id_text, id_font, MODAL_TEXT_MUTED));
                    let id_y = plate_center_y - id_galley.size().y / 2.0;

                    // Название занимает оставшуюся ширину между иконкой и идентификатором.
                    let name_x = plate_rect.min.x + padding_x + icon_size + gap;
                    let name_max_x =
                        plate_rect.max.x - padding_x - gap - id_galley.size().x - gap;
                    let name_w = (name_max_x - name_x).max(0.0);

                    let name_rect = Rect::from_min_size(
                        egui::pos2(name_x, name_y),
                        Vec2::new(name_w.min(name_galley.size().x), name_galley.size().y),
                    );
                    ui.painter().galley(name_rect.min, name_galley, MODAL_TEXT);

                    // Идентификатор: прижат к правому краю плашки.
                    let id_x = plate_rect.max.x - padding_x - id_galley.size().x;
                    let id_rect = Rect::from_min_size(egui::pos2(id_x, id_y), id_galley.size());
                    ui.painter().galley(id_rect.min, id_galley, MODAL_TEXT_MUTED);
                });

            // ---- Нижняя панель действий (высота 52, отступы 20/20/20, вправо, по центру вертикали) ----
            let confirmed = footer(ui);
            if confirmed {
                done = true;
            }
        });

    if cancelled {
        app.dialogs.place_object_confirm = None;
        return;
    }

    if done {
        let mut cmds: Vec<Command> = vec![
            Command::SetObjectParent {
                object: data.object_id.clone(),
                parent: Some(data.target_id.clone()),
            },
            Command::MoveObject {
                object: data.object_id.clone(),
                at: [data.drop_cell.0, data.drop_cell.1],
            },
        ];
        // Отдел перевозит всех потомков с собой.
        if data.object_kind == NodeKind::Department {
            let children: Vec<String> = published
                .geometry
                .objects
                .iter()
                .filter(|o| o.parent.as_deref() == Some(&data.object_id))
                .map(|o| o.id.clone())
                .collect();
            let dx = data.drop_cell.0 - data.start_cell.0;
            let dy = data.drop_cell.1 - data.start_cell.1;
            for child_id in children {
                if let Some(child) = published
                    .geometry
                    .objects
                    .iter()
                    .find(|o| o.id == child_id)
                {
                    cmds.push(Command::MoveObject {
                        object: child_id.clone(),
                        at: [child.rect.x + dx, child.rect.y + dy],
                    });
                }
            }
        }
        app.apply_batch(cmds);
        app.dialogs.place_object_confirm = None;
    }
}
