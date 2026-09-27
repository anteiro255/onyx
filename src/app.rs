use std::time;

use crate::constants;
use eframe::egui;

mod central_panel;
mod left_panel;
mod right_panel;

mod vault_storage;

pub struct AppState {
    pub storage: vault_storage::Storage,
    pub last_storage_save: time::Instant,
}

pub struct App {
    left_panel: left_panel::LeftPanel,
    central_panel: central_panel::CentralPanel,

    state: AppState,
}
impl App {
    pub fn new() -> Self {
        Self {
            left_panel: left_panel::LeftPanel::default(),
            central_panel: central_panel::CentralPanel::default(),

            state: AppState {
                storage: vault_storage::Storage::new(),
                last_storage_save: time::Instant::now(),
            },
        }
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            constants::WINDOW_NAME,
            options,
            Box::new(|cc| {
                egui_material_icons::initialize(&cc.egui_ctx);
                Ok(Box::new(self))
            }),
        )?;
        Ok(())
    }
    fn terminate(&mut self) {
        self.state.storage.save();
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            self.terminate();
        }

        if self.state.last_storage_save.elapsed() >= constants::VOLUME_STORAGE_AUTOSAVE_INTERVAL {
            self.state.last_storage_save = time::Instant::now();
            self.state.storage.save();
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.left_panel.show = self.central_panel.show_left_panel;
        self.left_panel.ui(ui, &mut self.state);
        self.central_panel.ui(ui);
    }
}
