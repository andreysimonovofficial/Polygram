//! Проверки маршрутизатора линий по пунктам ТЗ (DoD) + сверка с независимым перебором.

use super::model::{Params, Port, Quality, Route, Side, Terminal, U};
use super::obstacles::{h_free, v_free, Obs, Own};
use super::router::{route, solve};
use polygram_core::layout::Rect;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

// ---------- сцена ----------

fn term(r: Rect, k: i32) -> Terminal {
    let y = r.y as f32 + 0.6 + 1.3 * k as f32;
    Terminal {
        rect: r,
        left: Port {
            x: r.x as f32 + 0.75,
            y,
        },
        right: Port {
            x: (r.x + r.w) as f32 - 0.75,
            y,
        },
    }
}

fn term_y(r: Rect, y: f32) -> Terminal {
    Terminal {
        rect: r,
        left: Port {
            x: r.x as f32 + 0.75,
            y,
        },
        right: Port {
            x: (r.x + r.w) as f32 - 0.75,
            y,
        },
    }
}

fn run(src: &Terminal, dst: &Terminal, cards: &[Rect]) -> Route {
    route(src, dst, cards)
}

// ---------- свойства маршрута ----------

fn segs(p: &[(i32, i32)]) -> Vec<((i32, i32), (i32, i32))> {
    p.windows(2).map(|w| (w[0], w[1])).collect()
}

fn len_of(p: &[(i32, i32)]) -> u64 {
    segs(p)
        .iter()
        .map(|(a, b)| ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as u64)
        .sum()
}

/// Пересекает ли отрезок открытую внутренность прямоугольника (в клетках, раздутого на `grow`).
fn hits(a: (i32, i32), b: (i32, i32), r: &Rect, grow: i32) -> bool {
    let (x0, y0) = ((r.x - grow) * U, (r.y - grow) * U);
    let (x1, y1) = ((r.x + r.w + grow) * U, (r.y + r.h + grow) * U);
    let (lx, hx) = (a.0.min(b.0), a.0.max(b.0));
    let (ly, hy) = (a.1.min(b.1), a.1.max(b.1));
    if a.1 == b.1 {
        y0 < a.1 && a.1 < y1 && x0 < hx && lx < x1
    } else {
        x0 < a.0 && a.0 < x1 && y0 < hy && ly < y1
    }
}

/// Пересекаются ли (в т.ч. касаются) два ортогональных отрезка.
fn touch(a: ((i32, i32), (i32, i32)), b: ((i32, i32), (i32, i32))) -> bool {
    let bx = |s: ((i32, i32), (i32, i32))| {
        (
            s.0 .0.min(s.1 .0),
            s.0 .0.max(s.1 .0),
            s.0 .1.min(s.1 .1),
            s.0 .1.max(s.1 .1),
        )
    };
    let (ax0, ax1, ay0, ay1) = bx(a);
    let (bx0, bx1, by0, by1) = bx(b);
    ax0 <= bx1 && bx0 <= ax1 && ay0 <= by1 && by0 <= ay1
}

/// Все требования ТЗ к одному маршруту.
pub(super) fn assert_dod(r: &Route, src: &Terminal, dst: &Terminal, cards: &[Rect]) {
    let p = &r.points;
    let n = p.len();
    assert!(
        n == 2 || n >= 4,
        "первый и последний отрезки горизонтальны: n={n}, {p:?}"
    );

    // Ортогональность, отсутствие нулевых отрезков и разворотов.
    for (a, b) in segs(p) {
        assert!(a.0 == b.0 || a.1 == b.1, "диагональ {a:?}->{b:?}");
        assert!(a != b, "нулевой отрезок");
    }
    for w in p.windows(3) {
        let (a, b, c) = (w[0], w[1], w[2]);
        let collinear = (a.0 == b.0 && b.0 == c.0) || (a.1 == b.1 && b.1 == c.1);
        assert!(!collinear, "лишняя точка на прямой: {a:?} {b:?} {c:?}");
    }

    // Первый/последний отрезки — горизонтали; направление определяют стороны портов.
    let (f0, f1) = (p[0], p[1]);
    let (l0, l1) = (p[n - 2], p[n - 1]);
    assert_eq!(f0.1, f1.1, "первый отрезок горизонтален");
    assert_eq!(l0.1, l1.1, "последний отрезок горизонтален");
    let sgn = |v: i32| v.signum();
    assert_eq!(
        sgn(f1.0 - f0.0),
        if r.src == Side::Right { 1 } else { -1 },
        "из правого порта — только вправо, из левого — влево"
    );
    assert_eq!(
        sgn(l1.0 - l0.0),
        if r.dst == Side::Left { 1 } else { -1 },
        "в левый порт — слева направо, в правый — справа налево"
    );

    // Начало и конец — именно точки сцепления.
    assert_eq!(f0, src.port_u(r.src));
    assert_eq!(l1, dst.port_u(r.dst));

    // Минимум две клетки у первого и последнего горизонтальных отрезков.
    if r.quality <= Quality::Tight {
        assert!(
            (f1.0 - f0.0).abs() >= 2 * U,
            "первый отрезок короче 2 клеток: {p:?}"
        );
        assert!(
            (l1.0 - l0.0).abs() >= 2 * U,
            "последний отрезок короче 2 клеток: {p:?}"
        );
    }

    if r.quality == Quality::Fallback {
        return;
    }
    // Вертикаль первого/последнего отрезка — линия сетки вне карточки.
    if n > 2 {
        assert_eq!(f1.0 % U, 0, "конец первого отрезка на вертикали сетки");
        assert_eq!(l0.0 % U, 0, "начало последнего отрезка на вертикали сетки");
        assert!(f1.0 / U < src.rect.x || f1.0 / U > src.rect.x + src.rect.w);
        assert!(l0.0 / U < dst.rect.x || l0.0 / U > dst.rect.x + dst.rect.w);
        // Промежуточные отрезки идут по линиям сетки.
        for &(x, y) in &p[1..n - 1] {
            let _ = (x, y);
        }
        for (a, b) in segs(&p[1..n - 1]) {
            if a.0 == b.0 {
                assert_eq!(a.0 % U, 0, "вертикаль вне сетки: {a:?}->{b:?}");
            }
        }
        for w in p[1..n - 1].windows(2) {
            if w[0].1 == w[1].1 && !(w[0] == p[1] || w[1] == p[n - 2]) {
                assert_eq!(w[0].1 % U, 0, "горизонталь вне сетки");
            }
        }
    }

    // Петель нет: несмежные отрезки не касаются друг друга.
    let s = segs(p);
    for i in 0..s.len() {
        for j in i + 2..s.len() {
            assert!(!touch(s[i], s[j]), "самопересечение/петля: {p:?}");
        }
    }

    // Линия не заходит в карточки (с охранной зоной при качестве Clear).
    let grow = if r.quality == Quality::Clear { 1 } else { 0 };
    for (k, &(a, b)) in s.iter().enumerate() {
        for c in cards {
            let own_first = k == 0 && *c == src.rect;
            let own_last = k == s.len() - 1 && *c == dst.rect;
            if own_first || own_last {
                // Свой край: только внутри самой карточки и её охранной зоны.
                continue;
            }
            assert!(
                !hits(a, b, c, grow),
                "отрезок {a:?}->{b:?} задевает карточку {c:?} (grow={grow})"
            );
        }
    }
}

// ---------- золотые сценарии ----------

#[test]
fn three_segments_when_gap_is_four() {
    let a = Rect::new(0, 0, 30, 30);
    let b = Rect::new(34, 10, 30, 30);
    let (s, d) = (term_y(a, 6.4), term_y(b, 17.8));
    let r = run(&s, &d, &[a, b]);
    assert_dod(&r, &s, &d, &[a, b]);
    assert_eq!(r.quality, Quality::Clear);
    assert_eq!((r.src, r.dst), (Side::Right, Side::Left));
    assert_eq!(r.points.len(), 4, "Z из трёх отрезков: {:?}", r.points);
    // Обе вертикали совпадают: 2 клетки от каждой карточки.
    assert_eq!(r.points[1].0, 32 * U);
    assert_eq!(r.points[2].0, 32 * U);
}

#[test]
fn vertical_goes_to_the_middle_of_a_wide_gap() {
    let a = Rect::new(0, 0, 30, 30);
    let b = Rect::new(50, 10, 30, 30);
    let (s, d) = (term_y(a, 6.4), term_y(b, 17.8));
    let r = run(&s, &d, &[a, b]);
    assert_dod(&r, &s, &d, &[a, b]);
    assert_eq!(r.points.len(), 4);
    assert_eq!(r.points[1].0, 40 * U, "вертикаль посередине зазора");
}

#[test]
fn straight_line_when_ports_are_on_the_same_height() {
    let a = Rect::new(0, 0, 30, 30);
    let b = Rect::new(40, 0, 30, 30);
    let (s, d) = (term_y(a, 12.5), term_y(b, 12.5));
    let r = run(&s, &d, &[a, b]);
    assert_dod(&r, &s, &d, &[a, b]);
    assert_eq!(r.points.len(), 2, "одна прямая: {:?}", r.points);
}

#[test]
fn wall_between_is_bypassed_by_shortest_way() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(40, 0, 10, 10);
    let wall = Rect::new(20, -4, 6, 18);
    let cards = [a, b, wall];
    let (s, d) = (term_y(a, 4.5), term_y(b, 4.5));
    let r = run(&s, &d, &cards);
    assert_dod(&r, &s, &d, &cards);
    assert_eq!(r.quality, Quality::Clear);
    // Кратчайший обход: вверх или вниз за охранную зону стены — ровно 2 лишних подъёма.
    let manhattan = ((d.port_u(Side::Left).0 - s.port_u(Side::Right).0).abs()) as u64;
    let up = (4.5 * U as f32) as i64 - ((-4 - 1) * U) as i64;
    let down = ((14 + 1) * U) as i64 - (4.5 * U as f32) as i64;
    let detour = 2 * up.min(down) as u64;
    assert_eq!(
        len_of(&r.points),
        manhattan + detour,
        "длина обхода: {:?}",
        r.points
    );
    // Без излишних колен: вверх, вдоль стены, вниз — 4 колена (5 отрезков).
    assert_eq!(r.points.len(), 6, "{:?}", r.points);
}

#[test]
fn backward_edge_goes_around_both_cards() {
    let a = Rect::new(60, 0, 20, 20);
    let b = Rect::new(0, 4, 20, 20);
    let (s, d) = (term(a, 0), term(b, 1));
    let r = run(&s, &d, &[a, b]);
    assert_dod(&r, &s, &d, &[a, b]);
    // Ближайшие стороны: выход источника влево, вход приёмника справа — путь напрямую между карточками.
    assert_eq!((r.src, r.dst), (Side::Left, Side::Right));
}

#[test]
fn narrow_gap_shortens_stubs_but_still_routes() {
    // Соседи в одной клетке от карточки: отрезок в 2 клетки не помещается, но линия есть.
    let a = Rect::new(20, 20, 10, 10);
    let b = Rect::new(60, 20, 10, 10);
    let left = Rect::new(9, 10, 10, 30);
    let right = Rect::new(31, 10, 10, 30);
    let cards = [a, b, left, right];
    let (s, d) = (term(a, 0), term(b, 0));
    let r = run(&s, &d, &cards);
    assert_eq!(r.quality, Quality::Short);
    assert_dod(&r, &s, &d, &cards);
}

#[test]
fn boxed_in_card_still_gets_a_line() {
    // Соседи вплотную: выйти из карточки некуда — рисуется простая ортогональная ломаная.
    let a = Rect::new(20, 20, 10, 10);
    let b = Rect::new(60, 20, 10, 10);
    let left = Rect::new(10, 10, 10, 30);
    let right = Rect::new(30, 10, 10, 30);
    let cards = [a, b, left, right];
    let (s, d) = (term(a, 0), term(b, 0));
    let r = run(&s, &d, &cards);
    assert_eq!(r.quality, Quality::Fallback);
    assert!(r
        .points
        .windows(2)
        .all(|w| w[0].0 == w[1].0 || w[0].1 == w[1].1));
}

#[test]
fn result_is_deterministic() {
    let a = Rect::new(0, 0, 30, 30);
    let b = Rect::new(33, 40, 30, 30);
    let w = Rect::new(10, 31, 12, 8);
    let (s, d) = (term(a, 1), term(b, 2));
    assert_eq!(run(&s, &d, &[a, b, w]), run(&s, &d, &[a, b, w]));
}

// ---------- случайные сцены ----------

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: i32) -> i32 {
        (self.next() % n as u64) as i32
    }
}

fn scene(rng: &mut Rng) -> Vec<Rect> {
    let want = 3 + rng.below(7) as usize;
    let mut cards: Vec<Rect> = Vec::new();
    for _ in 0..200 {
        if cards.len() == want {
            break;
        }
        let r = Rect::new(
            rng.below(90),
            rng.below(60),
            6 + rng.below(9),
            6 + rng.below(9),
        );
        // Между карточками минимум клетка, как в ядре.
        if cards
            .iter()
            .all(|c| !Rect::new(r.x - 1, r.y - 1, r.w + 2, r.h + 2).overlaps(c))
        {
            cards.push(r);
        }
    }
    cards
}

#[test]
fn random_scenes_satisfy_dod() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut qualities = [0usize; 4];
    for _ in 0..400 {
        let cards = scene(&mut rng);
        if cards.len() < 2 {
            continue;
        }
        let i = rng.below(cards.len() as i32) as usize;
        let mut j = rng.below(cards.len() as i32) as usize;
        if i == j {
            j = (j + 1) % cards.len();
        }
        let (s, d) = (term(cards[i], rng.below(4)), term(cards[j], rng.below(4)));
        let r = run(&s, &d, &cards);
        qualities[r.quality as usize] += 1;
        assert_dod(&r, &s, &d, &cards);
    }
    assert!(
        qualities[0] > 200,
        "большинство линий идут с воздухом: {qualities:?}"
    );
}

// ---------- независимый перебор для сверки длины и колен ----------

/// Простой Дейкстра по (вершина, направление) без окна, растра и эвристики.
/// Возвращает (длина, колена) лучшего пути по правилам ТЗ без «прямых» сокращений.
fn oracle(src: &Terminal, dst: &Terminal, cards: &[Rect], par: Params) -> Option<(u64, u64)> {
    let obs: Vec<Obs> = cards
        .iter()
        .map(|r| {
            Obs::new(
                r,
                par.clear,
                if *r == src.rect {
                    Own::Src
                } else if *r == dst.rect {
                    Own::Dst
                } else {
                    Own::Other
                },
            )
        })
        .collect();
    let (bx0, by0, bx1, by1) = (-40, -40, 160, 140);
    let hb = |i: i32, j: i32| {
        obs.iter()
            .any(|o| o.y0 < j && j < o.y1 && o.x0 < i + 1 && i < o.x1)
    };
    let vb = |i: i32, j: i32| {
        obs.iter()
            .any(|o| o.x0 < i && i < o.x1 && o.y0 < j + 1 && j < o.y1)
    };
    let dxy = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let w = (bx1 - bx0 + 1) as usize;
    let h = (by1 - by0 + 1) as usize;
    let idx = |i: i32, j: i32, d: usize| (((j - by0) as usize * w + (i - bx0) as usize) * 4) + d;
    let mut dist = vec![(u64::MAX, u64::MAX); w * h * 4];
    let mut heap = BinaryHeap::new();

    for s in Side::BOTH {
        let (px, py) = src.port_u(s);
        let (edge, dir) = if s == Side::Right {
            (src.rect.x + src.rect.w, 1)
        } else {
            (src.rect.x, -1)
        };
        let out_d = if s == Side::Right { 0 } else { 1 };
        let mut v = edge + dir * par.min_stub;
        // горизонталь свободна от края до v
        while h_free(&obs, &[Own::Src], py, edge * U, v * U) && (bx0 + 1..bx1).contains(&v) {
            let first = ((v * U - px).abs()) as u64;
            if py % U == 0 {
                let c = (first, 0);
                let st = idx(v, py / U, out_d);
                if c < dist[st] {
                    dist[st] = c;
                    heap.push(Reverse((c, v, py / U, out_d)));
                }
            } else {
                let j0 = py.div_euclid(U);
                for (j, d) in [(j0, 3usize), (j0 + 1, 2usize)] {
                    if !v_free(&obs, v, py, j * U) {
                        continue;
                    }
                    let c = (first + (j * U - py).unsigned_abs() as u64, 1);
                    let st = idx(v, j, d);
                    if c < dist[st] {
                        dist[st] = c;
                        heap.push(Reverse((c, v, j, d)));
                    }
                }
            }
            v += dir;
        }
    }

    let mut goals: Vec<(i32, i32, i32, i32, i32, usize)> = Vec::new(); // v, j, ptx, pty, stub, last_dir
    for t in Side::BOTH {
        let (px, py) = dst.port_u(t);
        let (edge, dir) = if t == Side::Right {
            (dst.rect.x + dst.rect.w, 1)
        } else {
            (dst.rect.x, -1)
        };
        let last_dir = if t == Side::Left { 0 } else { 1 };
        let mut v = edge + dir * par.min_stub;
        while h_free(&obs, &[Own::Dst], py, edge * U, v * U) && (bx0 + 1..bx1).contains(&v) {
            let js: Vec<i32> = if py % U == 0 {
                vec![py / U]
            } else {
                vec![py.div_euclid(U), py.div_euclid(U) + 1]
            };
            for j in js {
                if v_free(&obs, v, j * U, py) {
                    goals.push((v, j, px, py, py - j * U, last_dir));
                }
            }
            v += dir;
        }
    }

    let mut best: Option<(u64, u64)> = None;
    while let Some(Reverse(((len, bends), i, j, d))) = heap.pop() {
        if (len, bends) > dist[idx(i, j, d)] {
            continue;
        }
        if best.map_or(false, |b| (len, bends) >= b) {
            continue;
        }
        for g in goals.iter().filter(|g| g.0 == i && g.1 == j) {
            let last = (i * U - g.2).unsigned_abs() as u64;
            let cand = if g.4 == 0 {
                if d == g.5 {
                    Some((last, 0))
                } else if d == (g.5 ^ 1) {
                    None
                } else {
                    Some((last, 1))
                }
            } else {
                let sd = if g.4 > 0 { 2 } else { 3 };
                let turn = if d == sd {
                    Some(0)
                } else if d == (sd ^ 1) {
                    None
                } else {
                    Some(1)
                };
                turn.map(|t| (g.4.unsigned_abs() as u64 + last, t + 1))
            };
            if let Some((l2, b2)) = cand {
                let total = (len + l2, bends + b2);
                if best.map_or(true, |b| total < b) {
                    best = Some(total);
                }
            }
        }
        for nd in 0..4 {
            if nd == (d ^ 1) {
                continue;
            }
            let (dx, dy) = dxy[nd];
            let (ni, nj) = (i + dx, j + dy);
            if ni <= bx0 || ni >= bx1 || nj <= by0 || nj >= by1 {
                continue;
            }
            let blocked = match nd {
                0 => hb(i, j),
                1 => hb(ni, j),
                2 => vb(i, j),
                _ => vb(i, nj),
            };
            if blocked {
                continue;
            }
            let c = (len + U as u64, bends + u64::from(nd != d));
            let st = idx(ni, nj, nd);
            if c < dist[st] {
                dist[st] = c;
                heap.push(Reverse((c, ni, nj, nd)));
            }
        }
    }
    best
}

#[test]
fn router_is_never_worse_than_brute_force() {
    let mut rng = Rng(0xDEADBEEFCAFEF00D);
    let par = Params {
        clear: 1,
        min_stub: 2,
    };
    let (mut compared, mut shortcuts) = (0, 0);
    for _ in 0..500 {
        let cards = scene(&mut rng);
        if cards.len() < 2 {
            continue;
        }
        let i = rng.below(cards.len() as i32) as usize;
        let mut j = rng.below(cards.len() as i32) as usize;
        if i == j {
            j = (j + 1) % cards.len();
        }
        let (s, d) = (term(cards[i], rng.below(4)), term(cards[j], rng.below(4)));
        let Some(c) = solve(&s, &d, &cards, par) else {
            assert!(
                oracle(&s, &d, &cards, par).is_none(),
                "роутер не нашёл путь, а перебор нашёл"
            );
            continue;
        };
        let got = (len_of(&c.points), (c.points.len() as u64) - 2);
        let want = oracle(&s, &d, &cards, par).expect("перебор не нашёл путь, а роутер нашёл");
        compared += 1;
        assert!(
            got <= want,
            "роутер хуже перебора: {got:?} > {want:?}, сцена {cards:?}, {i}->{j}"
        );
        if got < want {
            // Лучше перебора может быть только «прямой» вариант без обхода (Z или прямая).
            shortcuts += 1;
            assert!(
                c.points.len() <= 4,
                "выигрыш у перебора только за счёт прямых вариантов: {:?}",
                c.points
            );
        }
    }
    assert!(
        compared > 350,
        "сверено сцен: {compared} (прямых выигрышей: {shortcuts})"
    );
}
