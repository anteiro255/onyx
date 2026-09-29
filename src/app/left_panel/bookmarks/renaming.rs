use std::path::{Path, PathBuf};

use eframe::egui;

use crate::models::{BookmarkFS, BookmarkFSExt};

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
    pub fn commit(self, root: &mut BookmarkFS) {
        let Some(bookmark_node) = root.get_by_path(self.path.as_path()) else {
            log::error!(
                "Can't get the {} bookmark in the bookmarks pseudo fs to rename",
                self.path
                    .to_str()
                    .unwrap_or("<unable to convert the path to a string>")
            );
            return;
        };
        bookmark_node.set_name(self.buf);
    }
}
