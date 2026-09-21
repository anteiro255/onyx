use eframe::egui;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct Files {
    root: PathBuf,
}

impl Default for Files {
    fn default() -> Self {
        Self {
            root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl Files {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let root = self.root.clone();
                self.draw_dir(ui, &root);
            });
    }

    fn draw_dir(&mut self, ui: &mut egui::Ui, path: &Path) {
        ui.style_mut().interaction.selectable_labels = false;
        let Ok(read) = std::fs::read_dir(path) else {
            return;
        };

        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| {
            let is_file = e.file_type().map(|t| !t.is_dir()).unwrap_or(true);
            (is_file, e.file_name())
        });

        for entry in entries {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            if name.starts_with('.') {
                continue;
            }

            if is_dir {
                let id = ui.make_persistent_id(&entry_path);
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    id,
                    false,
                )
                .show_header(ui, |ui| {
                    ui.label(egui_material_icons::icons::ICON_FOLDER);
                    ui.label(name);
                })
                .body(|ui| {
                    self.draw_dir(ui, &entry_path);
                });
            } else {
                let _response = ui
                    .horizontal(|ui| {
                        ui.label(egui_material_icons::icons::ICON_FILE_COPY);
                        ui.label(name.clone());
                    })
                    .response;
            }
        }
    }
}
