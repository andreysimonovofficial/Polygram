//! Figma-подобный инструмент дизайна: свободный холст.
//! Рисуем прямоугольники, линии, эллипсы, текст, перетаскиваем элементы,
//! настраиваем свойства и экспортируем JSON, который Агент отрисовывает в точности.

use crate::design_format::*;
use crate::free_canvas::FreeCanvas;
use crate::screen_agent::ScreenAgent;
use crate::ui_agent::{AnalysisResult, LlmProvider, UiAgent};
use eframe::Frame;
use egui::{Context, FontData, FontDefinitions, FontFamily};
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::mpsc;

/// Состояние инструмента дизайна (только свободный холст).
pub struct DesignApp {
    pub free_canvas: FreeCanvas,
    pub file_path: Option<PathBuf>,
    pub status: String,
    /// Агент анализа UI-скриншота.
    pub ui_agent: UiAgent,
    /// Путь к последнему загруженному скриншоту.
    pub screenshot_path: Option<PathBuf>,
    /// Получатель результата фонового анализа (если поток ещё работает).
    pub analysis_receiver: Option<mpsc::Receiver<Result<AnalysisResult, String>>>,
}

impl DesignApp {
    pub fn new() -> Self {
        Self {
            free_canvas: FreeCanvas::new(),
            file_path: None,
            status: "Готов".to_owned(),
            ui_agent: UiAgent::new(LlmProvider::default()),
            screenshot_path: None,
        analysis_receiver: None,
        }
    }

    /// Зарегистрировать системные шрифты Windows в egui, чтобы FontFamily::Name
    /// (Arial / Inter / Segoe UI) не вызывал панику при отрисовке текста.
    pub fn register_fonts(ctx: &Context) {
        let mut fonts = FontDefinitions::default();

        let mut add_font = |key: &str, path: &str, name: FontFamily| {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(key.to_owned(), FontData::from_owned(data));
                fonts
                    .families
                    .entry(name)
                    .or_default()
                    .push(key.to_owned());
            }
        };

        // Сначала пробуем загрузить Arial (чаще всего доступен в Windows).
        add_font("arial", "C:/Windows/Fonts/arial.ttf", FontFamily::Name("Arial".into()));
        add_font(
            "segoeui",
            "C:/Windows/Fonts/segoeui.ttf",
            FontFamily::Name("Segoe UI".into()),
        );
        add_font("inter", "C:/Windows/Fonts/inter.ttf", FontFamily::Name("Inter".into()));

        // Fallback: если какой-то именованный шрифт не зарегистрирован,
        // используем данные уже загруженного шрифта, чтобы избежать паники egui.
        let fallback_key = fonts
            .families
            .get(&FontFamily::Name("Arial".into()))
            .and_then(|keys| keys.first())
            .cloned();
        if let Some(fb) = fallback_key {
            for name in [
                FontFamily::Name("Arial".into()),
                FontFamily::Name("Inter".into()),
                FontFamily::Name("Segoe UI".into()),
            ] {
                if fonts.families.get(&name).map_or(true, |keys| keys.is_empty()) {
                    fonts
                        .families
                        .entry(name)
                        .or_default()
                        .push(fb.clone());
                }
            }
        }

        ctx.set_fonts(fonts);
    }

    /// Отобразить весь интерфейс инструмента дизайна.
    pub fn ui(&mut self, ctx: &Context, frame: &mut Frame) {
        // Проверяем, пришёл ли результат фонового анализа.
        if let Some(rx) = &self.analysis_receiver {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(analysis) => {
                        UiAgent::apply_to_canvas(&mut self.free_canvas, &analysis);
                        self.status = format!(
                            "Агент: {} элементов из скриншота",
                            analysis.elements.len()
                        );
                    }
                    Err(e) => self.status = format!("Ошибка агента: {e}"),
                }
                self.analysis_receiver = None;
                ctx.request_repaint();
            }
        }

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
                    let analysing = self.analysis_receiver.is_some();
                    let btn = if analysing { "Анализирую…" } else { "Агент: снимок → фигуры" };
                    if ui.button(btn).clicked() && !analysing {
                        let Some(path) = self.screenshot_path.clone() else {
                            self.status = "Сначала загрузите скриншот".to_owned();
                            return;
                        };
                        match ScreenAgent::load_image(&path) {
                            Some(img) => {
                                let provider = self.ui_agent.provider.clone();
                                let (tx, rx) = mpsc::channel();
                                std::thread::spawn(move || {
                                    let agent = UiAgent::new(provider);
                                    let result = agent.analyze(&img);
                                    let _ = tx.send(result);
                                });
                                self.analysis_receiver = Some(rx);
                                self.status = "Агент: анализирую скриншот…".to_owned();
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
