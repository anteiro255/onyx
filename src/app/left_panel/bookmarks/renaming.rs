use std::path::{Path, PathBuf};

use eframe::egui;

use crate::{app::left_panel::bookmarks::action_buffer, models::bookmark};

pub struct Renaming {
    /// Synthetic path of the bookmark being renamed inline.
    pub path: PathBuf,
    pub buf: String,
    pub is_to_commit: bool,

    is_to_focus: bool,
}
impl Renaming {
    pub fn start(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            buf: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            is_to_commit: false,
            is_to_focus: true,
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.buf)
                .desired_width(150.0)
                .id(ui.make_persistent_id("bookmark_rename")),
        );
        if self.is_to_focus {
            response.request_focus();
            self.is_to_focus = false;
        }
        response
    }
    pub fn commit(
        self,
        action_buffer: &mut action_buffer::ActionBuffer,
        bookmark_fs: &mut bookmark::BookmarkFs,
    ) {
        let new_path = {
            let Some(parent) = self.path.parent() else {
                log::error!(
                    "can't get the parent of the path: {}",
                    self.path.to_string_lossy()
                );
                return;
            };
            parent.join(self.buf)
        };
        action_buffer.do_action(
            action_buffer::Action::Move {
                from: self.path,
                to: new_path,
            },
            bookmark_fs,
        );
    }
}
