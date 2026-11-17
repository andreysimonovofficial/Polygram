//! Хранилище пользовательских данных Polygram.
//!
//! Все файлы проекта (автосохранение, бэкапы, недавние файлы) хранятся в
//! системной директории приложения:
//! - Windows: %LOCALAPPDATA%\Polygram\
//! - Linux/macOS: ~/.config/polygram/

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Корневая директория данных приложения.
pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("Polygram"))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("polygram_data")
            })
    } else {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".config").join("polygram"))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("polygram_data")
            })
    }
}

/// Путь к файлу автосохранения.
pub fn auto_save_path() -> PathBuf {
    data_dir().join("auto-save.pg")
}

/// Путь к файлу списка недавних проектов.
pub fn recent_path() -> PathBuf {
    data_dir().join("recent.json")
}

/// Запись о недавнем проекте.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    pub last_opened: String,
}

/// Создать директорию данных, если не существует.
pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Атомарная запись с ротацией резервных копий.
/// Пишет во временный файл, затем переименовывает (атомарно на одной ФС),
/// так что прерывание записи не оставит битый JSON.
pub fn save_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    super::backup::atomic_save_with_backup(path, content)
}

/// Загрузить последний действительный файл из списка кандидатов
/// (основной, затем бэкапы от самого свежего к самому старому).
pub fn load_latest_valid(candidates: &[PathBuf]) -> Option<String> {
    for path in candidates {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                // Проверяем, что это валидный JSON (문서 .pg).
                if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                    return Some(content);
                }
            }
        }
    }
    None
}

/// Загрузить список недавних файлов.
pub fn load_recent_files() -> Vec<RecentFile> {
    let path = recent_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Сохранить список недавних файлов.
pub fn save_recent_files(recent: &[RecentFile]) -> std::io::Result<()> {
    ensure_data_dir()?;
    let json = serde_json::to_string_pretty(recent).unwrap_or_default();
    let _ = save_atomic(&recent_path(), &json);
    Ok(())
}

/// Добавить файл в список недавних (максимум 10 записей).
pub fn add_recent_file(recent: &mut Vec<RecentFile>, path: &Path, name: &str) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let path_str = path.to_string_lossy().to_string();

    // Удаляем дубликат, если есть.
    recent.retain(|r| r.path != path_str);

    // Добавляем в начало.
    recent.insert(
        0,
        RecentFile {
            path: path_str,
            name: name.to_owned(),
            last_opened: now,
        },
    );

    // Оставляем максимум 10.
    recent.truncate(10);

    let _ = save_recent_files(recent);
}
