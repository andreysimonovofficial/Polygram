//! Figma-подобный инструмент дизайна: свободный холст.
//! Рисуем прямоугольники, линии, эллипсы, текст, перетаскиваем элементы,
//! настраиваем свойства и экспортируем JSON, который Агент отрисовывает в точности.

use crate::design_format::*;
use crate::free_canvas::FreeCanvas;
use egui::Context;
use std::path::PathBuf;

/// Состояние инструмента дизайна (только свободный холст).
pub struct DesignApp {
    pub free_canvas: FreeCanvas,
    pub file_path: Option<PathBuf>,
    pub show_export_dialog: bool,
    pub export_json: String,
    pub show_open_dialog: bool,
    pub show_save_as_dialog: bool,
    pub dialog_path: String,
    pub status: String,
}

impl DesignApp {
    pub fn new() -> Self {
        Self {
            free_canvas: FreeCanvas::new(),
            file_path: None,
            show_export_dialog: false,
            export_json: String::new(),
            show_open_dialog: false,
            show_save_as_dialog: false,
            dialog_path: String::new(),
            status: "Готов".to_owned(),
        }
    }

    /// Отобразить весь интерфейс инструмента дизайна.
    pub fn ui(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Новый").clicked() {
                    self.free_canvas.doc = FreeCanvasDoc::default();
                    self.free_canvas.selected = None;
                    self.file_path = None;
                    self.status = "Новый холст создан".to_owned();
                }
                if ui.button("Открыть").clicked() {
                    self.dialog_path = self.file_path.as_ref().map_or(String::new(), |p| p.to_string_lossy().to_string());
                    self.show_open_dialog = true;
                }
                if ui.button("Сохранить").clicked() {
                    if let Some(path) = &self.file_path {
                        if let Err(e) = save_to_free_canvas(&self.free_canvas.doc, path) {
                            self.status = format!("Ошибка сохранения: {e}");
                        } else {
                            self.status = format!("Сохранено в {:?}", path);
                        }
                    } else {
                        self.show_save_as_dialog = true;
                        self.dialog_path = String::new();
                    }
                }
                if ui.button("Сохранить как…").clicked() {
                    self.dialog_path = self.file_path.as_ref().map_or(String::new(), |p| p.to_string_lossy().to_string());
                    self.show_save_as_dialog = true;
                }
                ui.separator();
                if ui.button("Экспорт для Агента").clicked() {
                    self.show_export_dialog = true;
                }
                ui.separator();
                ui.label(format!("Статус: {}", self.status));
            });
        });

        self.file_dialogs(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.free_canvas.ui(ui);
        });

        if self.show_export_dialog {
            self.export_dialog(ctx);
        }
    }

    /// Диалоги открытия и сохранения файла дизайна (JSON).
    fn file_dialogs(&mut self, ctx: &Context) {
        if self.show_open_dialog {
            egui::Window::new("Открыть файл дизайна")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 180.0])
                .show(ctx, |ui| {
                    ui.label("Путь к JSON-файлу дизайна:");
                    ui.separator();
                    ui.text_edit_singleline(&mut self.dialog_path);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Открыть").clicked() {
                            let path = std::path::PathBuf::from(self.dialog_path.trim());
                            match load_free_canvas(&path) {
                                Ok(doc) => {
                                    self.free_canvas.doc = doc;
                                    self.free_canvas.selected = None;
                                    self.file_path = Some(path);
                                    self.status = format!("Открыто: {:?}", self.file_path);
                                    self.show_open_dialog = false;
                                }
                                Err(e) => self.status = format!("Ошибка открытия: {e}"),
                            }
                        }
                        if ui.button("Отмена").clicked() {
                            self.show_open_dialog = false;
                        }
                    });
                });
        }

        if self.show_save_as_dialog {
            egui::Window::new("Сохранить дизайн как…")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 180.0])
                .show(ctx, |ui| {
                    ui.label("Путь для сохранения JSON-файла:");
                    ui.separator();
                    ui.text_edit_singleline(&mut self.dialog_path);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Сохранить").clicked() {
                            let path = std::path::PathBuf::from(self.dialog_path.trim());
                            if path.as_os_str().is_empty() {
                                self.status = "Укажите путь к файлу".to_owned();
                            } else if let Err(e) = save_to_free_canvas(&self.free_canvas.doc, &path) {
                                self.status = format!("Ошибка сохранения: {e}");
                            } else {
                                self.file_path = Some(path.clone());
                                self.status = format!("Сохранено в {:?}", path);
                                self.show_save_as_dialog = false;
                            }
                        }
                        if ui.button("Отмена").clicked() {
                            self.show_save_as_dialog = false;
                        }
                    });
                });
        }
    }

    /// Экспорт дизайна в JSON для Агента.
    fn export_dialog(&mut self, ctx: &Context) {
        egui::Window::new("Экспорт для Агента")
            .collapsible(true)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.label("JSON-дизайн для передачи Агенту:");
                ui.separator();
                self.export_json = self.free_canvas.doc.to_json().unwrap_or_default();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.code(&self.export_json);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Копировать в буфер обмена").clicked() {
                        ctx.copy_text(self.export_json.clone());
                        self.status = "JSON скопирован в буфер обмена".to_owned();
                    }
                    if ui.button("Закрыть").clicked() {
                        self.show_export_dialog = false;
                    }
                });
            });
    }
}

impl Default for DesignApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for DesignApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
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
