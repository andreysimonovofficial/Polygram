//! Слой линий: кэш маршрутов и ленивый пересчёт.
//!
//! Маршрутизатор — чистая функция; всё, что связано с хранением результатов, живёт здесь.
//! Слой не хранится в документе и не передаётся наружу.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use polygram_core::geometry::{EdgeGeom, ObjectGeom};
use polygram_core::layout::Rect;
use polygram_core::types::{Direction, NodeKind};

use super::model::{round_away, union_rect, Pt, Route, Side, Terminal, U};
use super::{ports, router};

/// Карточка Процесса, участвующая в маршрутизации.
#[derive(Clone, Debug)]
pub struct CardInfo {
    /// Идентификатор объекта.
    pub id: String,
    /// Занимаемый прямоугольник в клетках (округлён наружу по реальному размеру карточки).
    pub rect: Rect,
    /// Число строк в контейнере «Запрос».
    pub n_req: usize,
    /// Число строк в контейнере «Результат».
    pub n_res: usize,
}

/// Положение показателя: карточка, контейнер, номер строки.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryRef {
    /// Индекс карточки в списке слоя.
    pub card: usize,
    /// Контейнер.
    pub dir: Direction,
    /// Номер строки с нуля.
    pub row: usize,
}

/// Связь между двумя показателями.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeRef {
    /// Ключ связи.
    pub ix: u32,
    /// Ключ показателя-источника (строка «Результат»).
    pub from: u32,
    /// Ключ показателя-цели (строка «Запрос»).
    pub to: u32,
}

struct Cached {
    route: Route,
    key: u64,
    checked: u64,
}

/// Кэш маршрутов и описание карточек/показателей для текущей геометрии.
#[derive(Default)]
pub struct EdgeLayer {
    fingerprint: Option<u64>,
    cell_size: f32,
    generation: u64,
    cards: Vec<CardInfo>,
    entries: HashMap<u32, EntryRef>,
    edges: Vec<EdgeRef>,
    routes: HashMap<u32, Cached>,
    /// Сколько раз запускался маршрутизатор (для тестов и отладки).
    pub route_calls: u64,
}

fn kind_tag(k: NodeKind) -> u8 {
    match k {
        NodeKind::Process => 0,
        NodeKind::Department => 1,
        NodeKind::Container => 2,
        NodeKind::Metric => 3,
        #[allow(unreachable_patterns)]
        _ => 255,
    }
}

fn hash_rect(h: &mut DefaultHasher, r: &Rect) {
    r.x.hash(h);
    r.y.hash(h);
    r.w.hash(h);
    r.h.hash(h);
}

fn hash_pt(h: &mut DefaultHasher, p: &Pt) {
    p.hash(h);
}

impl EdgeLayer {
    /// Создать пустой слой.
    pub fn new() -> Self {
        Self::default()
    }

    /// Рёбра текущего снимка.
    pub fn edges(&self) -> &[EdgeRef] {
        &self.edges
    }

    /// Карточка по индексу.
    pub fn card(&self, i: usize) -> &CardInfo {
        &self.cards[i]
    }

    /// Положение показателя по ключу.
    pub fn entry(&self, ix: u32) -> Option<&EntryRef> {
        self.entries.get(&ix)
    }

    /// Поколение геометрии (растёт при любом изменении карточек и связей).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Смещение порта показателя от левого верхнего угла карточки (пиксели, масштаб 1).
    pub fn port_offset_px(&self, e: &EntryRef, side: Side) -> (f32, f32) {
        ports::row_port_px(self.cards[e.card].n_req, e.dir, e.row, side)
    }

    /// Рамка обеих карточек связи (клетки); нужна для отсечения невидимых линий.
    pub fn pair_frame(&self, edge: &EdgeRef) -> Option<Rect> {
        let a = self.entries.get(&edge.from)?;
        let b = self.entries.get(&edge.to)?;
        Some(union_rect(&self.cards[a.card].rect, &self.cards[b.card].rect))
    }

    /// Синхронизация со снимком геометрии. Вызывается перед отрисовкой линий.
    /// Если снимок не изменился — ничего не делает.
    pub fn sync(&mut self, objects: &[ObjectGeom], edges: &[EdgeGeom], cell_size: f32) {
        let fp = Self::fingerprint_of(objects, edges, cell_size);
        if self.fingerprint == Some(fp) {
            return;
        }
        if self.cell_size.to_bits() != cell_size.to_bits() {
            // Порты считаются из пикселей, поэтому смена размера клетки меняет всё.
            self.routes.clear();
        }
        self.cell_size = cell_size;
        self.fingerprint = Some(fp);
        self.generation += 1;

        self.cards.clear();
        self.entries.clear();
        for o in objects.iter().filter(|o| o.kind == NodeKind::Process) {
            let (w_px, h_px) = ports::card_size_px(o.requests.len(), o.results.len());
            // Округление наружу: линии не должны заходить на дробную часть карточки.
            let w = (w_px / cell_size).ceil() as i32;
            let h = (h_px / cell_size).ceil() as i32;
            let card = self.cards.len();
            self.cards.push(CardInfo {
                id: o.id.clone(),
                rect: Rect::new(o.rect.x, o.rect.y, w, h),
                n_req: o.requests.len(),
                n_res: o.results.len(),
            });
            for (row, e) in o.requests.iter().enumerate() {
                self.entries.insert(e.ix, EntryRef { card, dir: Direction::Request, row });
            }
            for (row, e) in o.results.iter().enumerate() {
                self.entries.insert(e.ix, EntryRef { card, dir: Direction::Result, row });
            }
        }
        self.edges = edges.iter().map(|e| EdgeRef { ix: e.ix, from: e.from, to: e.to }).collect();
        let alive: std::collections::HashSet<u32> = self.edges.iter().map(|e| e.ix).collect();
        self.routes.retain(|k, _| alive.contains(k));
    }

    fn fingerprint_of(objects: &[ObjectGeom], edges: &[EdgeGeom], cell_size: f32) -> u64 {
        let mut h = DefaultHasher::new();
        cell_size.to_bits().hash(&mut h);
        for o in objects {
            o.ix.hash(&mut h);
            kind_tag(o.kind).hash(&mut h);
            hash_rect(&mut h, &o.rect);
            o.requests.len().hash(&mut h);
            for e in &o.requests {
                e.ix.hash(&mut h);
            }
            u8::MAX.hash(&mut h);
            for e in &o.results {
                e.ix.hash(&mut h);
            }
            u8::MAX.hash(&mut h);
        }
        for e in edges {
            (e.ix, e.from, e.to).hash(&mut h);
        }
        h.finish()
    }

    /// Терминал для показателя: прямоугольник карточки и оба порта строки (субклетки).
    pub fn terminal(&self, e: &EntryRef) -> Terminal {
        let card = &self.cards[e.card];
        let sub = |px: f32| round_away(px * U as f32 / self.cell_size);
        let port = |side: Side| {
            let (ox, oy) = ports::row_port_px(card.n_req, e.dir, e.row, side);
            Pt::new(card.rect.x * U + sub(ox), card.rect.y * U + sub(oy))
        };
        Terminal { card: e.card as u32, rect: card.rect, left: port(Side::Left), right: port(Side::Right) }
    }

    /// Отпечаток маршрута: оба терминала и все карточки, строго пересекающие область влияния.
    fn route_key(&self, src: &Terminal, dst: &Terminal, region: &Rect) -> u64 {
        let mut h = DefaultHasher::new();
        for t in [src, dst] {
            t.card.hash(&mut h);
            hash_rect(&mut h, &t.rect);
            hash_pt(&mut h, &t.left);
            hash_pt(&mut h, &t.right);
        }
        for (i, c) in self.cards.iter().enumerate() {
            if region.overlaps(&c.rect) {
                i.hash(&mut h);
                hash_rect(&mut h, &c.rect);
            }
        }
        h.finish()
    }

    /// Маршрут связи. `None`, если связь не пригодна для линии (ключ источника не в «Результате»
    /// или ключ цели не в «Запросе»). Пересчитывается лениво и только если изменились
    /// терминалы или карточки в области влияния.
    pub fn route(&mut self, edge: &EdgeRef) -> Option<Route> {
        let src_e = *self.entries.get(&edge.from)?;
        let dst_e = *self.entries.get(&edge.to)?;
        if src_e.dir != Direction::Result || dst_e.dir != Direction::Request {
            return None;
        }
        if let Some(c) = self.routes.get(&edge.ix) {
            if c.checked == self.generation {
                return Some(c.route.clone());
            }
        }
        let src = self.terminal(&src_e);
        let dst = self.terminal(&dst_e);
        if let Some((region, key)) = self.routes.get(&edge.ix).map(|c| (c.route.region, c.key)) {
            if self.route_key(&src, &dst, &region) == key {
                let gen = self.generation;
                let c = self.routes.get_mut(&edge.ix)?;
                c.checked = gen;
                return Some(c.route.clone());
            }
        }
        let cards: Vec<(u32, Rect)> =
            self.cards.iter().enumerate().map(|(i, c)| (i as u32, c.rect)).collect();
        self.route_calls += 1;
        let route = router::route(&cards, &src, &dst);
        let key = self.route_key(&src, &dst, &route.region);
        self.routes.insert(edge.ix, Cached { route: route.clone(), key, checked: self.generation });
        Some(route)
    }
}
