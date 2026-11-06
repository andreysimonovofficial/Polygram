//! Figma-подобный инструмент дизайна: свободный холст.
//! Рисуем прямоугольники, линии, эллипсы, текст, перетаскиваем элементы,
//! настраиваем свойства и экспортируем JSON, который Агент отрисовывает в точности.

use crate::design_format::*;
use crate::free_canvas::FreeCanvas;
use crate::screen_agent::ScreenAgent;
use eframe::Frame;
use egui::Context;
use rfd::FileDialog;
use std::path::PathBuf;

/// Состояние инструмента дизайна (только свободный холст).
pub struct DesignApp {
    pub free_canvas: FreeCanvas,
    pub file_path: Option<PathBuf>,
    pub status: String,
}

impl DesignApp {
    pub fn new() -> Self {
        Self {
            free_canvas: FreeCanvas::new(),
            file_path: None,
            status: "Готов".to_owned(),
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
                ui.group(|ui| {
                    ui.label("Агент");
                    if ui.button("Загрузить снимок экрана…").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Изображение", &["png", "jpg", "jpeg", "bmp", "webp"])
                            .pick_file()
                        {
                            match ScreenAgent::load_image(&path) {
                                Some(img) => {
                                    let result =
                                        ScreenAgent::apply_image_to_canvas(&mut self.free_canvas, &img);
                                    self.status = format!(
                                        "Агент: снимок {}×{}, фигур {} (пиксель-в-пиксель)",
                                        result.image_size.0,
                                        result.image_size.1,
                                        result.shapes,
                                    );
                                }
                                None => self.status = "Агент: не удалось загрузить изображение".to_owned(),
                            }
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
