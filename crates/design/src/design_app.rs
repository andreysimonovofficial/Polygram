//! Figma-подобный инструмент дизайна: свободный холст.
//! Рисуем прямоугольники, линии, эллипсы, текст, перетаскиваем элементы,
//! настраиваем свойства и экспортируем JSON, который Агент отрисовывает в точности.

use crate::design_format::*;
use crate::free_canvas::FreeCanvas;
use crate::screen_agent::ScreenAgent;
use crate::ui_agent::{AnalysisResult, LlmProvider, UiAgent};
use eframe::Frame;
use egui::Context;
use rfd::FileDialog;
use std::path::PathBuf;

/// Состояние инструмента дизайна (только свободный холст).
pub struct DesignApp {
    pub free_canvas: FreeCanvas,
    pub file_path: Option<PathBuf>,
    pub status: String,
    /// Агент анализа UI-скриншота.
    pub ui_agent: UiAgent,
    /// Путь к последнему загруженному скриншоту.
    pub screenshot_path: Option<PathBuf>,
}

impl DesignApp {
    pub fn new() -> Self {
        Self {
            free_canvas: FreeCanvas::new(),
            file_path: None,
            status: "Готов".to_owned(),
            ui_agent: UiAgent::new(LlmProvider::default()),
            screenshot_path: None,
        }
    }

    /// Отобразить весь интерфейс инструмента дизайна.
    pub fn ui(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Новый").clicked() {
                    self.free_canvas.doc = FreeCanvasDoc::default();
                    self.free_canvas.selected.clear();
                    self.file_path = None;
                    self.status = "Новый холст создан".to_owned();
                }
                if ui.button("Открыть").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        match load_free_canvas(&path) {
                            Ok(doc) => {
                                self.free_canvas.doc = doc;
                                self.free_canvas.selected.clear();
                                self.file_path = Some(path);
                                self.status = format!("Открыто: {:?}", self.file_path);
                            }
                            Err(e) => self.status = format!("Ошибка открытия: {e}"),
                        }
                    }
                }
                if ui.button("Сохранить").clicked() {
                    if let Some(path) = &self.file_path {
                        if let Err(e) = save_to_free_canvas(&self.free_canvas.doc, path) {
                            self.status = format!("Ошибка сохранения: {e}");
                        } else {
                            self.status = format!("Сохранено в {:?}", path);
                        }
                    } else {
                        self.save_as();
                    }
                }
                if ui.button("Сохранить как…").clicked() {
                    self.save_as();
                }
                ui.separator();
                ui.label(format!("Статус: {}", self.status));
                ui.separator();

                // Провайдер LLM
                ui.group(|ui| {
                    ui.label("Агент LLM");
                    match &mut self.ui_agent.provider {
                        LlmProvider::Gemini { api_key } => {
                            ui.horizontal(|ui| {
                                ui.label("Gemini API ключ:");
                                ui.text_edit_singleline(api_key);
                            });
                        }
                        LlmProvider::Qwen { endpoint, api_key } => {
                            ui.horizontal(|ui| {
                                ui.label("Qwen endpoint:");
                                ui.text_edit_singleline(endpoint);
                            });
                            ui.horizontal(|ui| {
                                ui.label("API ключ:");
                                ui.text_edit_singleline(api_key.get_or_insert_with(String::new));
                            });
                        }
                    }
                    ui.separator();
                    if ui.button("Gemini").clicked() {
                        self.ui_agent.provider = match &self.ui_agent.provider {
                            LlmProvider::Gemini { api_key } => {
                                LlmProvider::Gemini { api_key: api_key.clone() }
                            }
                            _ => LlmProvider::Gemini { api_key: String::new() },
                        };
                    }
                    if ui.button("Qwen (локально)").clicked() {
                        self.ui_agent.provider =
                            LlmProvider::Qwen { endpoint: "http://localhost:11434/v1".to_owned(), api_key: None };
                    }
                });

                ui.separator();
                ui.group(|ui| {
                    ui.label("Скриншот UI → фигуры");
                    if ui.button("Загрузить снимок…").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Изображение", &["png", "jpg", "jpeg", "bmp", "webp"])
                            .pick_file()
                        {
                            match ScreenAgent::load_image(&path) {
                                Some(_) => {
                                    self.screenshot_path = Some(path);
                                    self.status = "Скриншот загружен, готов к анализу".to_owned();
                                }
                                None => self.status = "Не удалось загрузить скриншот".to_owned(),
                            }
                        }
                    }
                    if let Some(path) = &self.screenshot_path {
                        ui.label(format!("Файл: {:?}", path.file_name().unwrap_or_default()));
                    }
                    ui.separator();
                    if ui.button("Агент: снимок → фигуры").clicked() {
                        let Some(path) = self.screenshot_path.clone() else {
                            self.status = "Сначала загрузите скриншот".to_owned();
                            return;
                        };
                        match ScreenAgent::load_image(&path) {
                            Some(img) => {
                                match self.ui_agent.analyze(&img) {
                                    Ok(result) => {
                                        UiAgent::apply_to_canvas(&mut self.free_canvas, &result);
                                        self.status = format!(
                                            "Агент: {} элементов из скриншота",
                                            result.elements.len()
                                        );
                                    }
                                    Err(e) => self.status = format!("Ошибка агента: {e}"),
                                }
                            }
                            None => self.status = "Не удалось загрузить скриншот".to_owned(),
                        }
                    }
                });
            });
        });

        self.free_canvas.ui(ctx, frame);
    }

    /// Сохранить файл с выбором пути через стандартный диалог ОС.
    fn save_as(&mut self) {
        let default_name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name().and_then(|n| n.to_str()))
            .unwrap_or("design.json");
        if let Some(path) = FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name(default_name)
            .save_file()
        {
            if let Err(e) = save_to_free_canvas(&self.free_canvas.doc, &path) {
                self.status = format!("Ошибка сохранения: {e}");
            } else {
                self.file_path = Some(path.clone());
                self.status = format!("Сохранено в {:?}", path);
            }
        }
    }
}

impl Default for DesignApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for DesignApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        self.ui(ctx, frame);
        ctx.request_repaint();
    }
}

/// Сохранить свободный холст в JSON-файл.
fn save_to_free_canvas(doc: &FreeCanvasDoc, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = doc.to_json()?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Загрузить свободный холст из JSON-файла.
fn load_free_canvas(path: &std::path::Path) -> Result<FreeCanvasDoc, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let doc = FreeCanvasDoc::load_from_json(&json)?;
    Ok(doc)
}

// Анализ результата (не используется напрямую, оставлен для ссылки).
#[allow(dead_code)]
fn _unused_result(_result: &AnalysisResult) {}
