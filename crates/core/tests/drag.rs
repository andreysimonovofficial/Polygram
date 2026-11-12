//! Тесты чистой доменной логики дроп объекта (compute_drop_outcome, compute_drag_preview).
use polygram_core::drag_domain::{
    compute_drag_preview, compute_drop_outcome, DragPreviewInput, DropContext, DropOutcome,
};
use polygram_core::geometry::{Geometry, ObjectGeom};
use polygram_core::layout::Rect;
use polygram_core::types::NodeKind;

fn obj(id: &str, x: i32, y: i32, w: i32, h: i32, kind: NodeKind) -> ObjectGeom {
    ObjectGeom {
        ix: 0,
        id: id.to_owned(),
        name: id.to_owned(),
        kind,
        rect: Rect::new(x, y, w, h),
        requests: vec![],
        outputs: vec![],
        parent: None,
    }
}

fn geometry_with(objects: Vec<ObjectGeom>) -> Geometry {
    Geometry {
        rev: 1,
        objects,
        edges: vec![],
    }
}

#[test]
fn drop_in_empty_cell_moves_object() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(
        compute_drop_outcome(&ctx),
        DropOutcome::Move { at: (10, 10) }
    );
}

#[test]
fn drop_on_department_with_ghost_reparents_and_moves() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 4, 4, NodeKind::Department),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 5),
        over_dept: Some("d".to_owned()),
        has_ghost: true,
        dragged_size: (2, 2),
    };
    assert_eq!(
        compute_drop_outcome(&ctx),
        DropOutcome::ReparentAndMove { dept_id: "d".to_owned(), at: (10, 5) }
    );
}

#[test]
fn preview_over_empty_space_returns_cursor_cell() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (10, 10),
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.ghost, None);
    assert_eq!(preview.over, None);
    assert_eq!(preview.predicted, (10, 10));
}

#[test]
fn preview_over_department_with_child_finds_ghost_near_child() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 10, 10, NodeKind::Department),
        obj("c", 7, 7, 2, 2, NodeKind::Process),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (12, 12), // внутри отдела d, вне ребёнка c
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.over, Some("d".to_owned()));
    // Призрачная клетка найдена спиральным поиском (вне rect отдела), predicted = ghost
    assert_eq!(preview.predicted, preview.ghost.unwrap());
    assert!(preview.ghost.is_some());
}

#[test]
fn preview_over_department_without_children_finds_ghost_in_dept() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 10, 10, NodeKind::Department),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (8, 8),
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.over, Some("d".to_owned()));
    assert!(preview.ghost.is_some());
}

#[test]
fn preview_over_process_returns_process_over() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("p", 5, 5, 4, 4, NodeKind::Process),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (6, 6), // над процессом p
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.over, Some("p".to_owned()));
    assert!(preview.ghost.is_some()); // свободная клетка рядом с p
}

#[test]
fn drop_on_department_without_ghost_cancels() {
    // drop_cell == start_cell означает, что призрачной клетки нет (место занято).
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 10, 10, NodeKind::Department),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (0, 0),
        over_dept: Some("d".to_owned()),
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(compute_drop_outcome(&ctx), DropOutcome::Cancel);
}

#[test]
fn drop_on_occupied_cell_cancels() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("b", 10, 10, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(compute_drop_outcome(&ctx), DropOutcome::Cancel);
}

#[test]
fn drop_to_same_cell_cancels() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (0, 0),
        over_dept: None,
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(compute_drop_outcome(&ctx), DropOutcome::Cancel);
}

#[test]
fn drop_over_non_department_is_move() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("p", 5, 5, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: Some("p".to_owned()),
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(
        compute_drop_outcome(&ctx),
        DropOutcome::Move { at: (10, 10) }
    );
}
