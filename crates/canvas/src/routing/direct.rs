//! Короткие случаи без поиска: прямая линия и «Z» из трёх отрезков по общей вертикали.
//!
//! Это идеальные маршруты по ТЗ: либо один горизонтальный отрезок, либо
//! горизонталь – вертикаль – горизонталь, когда конец первого и начало последнего
//! отрезка лежат на одной вертикали сетки.

use super::cost::{Cand, BEND, LEN, NONNATURAL, TIE_DIV};
use super::model::{Params, Side, Terminal, U};
use super::obstacles::{h_free, v_free, Obs, Own};
use super::stubs::columns;

/// Порты на одной высоте с точностью до этого (субклеток) считаются лежащими на одной прямой.
pub const SAME_Y: i32 = 2;

/// Лучший из «прямых» вариантов по всем четырём сочетаниям сторон портов.
pub fn best(src: &Terminal, dst: &Terminal, obs: &[Obs], par: Params) -> Option<Cand> {
    let (xmid, _) = mid(src, dst);
    let mut best: Option<Cand> = None;
    for s in Side::BOTH {
        for t in Side::BOTH {
            let (psx, psy) = src.port_u(s);
            let (ptx, pty) = dst.port_u(t);
            let nonnat = if s == Side::Right && t == Side::Left {
                0
            } else {
                NONNATURAL
            };
            let dy = (pty - psy).abs();

            if dy <= SAME_Y {
                // Одна высота: годится только прямая линия, и только лицом к лицу.
                let facing = (s == Side::Right && t == Side::Left && ptx > psx)
                    || (s == Side::Left && t == Side::Right && ptx < psx);
                let len = (ptx - psx).abs();
                if facing && len >= 2 * U && h_free(obs, &[Own::Src, Own::Dst], psy, psx, ptx) {
                    let cost = len as u64 * LEN + nonnat;
                    offer(
                        &mut best,
                        Cand::new(cost, vec![(psx, psy), (ptx, psy)], s, t),
                    );
                }
                continue;
            }

            let cs = columns(obs, Own::Src, src, s, psy, par, None);
            let ct = columns(obs, Own::Dst, dst, t, pty, par, None);
            let (Some(cs), Some(ct)) = (cs, ct) else {
                continue;
            };
            let (lo, hi) = (cs.0.max(ct.0), cs.1.min(ct.1));
            for v in lo..=hi {
                if !v_free(obs, v, psy, pty) {
                    continue;
                }
                let vx = v * U;
                let len = (vx - psx).abs() + dy + (vx - ptx).abs();
                let tie = ((vx - xmid).abs() / TIE_DIV) as u64 * (dy / U).max(1) as u64;
                let cost = len as u64 * LEN + 2 * BEND + nonnat + tie;
                let pts = vec![(psx, psy), (vx, psy), (vx, pty), (ptx, pty)];
                offer(&mut best, Cand::new(cost, pts, s, t));
            }
        }
    }
    best
}

fn offer(best: &mut Option<Cand>, c: Cand) {
    if best.as_ref().map_or(true, |b| c.cost < b.cost) {
        *best = Some(c);
    }
}

/// Середина между естественными портами (выход справа, вход слева): к ней тянутся
/// вертикали и горизонтали, когда выбор между равными путями свободен.
pub fn mid(src: &Terminal, dst: &Terminal) -> (i32, i32) {
    let (sx, sy) = src.port_u(Side::Right);
    let (tx, ty) = dst.port_u(Side::Left);
    ((sx + tx) / 2, (sy + ty) / 2)
}
