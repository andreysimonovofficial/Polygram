//! Figma-подобный инструмент дизайна: свободный холст.
//! Рисуем прямоугольники, линии, эллипсы, текст, перетаскиваем элементы,
//! настраиваем свойства и экспортируем JSON, который Агент отрисовывает в точности.

use crate::design_format::*;
use crate::free_canvas::FreeCanvas;
use crate::screen_agent::ScreenAgent;
use crate::ui_agent::{AnalysisResult, LlmProvider, UiAgent};
use eframe::Frame;
use egui::{Context, FontData, FontDefinitions};
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

    /// Зарегистрировать шрифты в egui, чтобы FontFamily::Name
    /// (Arial / Inter / Segoe UI) не вызывал панику при отрисовке текста.
    /// Шрифты загружаются из папки `fonts/` рядом с исполняемым файлом
    /// (файлы вида arial-regular.ttf, arial-bold.ttf и т.д.), чтобы
    /// использовались именно те начертания, что выбраны в интерфейсе.
    /// Если файла нет — используется системный шрифт Windows как fallback.
    pub fn register_fonts(ctx: &Context) {
        let mut fonts = FontDefinitions::default();

        let families = [
            FontFamily::Arial,
            FontFamily::Inter,
            FontFamily::SegoeUI,
        ];
        let weights = [
            FontWeight::Thin,
            FontWeight::ExtraLight,
            FontWeight::Light,
            FontWeight::Regular,
            FontWeight::Medium,
            FontWeight::SemiBold,
            FontWeight::Bold,
            FontWeight::ExtraBold,
            FontWeight::Black,
        ];

        // Сначала обеспечиваем наличие хотя бы одного шрифта (абсолютный fallback).
        let fallback_key = Self::load_any_system_font(&mut fonts);

        // Для каждой комбинации семейство + насыщенность создаём запись в
        // fonts.families. Если конкретный файл не найден, используем fallback_key.
        for family in families {
            for weight in weights {
                let key = font_key(family, weight);
                let mut loaded = false;

                // 1. Папка fonts/ рядом с исполняемым файлом.
                if let Some(dir) = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                {
                    let path = dir.join(format!("fonts/{}.ttf", key));
                    if let Ok(data) = std::fs::read(&path) {
                        fonts.font_data.insert(key.clone(), FontData::from_owned(data));
                        fonts
                            .families
                            .entry(egui::FontFamily::Name(key.clone().into()))
                            .or_default()
                            .push(key.clone());
                        loaded = true;
                    }
                }
                // 2. Относительно текущей рабочей директории (для cargo run).
                if !loaded {
                    let path = format!("fonts/{}.ttf", key);
                    if let Ok(data) = std::fs::read(&path) {
                        fonts.font_data.insert(key.clone(), FontData::from_owned(data));
                        fonts
                            .families
                            .entry(egui::FontFamily::Name(key.clone().into()))
                            .or_default()
                            .push(key.clone());
                        loaded = true;
                    }
                }
                // 3. Системный шрифт Windows для этой комбинации.
                if !loaded {
                    for path in Self::font_file_candidates(family, weight) {
                        if let Ok(data) = std::fs::read(&path) {
                            fonts.font_data.insert(key.clone(), FontData::from_owned(data));
                            fonts
                                .families
                                .entry(egui::FontFamily::Name(key.clone().into()))
                                .or_default()
                                .push(key.clone());
                            loaded = true;
                            break;
                        }
                    }
                }
                // 4. Fallback на любой доступный шрифт, чтобы egui не паниковал.
                if !loaded {
                    if let Some(fb) = &fallback_key {
                        fonts
                            .families
                            .entry(egui::FontFamily::Name(key.clone().into()))
                            .or_default()
                            .push(fb.clone());
                    }
                }
            }
        }

        ctx.set_fonts(fonts);
    }

    /// Загружает любой доступный системный шрифт Windows как гарантированный fallback.
    /// Возвращает Some(key) если удалось загрузить, иначе None.
    fn load_any_system_font(fonts: &mut FontDefinitions) -> Option<String> {
        for path in [
            "C:/Windows/Fonts/arial.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/times.ttf",
        ] {
            if let Ok(data) = std::fs::read(path) {
                let key = "system".to_owned();
                fonts.font_data.insert(key.clone(), FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Name(key.clone().into()))
                        .or_default()
                        .push(key.clone());
                    return Some(key);
            }
        }
        None
    }

    /// Список путей-кандидатов к системному файлу шрифта Windows для комбинации семейства и насыщенности.
    /// Возвращает пути в порядке убывания предпочтительности.
    /// Для Inter подгружаются переменные шрифты из C:/Users/andre/Downloads/Inter/,
    /// которые поддерживают все насыщенности через ось wght.
    fn font_file_candidates(family: FontFamily, weight: FontWeight) -> Vec<String> {
        let base = match family {
            FontFamily::Arial => "arial",
            FontFamily::Inter => "inter",
            FontFamily::SegoeUI => "segoeui",
        };
        let mut candidates: Vec<String> = Vec::new();

        // Для Inter пробуем переменный шрифт из Downloads (поддерживает все насыщенности).
        if family == FontFamily::Inter {
            candidates.push(
                "C:/Users/andre/Downloads/Inter/Inter-VariableFont_opsz,wght.ttf".to_owned(),
            );
        }

        match weight {
            FontWeight::Thin => {
                candidates.push(format!("C:/Windows/Fonts/{}thin.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}light.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::ExtraLight => {
                candidates.push(format!("C:/Windows/Fonts/{}extralight.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}light.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::Light => {
                candidates.push(format!("C:/Windows/Fonts/{}light.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::Regular => {
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::Medium => {
                candidates.push(format!("C:/Windows/Fonts/{}medium.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::SemiBold => {
                candidates.push(format!("C:/Windows/Fonts/{}semibold.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}b.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::Bold => {
                candidates.push(format!("C:/Windows/Fonts/{}b.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}bold.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::ExtraBold => {
                candidates.push(format!("C:/Windows/Fonts/{}extrabold.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}b.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
            FontWeight::Black => {
                candidates.push(format!("C:/Windows/Fonts/{}black.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}b.ttf", base));
                candidates.push(format!("C:/Windows/Fonts/{}.ttf", base));
            }
        }
        candidates
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
