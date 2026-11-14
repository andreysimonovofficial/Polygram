//! Слой линий связи холста: кэш маршрутов и быстрый доступ к концам связей.
//!
//! Линия — визуальный элемент: ядро её не хранит и наружу (API, MCP) не отдаёт. Холст строит
//! её по алгоритму из положения карточек и кэширует: маршрут пересчитывается, только если
//! изменились его концы или карточки в его области влияния. Считаются лишь видимые линии.

use super::model::{Route, Side, Terminal};
use super::ports::port_in_cells;
use super::router;
use polygram_core::geometry::{EdgeGeom, Geometry, ObjectGeom};
use polygram_core::layout::Rect;
use polygram_core::types::NodeKind;
use polygram_core::EntryIx;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Где в геометрии лежит показатель: объект, номер строки, это выход?
#[derive(Clone, Copy)]
struct EntryPos {
    object: usize,
    row: usize,
    output: bool,
}

/// Концы связи в геометрии.
pub struct Ends<'a> {
    pub from: &'a ObjectGeom,
    pub from_row: usize,
    pub to: &'a ObjectGeom,
    pub to_row: usize,
}

/// Маршрут в кэше и то, при каком положении карточек он считался.
struct Cached {
    route: Route,
    /// Отпечаток концов связи и карточек в области влияния маршрута.
    key: u64,
    /// Поколение геометрии, на которое маршрут уже проверен.
    checked: u64,
}

#[derive(Default)]
pub struct EdgeLayer {
    fingerprint: Option<u64>,
    cell_size: f32,
    /// Номер поколения геометрии: растёт, когда карточки или связи изменились.
    generation: u64,
    entries: HashMap<u32, EntryPos>,
    /// Карточки, которые линии обходят (отделы-рамки — не препятствие).
    cards: Vec<Rect>,
    routes: HashMap<u32, Cached>,
    /// Сколько раз запускался маршрутизатор (для тестов кэша).
    #[cfg(test)]
    pub(crate) computed: usize,
}

impl EdgeLayer {
    /// Свериться с геометрией. Если карточки или связи изменились — маршруты не сбрасываются
    /// разом: каждый проверяется при обращении и пересчитывается, только если изменилось
    /// что-то в его области влияния. Так при перетаскивании карточки считаются лишь её линии.
    pub fn sync(&mut self, geometry: &Geometry, cell_size: f32) {
        let fp = fingerprint(geometry, cell_size);
        if self.fingerprint == Some(fp) {
            return;
        }
        if self.cell_size != cell_size {
            self.routes.clear();
        }
        self.fingerprint = Some(fp);
        self.cell_size = cell_size;
        self.generation += 1;
        self.entries.clear();
        self.cards.clear();
        for (oi, o) in geometry.objects.iter().enumerate() {
            for (row, e) in o.outputs.iter().enumerate() {
                self.entries.insert(
                    e.ix,
                    EntryPos {
                        object: oi,
                        row,
                        output: true,
                    },
                );
            }
            for (row, e) in o.requests.iter().enumerate() {
                self.entries.insert(
                    e.ix,
                    EntryPos {
                        object: oi,
                        row,
                        output: false,
                    },
                );
            }
            if o.kind != NodeKind::Department {
                self.cards.push(o.rect);
            }
        }
        let alive: HashSet<u32> = geometry.edges.iter().map(|e| e.ix).collect();
        self.routes.retain(|ix, _| alive.contains(ix));
    }

    /// Объекты и строки на концах связи.
    pub fn ends<'a>(&self, geometry: &'a Geometry, edge: &EdgeGeom) -> Option<Ends<'a>> {
        let f = self.entries.get(&edge.from).filter(|p| p.output)?;
        let t = self.entries.get(&edge.to).filter(|p| !p.output)?;
        Some(Ends {
            from: geometry.objects.get(f.object)?,
            from_row: f.row,
            to: geometry.objects.get(t.object)?,
            to_row: t.row,
        })
    }

    /// Маршрут связи: из кэша, если в его области влияния ничего не менялось, иначе новый.
    pub fn route(&mut self, geometry: &Geometry, edge: &EdgeGeom) -> Option<&Route> {
        if self
            .routes
            .get(&edge.ix)
            .map_or(false, |c| c.checked == self.generation)
        {
            return self.routes.get(&edge.ix).map(|c| &c.route);
        }
        let ends = self.ends(geometry, edge)?;
        let src = terminal(ends.from, EntryIx(edge.from), true, self.cell_size)?;
        let dst = terminal(ends.to, EntryIx(edge.to), false, self.cell_size)?;

        if let Some(c) = self.routes.get(&edge.ix) {
            if self.key(&src, &dst, c.route.region) == c.key {
                let generation = self.generation;
                let c = self.routes.get_mut(&edge.ix)?;
                c.checked = generation;
                return Some(&c.route);
            }
        }
        let route = router::route(&src, &dst, &self.cards);
        #[cfg(test)]
        {
            self.computed += 1;
        }
        let key = self.key(&src, &dst, route.region);
        let cached = Cached {
            route,
            key,
            checked: self.generation,
        };
        self.routes.insert(edge.ix, cached);
        self.routes.get(&edge.ix).map(|c| &c.route)
    }

    /// Отпечаток концов связи и всех карточек, задевающих область `region`.
    fn key(&self, src: &Terminal, dst: &Terminal, region: (i32, i32, i32, i32)) -> u64 {
        let mut h = DefaultHasher::new();
        for t in [src, dst] {
            (t.rect.x, t.rect.y, t.rect.w, t.rect.h).hash(&mut h);
            for p in [t.left, t.right] {
                (p.x.to_bits(), p.y.to_bits()).hash(&mut h);
            }
        }
        let (x0, y0, x1, y1) = region;
        for c in &self.cards {
            if c.x < x1 && c.x + c.w > x0 && c.y < y1 && c.y + c.h > y0 {
                (c.x, c.y, c.w, c.h).hash(&mut h);
            }
        }
        h.finish()
    }
}

pub(crate) fn terminal(
    o: &ObjectGeom,
    entry: EntryIx,
    is_output: bool,
    cell_size: f32,
) -> Option<Terminal> {
    let entries = if is_output { &o.outputs } else { &o.requests };
    let port = |side: Side| {
        port_in_cells(
            &o.rect,
            entries,
            entry,
            is_output,
            side == Side::Right,
            cell_size,
        )
    };
    Some(Terminal {
        rect: o.rect,
        left: port(Side::Left)?,
        right: port(Side::Right)?,
    })
}

/// Отпечаток всего, от чего зависят маршруты: положение карточек, строки показателей, связи.
fn fingerprint(geometry: &Geometry, cell_size: f32) -> u64 {
    let mut h = DefaultHasher::new();
    cell_size.to_bits().hash(&mut h);
    for o in &geometry.objects {
        (o.rect.x, o.rect.y, o.rect.w, o.rect.h).hash(&mut h);
        (o.kind == NodeKind::Department).hash(&mut h);
        for e in &o.requests {
            e.ix.hash(&mut h);
        }
        0xA5u8.hash(&mut h);
        for e in &o.outputs {
            e.ix.hash(&mut h);
        }
    }
    for e in &geometry.edges {
        (e.ix, e.from, e.to).hash(&mut h);
    }
    h.finish()
}
