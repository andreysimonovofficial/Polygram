use crate::app::drag::DragInteractionState;
use crate::app::PolygramApp;
use polygram_canvas::theme::Theme;
use polygram_core::{
    geometry::Geometry, geometry::ObjectGeom, layout::Rect as CoreRect, types::NodeKind, Published,
};
use std::sync::mpsc;

use egui::{Context, Event, PointerButton, Pos2, RawInput, Rect, Vec2};

use super::handle_canvas_interaction;

fn make_app() -> PolygramApp {
    let (_tx, rx) = mpsc::channel::<std::sync::Arc<Published>>();
    let mut app = PolygramApp::new(rx);
    let geometry = Geometry {
        rev: 1,
        objects: vec![ObjectGeom {
            ix: 0,
            id: "obj1".into(),
            name: "Объект 1".into(),
            kind: NodeKind::Process,
            rect: CoreRect::new(10, 10, 16, 16),
            requests: vec![],
            results: vec![],
            parent: None,
        }],
        edges: vec![],
    };
    let published = Published {
        rev: 1,
        dynamic: Default::default(),
        geometry,
        metric_defs: vec![],
        camera_fit_requested: false,
        auto_screenshot: false,
        virtual_input: Default::default(),
    };
    app.connection.snapshot = Some(std::sync::Arc::new(published));
    app
}

/// Эмулирует нажатие мыши на объект и начало перетаскивания через egui.
fn run_frame_button(
    ctx: &Context,
    app: &mut PolygramApp,
    pos: Pos2,
    button: PointerButton,
    pressed: bool,
    published: &std::sync::Arc<Published>,
    close_menu: &mut bool,
    theme: &Theme,
) {
    let mut raw_input = RawInput::default();
    raw_input.screen_rect = Some(Rect::EVERYTHING);
    raw_input.events.push(Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Default::default(),
    });
    ctx.begin_pass(raw_input);
    ctx.root_ui(|ui| {
        handle_canvas_interaction(app, ctx, ui, published, close_menu, theme);
    });
    let mut output = ctx.end_pass();
    output.textures_delta.clear();
}

#[test]
fn gui_drag_object_across_canvas() {
    let ctx = Context::default();
    let mut app = make_app();
    let theme = app.canvas.theme.clone();

    // Kamera по умолчанию: offset = (0,0), zoom = 1.0, cell_size = 16.0.
    // Объект obj1 на клетке (10, 10) -> экранная позиция (160, 160).
    let cell_px = 16.0;
    let start_screen = Pos2::new(10.0 * cell_px, 10.0 * cell_px);
    let end_cell = (20, 10);

    let mut close_menu = false;
    let published = app.connection.snapshot.as_ref().unwrap().clone();

    // ---- Кадр 1: нажатие на объект (через egui мышь) ----
    run_frame_button(
        &ctx,
        &mut app,
        start_screen,
        PointerButton::Primary,
        true,
        &published,
        &mut close_menu,
        &theme,
    );

    // ---- Старт перетаскивания (имитируем egui drag_started_by) ----
    app.drag
        .start_drag_object("obj1".into(), start_screen, (10, 10), Vec2::ZERO);
    assert!(
        matches!(app.drag.state, DragInteractionState::DraggingObject { .. }),
        "ожидался старт перетаскивания, получено {:?}",
        app.drag.state
    );
    assert_eq!(
        app.drag.drag_object(),
        Some("obj1"),
        "перетаскиваемый объект должен быть obj1"
    );

    // ---- Движение мыши к целевой клетке (клиентское предсказание) ----
    app.drag.update_prediction("obj1", end_cell);
    assert_eq!(
        app.drag.predicted_positions.get("obj1"),
        Some(&end_cell),
        "предсказанная позиция должна быть {:?}",
        end_cell
    );

    // ---- Дроп объекта ----
    let result = app.drag.end_drag_object();
    assert_eq!(
        result, Some(("obj1".to_string(), (10, 10), None)),
        "end_drag_object должен вернуть объект и стартовую клетку (target нет без begin_drop_object)"
    );
    assert!(
        matches!(app.drag.state, DragInteractionState::Idle),
        "после дропа должно быть Idle, получено {:?}",
        app.drag.state
    );
    // Предсказанная позиция остаётся (клиентское предсказание применено).
    assert_eq!(
        app.drag.predicted_positions.get("obj1"),
        Some(&end_cell),
        "предсказанная позиция после дропа должна остаться {:?}",
        end_cell
    );
}
