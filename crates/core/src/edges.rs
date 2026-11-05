use crate::types::*;

/// Данные ребра (связи) в интрузивном связном списке.
#[derive(Clone, Copy, Debug)]
pub struct EdgeData {
    pub from: EntryIx,
    pub to: EntryIx,
    pub next_from: u32,
    pub next_to: u32,
    pub throughput: f64,
    pub alive: bool,
}

/// Итератор по связям записи (входящие или исходящие).
pub struct EdgeIter<'a> {
    pub(crate) h: &'a crate::hierarchy::Hierarchy,
    pub(crate) cur: u32,
    pub(crate) by_from: bool,
}

impl<'a> Iterator for EdgeIter<'a> {
    type Item = EdgeIx;
    #[inline]
    fn next(&mut self) -> Option<EdgeIx> {
        if self.cur == NONE {
            return None;
        }
        let e = EdgeIx(self.cur);
        let d = &self.h.edges[self.cur as usize];
        self.cur = if self.by_from { d.next_from } else { d.next_to };
        Some(e)
    }
}
