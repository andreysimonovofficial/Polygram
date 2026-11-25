//! Состояние приложения Polygram.
//! Хранит Canvas, движок ядра, отслеживает клики по холсту, выделение объектов,
//! перетаскивание отделов за шапку, диалог переименования и диалоги показателей.
//!
//! Декомпозирован по модулям (один файл — одна ответственность):
//! - `drag`      — жизненный цикл драга объектов/контейнеров/показателей
//! - `hit_test`  — определение объекта/шапки/отдела под экранной позицией
//! - `selection` — управление выделением объектов
//! - `ui_event`  — обработка событий CentralPanel (клик, драг, selection box и т.д.)
//! - `departments` — вычисление геометрии отделов (fit_departments_to_children)
//! - `dialogs`   — модальные диалоги
//! - `interaction` — контекстное меню, диалоги показателей, переименование
//! - `icons`     — иконки

use std::collections::HashSet;

use egui::Pos2;
use polygram_canvas::{Canvas, register_inter_fonts};
use polygram_core::{Command, Engine, runtime::Published, types::Scope};
use polygram_core::geometry::Geometry;

mod dialogs;
mod icons;
mod departments;
mod interaction;
mod drag;
mod hit_test;
mod selection;
mod ui_event;

/// Главное состояние приложения Polygram.
pub struct PolygramApp {
    canvas: Canvas,
    /// Движок ядра Polygram (иерархия, значения, статусы, команды).
    engine: Engine,
    /// Экранная позиция последнего правого клика по холсте (открывает контекстное меню).
    right_click: Option<Pos2>,
    /// Объект, на который был сделан правый клик (если отдел или процесс).
    right_click_object: Option<String>,
    /// Установлены ли загрузчики изображений egui_extras (включая SVG).
    image_loaders_installed: bool,
    /// Зарегистрированы ли шрифты Inter в контексте egui.
    inter_fonts_installed: bool,
    /// Меню только что открыто правым кликом — игнорировать close-логику в этом кадре.
    menu_just_opened: bool,
    /// Выделенные объекты.
    pub selected_objects: Vec<String>,
    /// Объекты, которые сейчас перетаскиваются (один или группа выделенных).
    drag_objects: Vec<String>,
    /// Смещение курсора относительно левого верхнего угла каждого объекта в клетках.
    drag_offsets: Vec<(f32, f32)>,
    /// Начальная позиция мыши при начале драга (экран).
    drag_start_screen: Option<Pos2>,
    /// Стартовые клетки перетаскиваемых объектов (для отмены при возврате на место).
    drag_start_cells: Vec<(i32, i32)>,
    /// Отдел, над которым нависает перетаскиваемый объект.
    drag_over: Option<String>,
    /// Свободная клетка для призрачной копии при перетаскивании объекта в отдел.
    drag_ghost: Option<(i32, i32)>,
    /// Направление контейнера перетаскиваемого объекта (для Контейнера/Показателя).
    drag_object_dir: Option<polygram_core::types::ContainerDirection>,
    /// Источник перетаскивания показателя: (id объекта, направление, имя показателя).
    drag_metric_source: Option<(String, polygram_core::types::Direction, String)>,
    /// Начальная позиция перетаскивания показателя (экран).
    drag_metric_start: Option<Pos2>,
    /// Фракционное смещение (в клетках) для плавного визуального перемещения
    /// перетаскиваемых объектов и их детей во время драга. rect остаётся
    /// привязанным к сетке (целые клетки) для логики и сброса; визуально
    /// объект сдвигается на эту фракцию при отрисовке.
    drag_visual_offset: Option<(f32, f32)>,
    /// Идентификаторы объектов, которым применять drag_visual_offset при
    /// отрисовке (перетаскиваемые + дети dragged-отделов).
    drag_visual_ids: HashSet<String>,
    /// Объект, который сейчас переименовывается.
    pub rename_target: Option<String>,
    /// Введённое новое имя для переименования.
    pub rename_input: String,
    /// Начало выделения рамкой (selection box).
    selection_box_start: Option<Pos2>,
    /// Состояние диалогов.
    dialogs: dialogs::DialogState,
    /// Кэш геометрии с прогнозами (invalidates при изменении объектов или начале драга).
    cached_geometry: Option<Geometry>,
}

impl PolygramApp {
    /// Создаёт новое состояние приложения с холстом и движком по умолчанию.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
            engine: Engine::new(),
            right_click: None,
            right_click_object: None,
            image_loaders_installed: false,
            inter_fonts_installed: false,
            menu_just_opened: false,
            selected_objects: Vec::new(),
            drag_objects: Vec::new(),
            drag_offsets: Vec::new(),
            drag_start_screen: None,
            drag_start_cells: Vec::new(),
            drag_over: None,
            drag_ghost: None,
            drag_object_dir: None,
            drag_metric_source: None,
            drag_metric_start: None,
            drag_visual_offset: None,
            drag_visual_ids: HashSet::new(),
            rename_target: None,
            rename_input: String::new(),
            selection_box_start: None,
            dialogs: dialogs::DialogState::new(),
            cached_geometry: None,
        }
    }

    /// Выполнить команду в движке и обновить опубликованное состояние.
    pub fn apply_cmd(&mut self, cmd: Command) {
        let _ = self.engine.apply(Scope::Root, cmd);
        self.sync_canvas_from_engine();
    }

    /// Выполнить пачку команд в движке.
    pub fn apply_batch(&mut self, cmds: Vec<Command>) {
        let _ = self.engine.apply_batch(Scope::Root, cmds);
        self.sync_canvas_from_engine();
    }

    /// Синхронизировать объекты Canvas с геометрией движка (после применения команд).
    fn sync_canvas_from_engine(&mut self) {
        let mut geometry = self.engine.geometry();
        // Отдел не имеет фиксированного размера: рамка обтягивается под детей.
        crate::app::departments::fit_departments_to_children(&mut geometry, None);
        self.canvas.objects = geometry.objects;
        self.cached_geometry = None;
    }

    /// Последний опубликованный снимок состояния движка.
    pub fn published(&self) -> Published {
        let geometry = self.engine.geometry();
        let dynamic = self.engine.dynamic_state();
        let metric_defs = self.engine.metric_def_views();
        Published {
            rev: self.engine.rev(),
            dynamic,
            geometry,
            metric_defs,
            camera_fit_requested: self.engine.camera_fit_requested(),
            auto_screenshot: self.engine.auto_screenshot(),
        }
    }
}

impl Default for PolygramApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for PolygramApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Устанавливаем загрузчики изображений egui_extras (включая SVG) при первом кадре.
        if !self.image_loaders_installed {
            egui_extras::loaders::install_image_loaders(ui.ctx());
            self.image_loaders_installed = true;
        }

        // Регистрируем шрифты Inter (500/600/700) при первом кадре.
        if !self.inter_fonts_installed {
            register_inter_fonts(ui.ctx());
            self.inter_fonts_installed = true;
        }

        let published = self.published();

        // Если открыт полноэкранный экран показателей — рисуем его вместо холста.
        if self.dialogs.show_metrics_screen {
            interaction::container_menu::render_metrics_screen(
                self,
                ui,
                &published,
                self.canvas.theme,
            );
            ui.ctx().request_repaint();
            return;
        }

        egui::CentralPanel::default().show(ui, |ui| {
            self.handle_canvas_panel(ui, &published);
        });

        // Диалоги после панели (вне CentralPanel).
        interaction::context_menu(self, ui.ctx(), &published, self.canvas.theme);
        interaction::add_metric::handle_add_metric_dialog(self, ui.ctx(), &published);
        interaction::rename_dialog(self, ui.ctx());
        interaction::rename::inline_rename_editor(self, ui.ctx(), &published);
        interaction::place_object::handle_place_object_confirm(self, ui.ctx(), &published);

        // request_repaint() НЕ вызывается безусловно каждый кадр.
        // Вызывается только при реальном изменении состояния:
        // - перемещение камеры (в handle_camera)
        // - драг объекта (в update_drag_objects)
        // - открытие/закрытие диалогов (в interaction модулях)
        // Это устраняет фризы от непрерывного максимального FPS.
    }
}
