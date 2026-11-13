//! Резервные копии и атомарная запись файлов.
//!
//! Единственная ответственность — ротация `.bak`-копий и атомарная запись
//! контента через временный файл + переименование, чтобы прерывание не оставило битый JSON.

use std::path::{Path, PathBuf};

/// Количество резервных копий, которые хранятся рядом с основным файлом.
pub const MAX_BACKUPS: usize = 2;

/// Путь к резервной копии порядкового номера `n` (1 — самая свежая).
pub fn backup_path(path: &Path, n: usize) -> PathBuf {
    let mut p = path.to_path_buf();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("pg");
    p.set_extension(format!("{}.bak{}", ext, n));
    p
}

/// Перед перезаписью основного файла ротировать существующие резервные копии:
/// .bak2 <- .bak1, .bak1 <- текущий основной файл.
fn rotate_backups(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    // Удаляем самую старую копию.
    let oldest = backup_path(path, MAX_BACKUPS);
    let _ = std::fs::remove_file(&oldest);
    // Сдвигаем копии на один шаг назад.
    for i in (1..MAX_BACKUPS).rev() {
        let newer = backup_path(path, i);
        let older = backup_path(path, i + 1);
        if newer.exists() {
            let _ = std::fs::remove_file(&older);
            std::fs::rename(&newer, &older)?;
        }
    }
    // Копируем текущий основной файл в .bak1.
    let first = backup_path(path, 1);
    let _ = std::fs::remove_file(&first);
    std::fs::copy(path, &first)?;
    Ok(())
}

/// Атомарная запись с ротацией резервных копий.
/// Пишет во временный файл, затем переименовывает (атомарно на одной ФС),
/// так что прерывание записи не оставит битый JSON.
pub(crate) fn atomic_save_with_backup(path: &Path, content: &str) -> std::io::Result<()> {
    let _ = rotate_backups(path);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
