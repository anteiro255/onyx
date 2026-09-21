use eframe::egui;

#[derive(Default)]
pub struct CentralPanel {
    pub show_left_panel: bool,
}
impl CentralPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            if ui
                .button(egui_material_icons::icons::ICON_LEFT_PANEL_CLOSE)
                .clicked()
            {
                self.show_left_panel = !self.show_left_panel;
            }
        });
    }
}
