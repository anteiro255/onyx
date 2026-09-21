use eframe::egui;

mod bookmarks;
mod files;
mod search;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tab {
    #[default]
    Files,
    Bookmarks,
    Search,
}

#[derive(Default)]
pub struct Tabs {
    selected: Tab,

    files_tab: files::Files,
    bookmarks_tab: bookmarks::Bookmarks,
    search_tab: search::Search,
}
impl Tabs {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.selected,
                Tab::Files,
                egui_material_icons::icons::ICON_FILE_COPY,
            );
            ui.selectable_value(
                &mut self.selected,
                Tab::Bookmarks,
                egui_material_icons::icons::ICON_BOOKMARKS,
            );
            ui.selectable_value(
                &mut self.selected,
                Tab::Search,
                egui_material_icons::icons::ICON_SEARCH,
            );
        });
        ui.separator();
    }
}

#[derive(Default)]
pub struct LeftPanel {
    pub show: bool,
    tabs: Tabs,
}
impl LeftPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("left_panel")
            .resizable(true)
            .show_collapsible(ui, &mut self.show, |ui| {
                self.tabs.ui(ui);
                match self.tabs.selected {
                    Tab::Bookmarks => self.tabs.bookmarks_tab.ui(ui),
                    Tab::Files => self.tabs.files_tab.ui(ui),
                    Tab::Search => self.tabs.search_tab.ui(ui),
                }
            });
    }
}
