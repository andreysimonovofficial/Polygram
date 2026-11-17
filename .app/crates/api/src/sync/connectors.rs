//! Структуры коннекторов, файловая привязка и пути.
//!
//! Единственная ответственность — работа с файлами коннекторов (`connectors/{id}.json`)
//! и путями к каталогу коннекторов.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Имя каталога коннекторов.
pub const CONNECTORS_DIR_NAME: &str = "connectors";

/// Описание коннектора из файла connectors/{id}.json.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConnectorFile {
    pub id: String,
    pub name: String,
    pub placeholder: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub cache: serde_json::Value,
}

/// Конфигурация коннектора (хранится в файле connectors/{id}.json).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub id: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub cache: serde_json::Value,
}

/// Краткая информация о коннекторе для списка.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ConnectorInfo {
    pub id: String,
    pub name: String,
    pub placeholder: String,
}

/// Каталог коннекторов рядом с исполняемым файлом.
pub fn connectors_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join(CONNECTORS_DIR_NAME)
}

/// Путь к файлу привязок bindings.json внутри каталога коннекторов.
pub fn bindings_path(dir: &PathBuf) -> PathBuf {
    dir.join("bindings.json")
}

/// Создать дефолтные коннекторы, если каталог пуст.
pub fn ensure_default_connectors(dir: &PathBuf) -> std::io::Result<()> {
    let count = fs::read_dir(dir)?.count();
    if count > 0 {
        return Ok(());
    }
    let defaults = vec![
        ConnectorFile {
            id: "google-sheets".to_owned(),
            name: "Google Таблицы".to_owned(),
            placeholder: "https://docs.google.com/spreadsheets/d/... или Sheet1!A1".to_owned(),
            config: serde_json::json!({"url": "", "intervalSecs": 30}),
            cache: serde_json::json!({}),
        },
        ConnectorFile {
            id: "todoist".to_owned(),
            name: "Todoist".to_owned(),
            placeholder: "todoist:task:ID:due_date".to_owned(),
            config: serde_json::json!({"intervalSecs": 60}),
            cache: serde_json::json!({}),
        },
        ConnectorFile {
            id: "static".to_owned(),
            name: "Константа (тест)".to_owned(),
            placeholder: "42".to_owned(),
            config: serde_json::json!({"intervalSecs": 30}),
            cache: serde_json::json!({}),
        },
    ];
    for cf in &defaults {
        let path = dir.join(format!("{}.json", cf.id));
        let _ = fs::write(&path, serde_json::to_string_pretty(cf).unwrap_or_default());
    }
    Ok(())
}

/// Загрузить привязки из файла bindings.json.
pub fn load_bindings(dir: &PathBuf) -> Vec<crate::sync::binding::ApiBinding> {
    let path = bindings_path(dir);
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<Vec<crate::sync::binding::ApiBinding>>(&content)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Сохранить привязки в файл bindings.json.
pub fn save_bindings(dir: &PathBuf, bindings: &[crate::sync::binding::ApiBinding]) -> std::io::Result<()> {
    let path = bindings_path(dir);
    let json = serde_json::to_string_pretty(bindings)?;
    crate::atomic_save_with_backup(&path, &json)?;
    Ok(())
}
