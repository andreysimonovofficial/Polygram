//! Тесты слоя: кэш, ленивый пересчёт, терминалы, перевод в экран, цвет и скругление.

use egui::{Pos2, Vec2};
use polygram_core::geometry::{EdgeGeom, EntryGeom, ObjectGeom};
use polygram_core::layout::Rect;
use polygram_core::types::{Direction, MetricValue, NodeKind, StatusCode};
use polygram_core::DynamicState;

use super::layer::{EdgeLayer, EdgeRef};
use super::model::{Quality, Side, U};
use super::ports;
use super::screen::route_to_screen;
use crate::paint::edge::{edge_color, rounded_points};
use crate::paint::object::process_total_height;
use crate::Theme;

fn entry(ix: u32) -> EntryGeom {
    EntryGeom {
        ix,
        name: format!("m{ix}"),
        value: MetricValue::Null,
        supplied: MetricValue::Null,
        unit: String::new(),
    }
}

/// Процесс: запросы получают ключи `base..base+n_req`, результаты — следующие `n_res`.
fn process(ix: u32, x: i32, y: i32, n_req: u32, n_res: u32, base: u32) -> ObjectGeom {
    ObjectGeom {
        ix,
        id: format!("p{ix}"),
        name: format!("Процесс {ix}"),
        kind: NodeKind::Process,
        rect: Rect::new(x, y, 32, 17),
        direction: None,
        requests: (0..n_req).map(|i| entry(base + i)).collect(),
        results: (0..n_res).map(|i| entry(base + n_req + i)).collect(),
        parent: None,
    }
}

fn department(ix: u32, rect: Rect) -> ObjectGeom {
    ObjectGeom {
        ix,
        id: format!("d{ix}"),
        name: "Отдел".into(),
        kind: NodeKind::Department,
        rect,
        direction: None,
        requests: vec![],
        results: vec![],
        parent: None,
    }
}

/// Два процесса: источник (ключи 0..4, результат — ключ 2), цель (ключи 10..14, запрос — ключ 10).
fn scene(dst_x: i32, dst_y: i32) -> (Vec<ObjectGeom>, Vec<EdgeGeom>) {
    let a = process(0, 0, 0, 2, 2, 0);
    let b = process(1, dst_x, dst_y, 2, 2, 10);
    (vec![a, b], vec![EdgeGeom { ix: 0, from: 2, to: 10 }])
}

fn first_edge(l: &EdgeLayer) -> EdgeRef {
    l.edges()[0]
}

#[test]
fn card_height_matches_painter() {
    let dynamic = DynamicState::default();
    for (nq, nr) in [(0usize, 0usize), (1, 0), (0, 3), (2, 2), (4, 7)] {
        let o = process(0, 0, 0, nq as u32, nr as u32, 0);
        let painted = process_total_height(&o, &dynamic, 1.0, 512.0);
        let (w, h) = ports::card_size_px(nq, nr);
        assert!((painted - h).abs() < 1e-3, "{nq}/{nr}: {painted} vs {h}");
        assert_eq!(w, 512.0);
    }
}

#[test]
fn ports_sit_on_row_edges_at_row_centers() {
    // Запрос 0: контейнер начинается на 80, строка — на 80+8+48 = 136, центр 152.
    assert_eq!(ports::row_port_px(2, Direction::Request, 0, Side::Left), (16.0, 152.0));
    assert_eq!(ports::row_port_px(2, Direction::Request, 1, Side::Right), (496.0, 192.0));
    // Результат при двух запросах: 80 + (56+80) + 8 = 224; центр строки 0 = 224+8+48+16 = 296.
    assert_eq!(ports::row_port_px(2, Direction::Result, 0, Side::Left), (16.0, 296.0));
}

#[test]
fn route_is_cached_until_geometry_changes() {
    let (objs, edges) = scene(40, 6);
    let mut l = EdgeLayer::new();
    l.sync(&objs, &edges, 16.0);
    let e = first_edge(&l);
    let r1 = l.route(&e).expect("маршрут");
    assert_eq!(l.route_calls, 1);
    assert_eq!(r1.quality, Quality::Clean);

    // Тот же снимок — ничего не пересчитывается.
    l.sync(&objs, &edges, 16.0);
    let r2 = l.route(&e).unwrap();
    assert_eq!(r1, r2);
    assert_eq!(l.route_calls, 1);

    // Сдвиг карточки цели по Y меняет маршрут.
    let (objs2, edges2) = scene(40, 18);
    l.sync(&objs2, &edges2, 16.0);
    let r3 = l.route(&e).unwrap();
    assert_eq!(l.route_calls, 2);
    assert_ne!(r1.points, r3.points);
}

#[test]
fn far_card_does_not_recompute_but_own_card_does() {
    let (mut objs, edges) = scene(40, 6);
    objs.push(process(2, 900, 900, 1, 1, 20));
    let mut l = EdgeLayer::new();
    l.sync(&objs, &edges, 16.0);
    let e = first_edge(&l);
    l.route(&e).unwrap();
    assert_eq!(l.route_calls, 1);

    // Далёкая карточка сдвинулась — пересчёта нет.
    objs[2].rect.x += 7;
    l.sync(&objs, &edges, 16.0);
    l.route(&e).unwrap();
    assert_eq!(l.route_calls, 1);

    // Сдвинулась карточка самой связи — ровно один пересчёт.
    objs[0].rect.y += 7;
    l.sync(&objs, &edges, 16.0);
    l.route(&e).unwrap();
    assert_eq!(l.route_calls, 2);
}

#[test]
fn huge_department_frame_is_not_an_obstacle() {
    let (mut objs, edges) = scene(40, 6);
    objs.insert(0, department(9, Rect::new(-5, -5, 2000, 2000)));
    let mut l = EdgeLayer::new();
    l.sync(&objs, &edges, 16.0);
    let r = l.route(&first_edge(&l)).unwrap();
    assert_eq!(r.quality, Quality::Clean);
}

#[test]
fn invalid_edges_have_no_route() {
    let (objs, _) = scene(40, 6);
    let mut l = EdgeLayer::new();
    // Источник — запрос (ключ 0), цель — результат (ключ 2): направление неверное.
    l.sync(&objs, &[EdgeGeom { ix: 0, from: 0, to: 2 }], 16.0);
    assert!(l.route(&first_edge(&l)).is_none());
    // Несуществующие ключи.
    l.sync(&objs, &[EdgeGeom { ix: 1, from: 77, to: 78 }], 16.0);
    assert!(l.route(&first_edge(&l)).is_none());
}

#[test]
fn cell_size_change_clears_cache() {
    let (objs, edges) = scene(40, 6);
    let mut l = EdgeLayer::new();
    l.sync(&objs, &edges, 16.0);
    let e = first_edge(&l);
    l.route(&e).unwrap();
    l.sync(&objs, &edges, 20.0);
    l.route(&e).unwrap();
    assert_eq!(l.route_calls, 2);
}

#[test]
fn obstacle_card_between_forces_detour_without_crossing() {
    // Между источником (0..32) и целью (80..) стоит карточка (40..72) на той же высоте.
    let mut objs = vec![process(0, 0, 0, 2, 2, 0), process(1, 80, 0, 2, 2, 10), process(2, 40, 0, 1, 1, 20)];
    objs[2].rect = Rect::new(40, 0, 32, 17);
    let edges = vec![EdgeGeom { ix: 0, from: 2, to: 10 }];
    let mut l = EdgeLayer::new();
    l.sync(&objs, &edges, 16.0);
    let r = l.route(&first_edge(&l)).unwrap();
    assert_eq!(r.quality, Quality::Clean);
    let wall = l.card(2).rect;
    for w in r.points.windows(2).skip(1).take(r.points.len().saturating_sub(3)) {
        let (a, b) = (w[0], w[1]);
        if a.y == b.y {
            let inside_y = wall.y * U < a.y && a.y < (wall.y + wall.h) * U;
            let overlap_x = a.x.min(b.x) < (wall.x + wall.w) * U && wall.x * U < a.x.max(b.x);
            assert!(!(inside_y && overlap_x));
        }
    }
}

#[test]
fn screen_route_is_orthogonal_at_any_zoom() {
    let (objs, edges) = scene(40, 6);
    let mut l = EdgeLayer::new();
    l.sync(&objs, &edges, 16.0);
    let e = first_edge(&l);
    let route = l.route(&e).unwrap();
    let (src_e, dst_e) = (*l.entry(e.from).unwrap(), *l.entry(e.to).unwrap());
    for zoom in [0.4f32, 0.75, 1.0, 1.7] {
        for shift in [Vec2::ZERO, Vec2::new(5.3, -7.1)] {
            let offset = (30.0f32, 12.0f32);
            let port = |entry: &super::layer::EntryRef, side: Side, drag: Vec2| {
                let card = l.card(entry.card);
                let origin = Pos2::new(
                    (card.rect.x as f32 * 16.0 + offset.0) * zoom,
                    (card.rect.y as f32 * 16.0 + offset.1) * zoom,
                );
                let (ox, oy) = l.port_offset_px(entry, side);
                origin + drag + Vec2::new(ox, oy) * zoom
            };
            let sp = port(&src_e, route.src_side, shift);
            let dp = port(&dst_e, route.dst_side, Vec2::ZERO);
            let pts = route_to_screen(&route.points, 16.0, offset, zoom, sp, dp);
            assert_eq!(pts[0], sp);
            assert_eq!(*pts.last().unwrap(), dp);
            for w in pts.windows(2) {
                assert!((w[0].x - w[1].x).abs() < 1e-3 || (w[0].y - w[1].y).abs() < 1e-3, "диагональ при zoom {zoom}");
            }
            let n = pts.len();
            assert!((pts[0].y - pts[1].y).abs() < 1e-3);
            assert!((pts[n - 1].y - pts[n - 2].y).abs() < 1e-3);
        }
    }
}

#[test]
fn edge_color_follows_statuses() {
    let t = Theme::dark();
    assert_eq!(edge_color(&t, StatusCode::Ok, StatusCode::Ok), t.status_ok);
    assert_eq!(edge_color(&t, StatusCode::Surplus, StatusCode::Ok), t.status_ok);
    assert_eq!(edge_color(&t, StatusCode::InProgress, StatusCode::Ok), t.status_deficit);
    assert_eq!(edge_color(&t, StatusCode::Expected, StatusCode::Ok), t.status_deficit);
    assert_eq!(edge_color(&t, StatusCode::Ok, StatusCode::Expected), t.status_deficit);
    assert_eq!(edge_color(&t, StatusCode::Ok, StatusCode::Surplus), t.status_ok);
}

#[test]
fn rounded_corner_is_tangent_arc() {
    let pts = [Pos2::new(0.0, 0.0), Pos2::new(100.0, 0.0), Pos2::new(100.0, 80.0)];
    assert_eq!(rounded_points(&pts, 0.0), pts.to_vec());
    let r = 10.0;
    let out = rounded_points(&pts, r);
    assert_eq!(out[0], pts[0]);
    assert_eq!(*out.last().unwrap(), pts[2]);
    // Дуга начинается на r до угла и заканчивается на r после него, центр — (90, 10).
    assert!((out[1] - Pos2::new(90.0, 0.0)).length() < 1e-3);
    assert!((out[out.len() - 2] - Pos2::new(100.0, 10.0)).length() < 1e-3);
    for p in &out[1..out.len() - 1] {
        assert!(((*p - Pos2::new(90.0, 10.0)).length() - r).abs() < 1e-3);
    }
    // Радиус ограничен половиной самого короткого отрезка (80 / 2 = 40 → 1000 урезается до 40).
    let big = rounded_points(&pts, 1000.0);
    assert!((big[1] - Pos2::new(60.0, 0.0)).length() < 1e-3);
}
