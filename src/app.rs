use crate::constants;
use eframe::egui;

mod central_panel;
mod left_panel;
mod right_panel;

#[derive(Default)]
pub struct App {
    left_panel: left_panel::LeftPanel,
    central_panel: central_panel::CentralPanel,
}

impl App {
    pub fn new() -> Self {
        Self {
            left_panel: left_panel::LeftPanel::default(),
            central_panel: central_panel::CentralPanel::default(),
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
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {}
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.left_panel.show = self.central_panel.show_left_panel;
        self.left_panel.ui(ui);
        self.central_panel.ui(ui);
    }
}
