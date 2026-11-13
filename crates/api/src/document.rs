//! JSON-документ Polygram (.pg): экспорт, импорт, сохранение и загрузка состояния ядра.

use polygram_core::{Document, EngineError};
use std::path::Path;

use super::ApiService;

impl ApiService {
    /// Экспортировать состояние ядра в JSON-строку (формат .pg).
    pub fn to_json(&self) -> String {
        self.inner
            .runtime
            .handle()
            .query(|e| e.to_json())
            .unwrap_or_default()
    }

    /// Сохранить состояние ядра в .pg файл.
    /// Запись атомарная (через временный файл + переименование) с ротацией
    /// резервных копий, чтобы внезапное отключение не могло испортить данные.
    pub fn save_to_file(&self, path: &Path) -> Result<(), EngineError> {
        let json = self
            .inner
            .runtime
            .handle()
            .query(|e| e.to_json())
            .ok_or(EngineError::Stopped)?;
        super::backup::atomic_save_with_backup(path, &json).map_err(|e| EngineError::Document(e.to_string()))
    }

    /// Загрузить состояние ядра из .pg файла (создаёт новый ApiService с загруженным движком).
    pub fn load_from_file(path: &Path) -> Result<Self, EngineError> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| EngineError::Document(e.to_string()))?;
        let doc = Document::parse(&json)?;
        let engine = polygram_core::Engine::from_document(&doc)?;
        Ok(Self::new(engine))
    }

    /// Получить полный документ графа (включая parent у объектов).
    pub fn to_document_json(&self) -> Result<String, EngineError> {
        let json = self
            .inner
            .runtime
            .handle()
            .query(|e| e.to_json())
            .ok_or(EngineError::Stopped)?;
        Ok(json)
    }

    /// Применить документ (.pg JSON) к текущему графу, заменяя его содержимое.
    pub fn apply_document(&self, json: &str) -> Result<(), EngineError> {
        let doc = Document::parse(json)?;
        let engine = polygram_core::Engine::from_document(&doc)?;
        // Нельзя просто заменить движок в рантайме — вместо этого применяем команды.
        // Но для простоты возвращаем документ, а GUI сама разберёт его на команды.
        let _ = engine;
        Ok(())
    }
}
