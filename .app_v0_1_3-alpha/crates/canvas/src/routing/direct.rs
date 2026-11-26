//! Прямые варианты без поиска: прямая линия и «Z».

use super::cost::{BEND_W, LEN_W, NONNATURAL, TIE_DIV};
use super::model::{Pt, Side, Terminal, U};
use super::obstacles::{h_free, v_free, Obstacle, Own};
use super::stubs::columns;

/// Допуск «порты на одной высоте» (субклетки).
pub const SAME_Y: i32 = 2;

/// Кандидат маршрута: стороны, точки, упакованная стоимость и длина.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Сторона источника.
    pub src_side: Side,
    /// Сторона цели.
    pub dst_side: Side,
    /// Точки маршрута (субклетки).
    pub points: Vec<Pt>,
    /// Упакованная стоимость.
    pub cost: u64,
    /// Длина (субклетки).
    pub len: i64,
}

/// Середина между карточками (субклетки) по естественной паре портов:
/// целое деление с отсечением к нулю.
pub fn mid(src: &Terminal, dst: &Terminal) -> (i32, i32) {
    let (a, b) = (src.right, dst.left);
    ((a.x + b.x) / 2, (a.y + b.y) / 2)
}

/// Лучший прямой вариант среди четырёх пар сторон. При равной цене побеждает найденный первым.
pub fn best(obs: &[Obstacle], src: &Terminal, dst: &Terminal, min_stub: i32) -> Option<Candidate> {
    let (xmid, _) = mid(src, dst);
    let mut best: Option<Candidate> = None;
    for s in Side::BOTH {
        for t in Side::BOTH {
            let (ps, pt) = (src.port(s), dst.port(t));
            let dy = (pt.y - ps.y).abs();
            let nonnat = if s == Side::Right && t == Side::Left { 0 } else { NONNATURAL };
            if dy <= SAME_Y {
                let facing = (s == Side::Right && t == Side::Left && pt.x > ps.x)
                    || (s == Side::Left && t == Side::Right && pt.x < ps.x);
                if !facing {
                    continue;
                }
                let len = (pt.x - ps.x).abs();
                if len < 2 * U || !h_free(obs, ps.y, ps.x, pt.x, &[Own::Src, Own::Dst]) {
                    continue;
                }
                let cost = len as u64 * LEN_W + nonnat;
                consider(&mut best, Candidate { src_side: s, dst_side: t, points: vec![ps, pt], cost, len: len as i64 });
            } else {
                let Some(cs) = columns(obs, Own::Src, src.edge_x(s), s, ps.y, min_stub, None) else { continue };
                let Some(ct) = columns(obs, Own::Dst, dst.edge_x(t), t, pt.y, min_stub, None) else { continue };
                let (lo, hi) = (cs.0.max(ct.0), cs.1.min(ct.1));
                for v in lo..=hi {
                    if !v_free(obs, v, ps.y, pt.y) {
                        continue;
                    }
                    let vx = v * U;
                    let len = (vx - ps.x).abs() + dy + (vx - pt.x).abs();
                    let beauty = ((vx - xmid).abs() / TIE_DIV) as u64 * ((dy / U).max(1) as u64);
                    let cost = len as u64 * LEN_W + 2 * BEND_W + nonnat + beauty;
                    consider(
                        &mut best,
                        Candidate {
                            src_side: s,
                            dst_side: t,
                            points: vec![ps, Pt::new(vx, ps.y), Pt::new(vx, pt.y), pt],
                            cost,
                            len: len as i64,
                        },
                    );
                }
            }
        }
    }
    best
}

fn consider(best: &mut Option<Candidate>, c: Candidate) {
    if best.as_ref().map_or(true, |b| c.cost < b.cost) {
        *best = Some(c);
    }
}
