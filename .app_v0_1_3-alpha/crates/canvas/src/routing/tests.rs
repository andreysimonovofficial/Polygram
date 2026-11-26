//! Тесты маршрутизатора: золотые сценарии, случайные сцены, сверка со сжатой сеткой.

use polygram_core::layout::Rect;

use super::cost::{cost_bends, cost_len};
use super::model::*;
use super::obstacles::{self, Own};
use super::router::{route, TIERS};
use super::search::search;

/// Терминал с портами в 0,75 клетки от краёв карточки на высоте `y` (клетки).
fn term_y(card: u32, r: Rect, y: f32) -> Terminal {
    Terminal {
        card,
        rect: r,
        left: Pt::new(round_away((r.x as f32 + 0.75) * 16.0), round_away(y * 16.0)),
        right: Pt::new(round_away(((r.x + r.w) as f32 - 0.75) * 16.0), round_away(y * 16.0)),
    }
}

/// Терминал строки `k`: высота = y карточки + 0,6 + 1,3 × k.
fn term_k(card: u32, r: Rect, k: u32) -> Terminal {
    term_y(card, r, r.y as f32 + 0.6 + 1.3 * k as f32)
}

fn cards_of(rects: &[Rect]) -> Vec<(u32, Rect)> {
    rects.iter().enumerate().map(|(i, r)| (i as u32, *r)).collect()
}

fn seg_hits_open_rect(a: Pt, b: Pt, o: &obstacles::Obstacle) -> bool {
    let (x0, y0, x1, y1) = (o.x0 * U, o.y0 * U, o.x1 * U, o.y1 * U);
    if a.y == b.y {
        y0 < a.y && a.y < y1 && a.x.min(b.x) < x1 && x0 < a.x.max(b.x)
    } else {
        x0 < a.x && a.x < x1 && a.y.min(b.y) < y1 && y0 < a.y.max(b.y)
    }
}

fn segs_touch(a0: Pt, a1: Pt, b0: Pt, b1: Pt) -> bool {
    let (ax0, ax1, ay0, ay1) = (a0.x.min(a1.x), a0.x.max(a1.x), a0.y.min(a1.y), a0.y.max(a1.y));
    let (bx0, bx1, by0, by1) = (b0.x.min(b1.x), b0.x.max(b1.x), b0.y.min(b1.y), b0.y.max(b1.y));
    ax0 <= bx1 && bx0 <= ax1 && ay0 <= by1 && by0 <= ay1
}

/// Проверка маршрута по разделу 11.1 ТЗ.
fn check_route(r: &Route, src: &Terminal, dst: &Terminal, cards: &[(u32, Rect)]) {
    let p = &r.points;
    let n = p.len();
    assert!(n == 2 || n >= 4, "точек {n}");
    assert_eq!(p[0], src.port(r.src_side));
    assert_eq!(p[n - 1], dst.port(r.dst_side));
    for w in p.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y, "диагональ {:?}", w);
        assert!(w[0] != w[1], "нулевой отрезок");
    }
    for w in p.windows(3) {
        let same = (w[0].x == w[1].x && w[1].x == w[2].x) || (w[0].y == w[1].y && w[1].y == w[2].y);
        assert!(!same, "лишняя точка {:?}", w);
    }
    // Первый и последний отрезки горизонтальны, направление по сторонам.
    assert_eq!(p[0].y, p[1].y);
    assert_eq!(p[n - 1].y, p[n - 2].y);
    assert_eq!((p[1].x - p[0].x).signum(), r.src_side.sign());
    assert_eq!((p[n - 1].x - p[n - 2].x).signum(), -r.dst_side.sign());
    // Нет самокасаний между несмежными отрезками.
    for i in 0..n - 1 {
        for j in i + 2..n - 1 {
            assert!(!segs_touch(p[i], p[i + 1], p[j], p[j + 1]), "самокасание {i}/{j}");
        }
    }
    if r.quality == Quality::Fallback {
        return;
    }
    let (clear, stub) = match r.quality {
        Quality::Clean => (1, 2),
        Quality::Tight => (0, 2),
        _ => (0, 1),
    };
    // Заглушки не короче минимума, вертикали на целых колонках вне своих карточек.
    if n == 2 {
        assert!((p[1].x - p[0].x).abs() >= 2 * U);
    } else {
        assert!((p[1].x - p[0].x).abs() >= stub * U);
        assert!((p[n - 1].x - p[n - 2].x).abs() >= stub * U);
        assert_eq!(p[1].x % U, 0);
        assert_eq!(p[n - 2].x % U, 0);
        let c1 = p[1].x / U;
        assert!(c1 < src.rect.x || c1 > src.rect.x + src.rect.w);
        let c2 = p[n - 2].x / U;
        assert!(c2 < dst.rect.x || c2 > dst.rect.x + dst.rect.w);
        for i in 1..n - 2 {
            if p[i].y == p[i + 1].y {
                assert_eq!(p[i].y % U, 0, "горизонталь не на ряду");
            } else {
                assert_eq!(p[i].x % U, 0, "вертикаль не на колонке");
            }
        }
    }
    // Препятствия.
    let obs = obstacles::build(cards, src, dst, clear);
    for i in 0..n - 1 {
        let first_or_last = i == 0 || i == n - 2;
        for o in &obs {
            if first_or_last && matches!(o.own, Own::Src | Own::Dst) {
                continue;
            }
            assert!(!seg_hits_open_rect(p[i], p[i + 1], o), "отрезок {i} задевает карточку {:?}", o);
        }
    }
}

fn len_manhattan_plus(src: &Terminal, dst: &Terminal) -> i64 {
    manhattan(src.right, dst.left) as i64
}

#[test]
fn three_segments_gap_4() {
    let (a, b) = (Rect::new(0, 0, 30, 30), Rect::new(34, 10, 30, 30));
    let (s, d) = (term_y(0, a, 6.4), term_y(1, b, 17.8));
    let cards = cards_of(&[a, b]);
    let r = route(&cards, &s, &d);
    check_route(&r, &s, &d, &cards);
    assert_eq!(r.quality, Quality::Clean);
    assert_eq!((r.src_side, r.dst_side), (Side::Right, Side::Left));
    assert_eq!(r.points.len(), 4);
    assert_eq!(r.points[1].x, 32 * U);
    assert_eq!(r.points[2].x, 32 * U);
}

#[test]
fn vertical_in_middle_of_wide_gap() {
    let (a, b) = (Rect::new(0, 0, 30, 30), Rect::new(50, 10, 30, 30));
    let (s, d) = (term_y(0, a, 6.4), term_y(1, b, 17.8));
    let r = route(&cards_of(&[a, b]), &s, &d);
    assert_eq!(r.points.len(), 4);
    assert_eq!(r.points[1].x, 40 * U);
}

#[test]
fn straight_line() {
    let (a, b) = (Rect::new(0, 0, 30, 30), Rect::new(40, 0, 30, 30));
    let (s, d) = (term_y(0, a, 12.5), term_y(1, b, 12.5));
    let r = route(&cards_of(&[a, b]), &s, &d);
    assert_eq!(r.points.len(), 2);
    assert_eq!(r.quality, Quality::Clean);
}

#[test]
fn detour_around_wall() {
    let (a, b, w) = (Rect::new(0, 0, 10, 10), Rect::new(40, 0, 10, 10), Rect::new(20, -4, 6, 18));
    let (s, d) = (term_y(0, a, 4.5), term_y(1, b, 4.5));
    let cards = cards_of(&[a, b, w]);
    let r = route(&cards, &s, &d);
    check_route(&r, &s, &d, &cards);
    assert_eq!(r.quality, Quality::Clean);
    assert_eq!(r.points.len(), 6);
    // Манхэттен между портами + удвоенный меньший подъём (вверх 152 субклетки до зоны над стеной).
    assert_eq!(r.length(), len_manhattan_plus(&s, &d) + 2 * 152);
}

#[test]
fn backward_link_uses_outer_sides() {
    let (a, b) = (Rect::new(60, 0, 20, 20), Rect::new(0, 4, 20, 20));
    let (s, d) = (term_k(0, a, 0), term_k(1, b, 1));
    let cards = cards_of(&[a, b]);
    let r = route(&cards, &s, &d);
    check_route(&r, &s, &d, &cards);
    assert_eq!((r.src_side, r.dst_side), (Side::Left, Side::Right));
}

#[test]
fn narrow_gap_gives_short_quality() {
    let (a, b) = (Rect::new(20, 20, 10, 10), Rect::new(60, 20, 10, 10));
    let (l, rr) = (Rect::new(9, 10, 10, 30), Rect::new(31, 10, 10, 30));
    let (s, d) = (term_k(0, a, 0), term_k(1, b, 0));
    let cards = cards_of(&[a, b, l, rr]);
    let r = route(&cards, &s, &d);
    check_route(&r, &s, &d, &cards);
    assert_eq!(r.quality, Quality::Short);
}

#[test]
fn boxed_in_falls_back() {
    let (a, b) = (Rect::new(20, 20, 10, 10), Rect::new(60, 20, 10, 10));
    let (l, rr) = (Rect::new(10, 10, 10, 30), Rect::new(30, 10, 10, 30));
    let (s, d) = (term_k(0, a, 0), term_k(1, b, 0));
    let cards = cards_of(&[a, b, l, rr]);
    let r = route(&cards, &s, &d);
    assert_eq!(r.quality, Quality::Fallback);
    check_route(&r, &s, &d, &cards);
}

#[test]
fn deterministic() {
    let (a, b, w) = (Rect::new(0, 0, 30, 30), Rect::new(33, 40, 30, 30), Rect::new(10, 31, 12, 8));
    let (s, d) = (term_k(0, a, 1), term_k(1, b, 2));
    let cards = cards_of(&[a, b, w]);
    assert_eq!(route(&cards, &s, &d), route(&cards, &s, &d));
}

// ---------- Случайные сцены ----------

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u32) as i32
    }
}

fn random_scene(rng: &mut Lcg) -> (Vec<(u32, Rect)>, Terminal, Terminal) {
    scene(rng, (140, 100), (3, 7), (10, 34), (8, 30))
}

/// Плотная сцена: много карточек на малой площади, чтобы чаще требовались обходы.
fn crowded_scene(rng: &mut Lcg) -> (Vec<(u32, Rect)>, Terminal, Terminal) {
    scene(rng, (90, 60), (7, 11), (6, 20), (6, 16))
}

fn scene(
    rng: &mut Lcg,
    area: (i32, i32),
    count: (i32, i32),
    w: (i32, i32),
    h: (i32, i32),
) -> (Vec<(u32, Rect)>, Terminal, Terminal) {
    loop {
        let n = rng.range(count.0, count.1) as usize;
        let mut rects: Vec<Rect> = Vec::new();
        let mut tries = 0;
        while rects.len() < n && tries < 400 {
            tries += 1;
            let r = Rect::new(rng.range(0, area.0), rng.range(0, area.1), rng.range(w.0, w.1), rng.range(h.0, h.1));
            if rects.iter().all(|o| !o.overlaps(&r)) {
                rects.push(r);
            }
        }
        if rects.len() < 2 {
            continue;
        }
        let s = term_k(0, rects[0], rng.range(0, 3) as u32);
        let d = term_k(1, rects[1], rng.range(0, 3) as u32);
        return (cards_of(&rects), s, d);
    }
}

#[test]
fn random_scenes_satisfy_requirements() {
    let mut rng = Lcg(42);
    let (mut clean, mut total) = (0, 0);
    for _ in 0..400 {
        let (cards, s, d) = random_scene(&mut rng);
        let r = route(&cards, &s, &d);
        check_route(&r, &s, &d, &cards);
        total += 1;
        if r.quality == Quality::Clean {
            clean += 1;
        }
    }
    assert!(clean * 2 >= total, "Чисто только {clean} из {total}");
}

/// Сжатая сетка должна давать ту же длину и число колен, что полная сетка из всех целых линий.
#[test]
fn compressed_grid_matches_dense_grid() {
    let mut rng = Lcg(7);
    for _ in 0..120 {
        let (cards, s, d) = random_scene(&mut rng);
        for (clear, stub, _) in TIERS.iter().take(2) {
            let obs = obstacles::build(&cards, &s, &d, *clear);
            let sparse = search(&obs, &s, &d, *stub, 12, false);
            let dense = search(&obs, &s, &d, *stub, 12, true);
            match (sparse, dense) {
                (None, None) => {}
                (Some(a), Some(b)) => {
                    assert_eq!(cost_len(a.cost), cost_len(b.cost), "длина");
                    assert_eq!(cost_bends(a.cost), cost_bends(b.cost), "колена");
                }
                (a, b) => panic!("расхождение существования пути: {:?} / {:?}", a.is_some(), b.is_some()),
            }
        }
    }
}

/// Плотные сцены: требования ТЗ выполняются, а сжатая сетка совпадает с полной.
#[test]
fn crowded_scenes_requirements_and_dense_match() {
    let mut rng = Lcg(2026);
    let mut by_quality = [0usize; 4];
    let mut searched = 0usize;
    for _ in 0..300 {
        let (cards, s, d) = crowded_scene(&mut rng);
        let r = route(&cards, &s, &d);
        check_route(&r, &s, &d, &cards);
        by_quality[r.quality as usize] += 1;
        for (clear, stub, _) in TIERS.iter().take(2) {
            let obs = obstacles::build(&cards, &s, &d, *clear);
            let sparse = search(&obs, &s, &d, *stub, 12, false);
            let dense = search(&obs, &s, &d, *stub, 12, true);
            match (sparse, dense) {
                (None, None) => {}
                (Some(a), Some(b)) => {
                    searched += 1;
                    assert_eq!(cost_len(a.cost), cost_len(b.cost), "длина");
                    assert_eq!(cost_bends(a.cost), cost_bends(b.cost), "колена");
                }
                (a, b) => panic!("расхождение существования пути: {:?} / {:?}", a.is_some(), b.is_some()),
            }
        }
    }
    println!("качество Чисто/Впритык/Коротко/Запасной = {:?}, сверок = {}", by_quality, searched);
    assert!(by_quality[0] > 0 && searched > 100);
}
