//! Генератор файла стресс-теста `examples/nested_1000.pg` (этап 1 — загрузка).
//! 1000 отделов без родителей, на разных клетках (чтобы загрузка не пала на наложении).
//! dep999 стоит в (0,0) — после цепочки SetObjectParent все отделы схлопнутся к нему,
//! образовав 1000-уровневую вложенность у начала координат.
//! Запуск генератора: cargo run -p polygram --example gen_nested

use polygram_core::{
    document::{Document, DocEdge, DocObject, DocMetricDef},
    types::NodeKind,
};
use std::fs;

fn main() {
    let mut objects: Vec<DocObject> = Vec::new();
    let pitch: i32 = 40;

    for i in 0..1000 {
        let id = format!("dep{:03}", i);
        // dep999 — лист в начале координат, остальные — с шагом 40 (не накладываются).
        let at = if i == 999 {
            [0, 0]
        } else {
            [(i + 1) as i32 * pitch, 0]
        };
        objects.push(DocObject {
            id,
            name: format!("Отдел {:03}", i),
            kind: NodeKind::Department,
            at,
            parent: None,
            requests: Vec::new(),
            outputs: Vec::new(),
        });
    }

    let metric_defs: Vec<DocMetricDef> = vec![];
    let edges: Vec<DocEdge> = vec![];

    let doc = Document {
        format: polygram_core::document::FORMAT.to_owned(),
        name: "Сtresstest: 1000 отделов (цепь вложенности строим командами)".to_owned(),
        metric_defs,
        objects,
        edges,
    };

    let out = "examples/nested_1000.pg";
    fs::write(out, doc.to_json()).expect("write");
    println!("Wrote {}", out);
    println!("Objects: {}", doc.objects.len());
    println!("Edges: {}", doc.edges.len());
    println!("MetricDefs: {}", doc.metric_defs.len());
}
