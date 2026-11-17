//! Автоматические «стены» архитектуры: проверяются при каждом запуске тестов.
use std::{fs, path::Path};

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                rust_files(&p, out);
            } else if p.extension().map_or(false, |x| x == "rs") {
                out.push(p);
            }
        }
    }
}

/// Мега-компоненты запрещены: ни один исходный файл не длиннее 1000 строк.
#[test]
fn no_mega_files() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    rust_files(&crates, &mut files);
    assert!(!files.is_empty());
    for f in files {
        let n = fs::read_to_string(&f).unwrap().lines().count();
        assert!(
            n <= 1000,
            "{} длиннее 1000 строк ({n}): разрежь на комнаты",
            f.display()
        );
    }
}

/// Ядро не знает об окне, графике и сети.
#[test]
fn core_stays_headless() {
    let manifest =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let deps = manifest.split("[dependencies]").nth(1).unwrap_or("");
    for banned in [
        "egui", "eframe", "wgpu", "winit", "tokio", "axum", "hyper", "reqwest",
    ] {
        assert!(
            !deps.contains(banned),
            "ядро не должно зависеть от «{banned}»"
        );
    }
}
