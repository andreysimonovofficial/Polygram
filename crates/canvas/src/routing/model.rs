//! Типы маршрутизатора линий связи. Без egui: только целые числа и клетки сетки.
//!
//! Все координаты маршрутизатора — в «субклетках»: 1 клетка = [`U`] единиц. Так порты
//! (они лежат внутри строк карточки, а не на линиях сетки) и линии сетки живут в одной
//! целочисленной системе, и длины путей сравниваются точно, без дробей.

use polygram_core::layout::Rect;

/// Субединиц в одной клетке сетки.
pub const U: i32 = 16;

/// Сторона карточки, на которой лежит точка сцепления.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::Right, Side::Left];
}

/// Точка сцепления строки показателя, в клетках (не зависит от зума камеры).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Port {
    pub x: f32,
    pub y: f32,
}

/// Один конец связи: карточка и точки сцепления нужной строки слева и справа.
#[derive(Clone, Copy, Debug)]
pub struct Terminal {
    pub rect: Rect,
    pub left: Port,
    pub right: Port,
}

impl Terminal {
    pub fn port(&self, side: Side) -> Port {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }

    /// Порт в субклетках.
    pub(crate) fn port_u(&self, side: Side) -> (i32, i32) {
        let p = self.port(side);
        (
            (p.x * U as f32).round() as i32,
            (p.y * U as f32).round() as i32,
        )
    }

    /// Вертикаль карточки, от которой отсчитывается отступ наружу.
    pub(crate) fn edge_x(&self, side: Side) -> i32 {
        match side {
            Side::Left => self.rect.x,
            Side::Right => self.rect.x + self.rect.w,
        }
    }
}

/// Насколько удалось соблюсти «воздух» вокруг карточек. Чем выше, тем хуже.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    /// Линия держится минимум в одной клетке от чужих карточек.
    Clear,
    /// Пришлось идти вплотную к карточке (по её границе).
    Tight,
    /// Пришлось сократить первый/последний отрезок до одной клетки.
    Short,
    /// Пути нет: нарисована простая ломаная без обхода препятствий.
    Fallback,
}

/// Готовый маршрут. Точки — в субклетках; первая и последняя — порты.
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub src: Side,
    pub dst: Side,
    pub points: Vec<(i32, i32)>,
    pub quality: Quality,
    /// Область (клетки: x0, y0, x1, y1), за пределами которой изменения карточек маршрут
    /// не меняют: любой путь не длиннее найденного целиком лежит внутри неё.
    pub region: (i32, i32, i32, i32),
}

/// Параметры одного прохода поиска.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Охранная зона вокруг карточек (клеток): внутрь неё промежуточные линии не заходят.
    pub clear: i32,
    /// Минимальная длина первого и последнего отрезка от края карточки (клеток).
    pub min_stub: i32,
}

/// Убирает повторы и лишние промежуточные точки на одной прямой.
/// Разворот на 180° на одной прямой тоже схлопывается.
pub fn simplify(points: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut out: Vec<(i32, i32)> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() == Some(&p) {
            continue;
        }
        while out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            let collinear = (a.0 == b.0 && b.0 == p.0) || (a.1 == b.1 && b.1 == p.1);
            if collinear {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}
