//! Единицы, стороны, терминалы и маршрут.

use polygram_core::layout::Rect;

/// Субклеток в клетке. Весь расчёт маршрута идёт в субклетках целыми числами.
pub const U: i32 = 16;

/// Точка в субклетках.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pt {
    /// X в субклетках.
    pub x: i32,
    /// Y в субклетках.
    pub y: i32,
}

impl Pt {
    /// Создать точку.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Сторона карточки, из которой линия выходит или в которую входит.
/// Порядок обхода важен для детерминизма: Правая раньше Левой.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Правая сторона.
    Right,
    /// Левая сторона.
    Left,
}

impl Side {
    /// Обе стороны в порядке обхода.
    pub const BOTH: [Side; 2] = [Side::Right, Side::Left];

    /// Знак «наружу» от карточки: +1 вправо, −1 влево.
    pub fn sign(self) -> i32 {
        match self {
            Side::Right => 1,
            Side::Left => -1,
        }
    }
}

/// Качество маршрута. Чем дальше по списку, тем хуже.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    /// Охранная зона 1 клетка, заглушка не короче 2 клеток.
    Clean,
    /// Охранная зона 0, заглушка не короче 2 клеток.
    Tight,
    /// Охранная зона 0, заглушка не короче 1 клетки.
    Short,
    /// Запасная ломаная без поиска; может пересекать карточки.
    Fallback,
}

/// Конец связи: прямоугольник карточки (клетки), идентификатор и два порта (субклетки).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Terminal {
    /// Идентификатор карточки (по нему определяется «своя» карточка).
    pub card: u32,
    /// Прямоугольник карточки в клетках.
    pub rect: Rect,
    /// Левый порт строки (субклетки).
    pub left: Pt,
    /// Правый порт строки (субклетки).
    pub right: Pt,
}

impl Terminal {
    /// Порт нужной стороны.
    pub fn port(&self, side: Side) -> Pt {
        match side {
            Side::Right => self.right,
            Side::Left => self.left,
        }
    }

    /// Вертикаль карточки на нужной стороне (клетки).
    pub fn edge_x(&self, side: Side) -> i32 {
        match side {
            Side::Right => self.rect.x + self.rect.w,
            Side::Left => self.rect.x,
        }
    }
}

/// Готовый маршрут.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Сторона источника.
    pub src_side: Side,
    /// Сторона цели.
    pub dst_side: Side,
    /// Точки в субклетках: первая — порт источника, последняя — порт цели.
    pub points: Vec<Pt>,
    /// Качество.
    pub quality: Quality,
    /// Область влияния (клетки): изменения карточек вне неё маршрут не меняют.
    pub region: Rect,
}

impl Route {
    /// Длина ломаной в субклетках.
    pub fn length(&self) -> i64 {
        self.points
            .windows(2)
            .map(|w| ((w[1].x - w[0].x).abs() + (w[1].y - w[0].y).abs()) as i64)
            .sum()
    }

    /// Число колен.
    pub fn bends(&self) -> usize {
        self.points.len().saturating_sub(2)
    }
}

/// Округление «от нуля» (половины уходят от нуля, в том числе для отрицательных).
pub fn round_away(v: f32) -> i32 {
    v.round() as i32
}

/// Убрать повторы подряд и средние точки из трёх на одной прямой
/// (включая разворот на 180° по той же прямой).
pub fn simplify(pts: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last() == Some(&p) {
            continue;
        }
        out.push(p);
        while out.len() >= 3 {
            let n = out.len();
            let (a, b, c) = (out[n - 3], out[n - 2], out[n - 1]);
            let same_x = a.x == b.x && b.x == c.x;
            let same_y = a.y == b.y && b.y == c.y;
            if same_x || same_y {
                out.remove(n - 2);
            } else {
                break;
            }
        }
    }
    out
}

/// Рамка клеток, охватывающая два прямоугольника (в клетках).
pub fn union_rect(a: &Rect, b: &Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

/// Манхэттенское расстояние между двумя точками (субклетки).
pub fn manhattan(a: Pt, b: Pt) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
