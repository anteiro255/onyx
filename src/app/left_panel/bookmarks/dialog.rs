use std::path::PathBuf;

use eframe::egui;

use crate::models::{
    BookmarkNode,
    bookmark::{self, BookmarkFSExt},
};

pub enum Dialog {
    CreateNote {
        name: String,
        // Pseudo bookmark fs path
        bookmark_folder_parent: PathBuf,
        // Real fs path
        note_path_buf: PathBuf,
    },
    CreateFolder {
        name: String,
        // Pseudo bookmark fs path
        bookmark_folder_parent: PathBuf,
    },
    EditNote {
        // Pseudo bookmark fs path
        bookmark_path: PathBuf,

        name: String,
        // Real fs path
        note_path: PathBuf,
    },
}

impl Dialog {
    pub fn create_note(parent_path: PathBuf) -> Self {
        Self::CreateNote {
            name: String::new(),
            bookmark_folder_parent: parent_path,
            note_path_buf: PathBuf::new(),
        }
    }
    pub fn create_folder(parent_path: PathBuf) -> Self {
        Self::CreateFolder {
            name: String::new(),
            bookmark_folder_parent: parent_path,
        }
    }
    pub fn edit_note(bookmark_path: PathBuf, name: String, note_path: PathBuf) -> Self {
        Self::EditNote {
            bookmark_path,
            name,
            note_path,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Dialog::CreateNote { name, .. } => name,
            Dialog::CreateFolder { name, .. } => name,
            Dialog::EditNote { name, .. } => name,
        }
    }
}

#[derive(Default)]
pub struct DialogWindow(pub Option<Dialog>);
impl DialogWindow {
    pub fn show(&mut self, ctx: &egui::Context, bookmark_fs: &mut bookmark::BookmarkFS) {
        let Some(dialog) = self.0.as_mut() else {
            return;
        };

        let title = match dialog {
            Dialog::CreateNote { .. } => "Create note",
            Dialog::CreateFolder { .. } => "Create folder",
            Dialog::EditNote { .. } => "Edit note",
        };

        let mut open = true;
        let mut completed = false;

        let position = ctx.input(|i| i.viewport_rect().center());

        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos(position)
            .show(ctx, |ui| match dialog {
                Dialog::CreateNote {
                    name,
                    bookmark_folder_parent,
                    note_path_buf: note_path,
                } => {
                    ui.label("Name:");
                    ui.text_edit_singleline(name);

                    ui.label("Note path:");
                    let mut s = note_path.to_string_lossy().to_string();
                    if ui.text_edit_singleline(&mut s).changed() {
                        *note_path = PathBuf::from(&s);
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() && !name.trim().is_empty() {
                            bookmark_fs.insert_into(
                                bookmark_folder_parent,
                                bookmark::BookmarkNode::Note {
                                    note_path: note_path.clone(),
                                    name: name.clone(),
                                },
                            );

                            completed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            completed = true;
                        }
                    });
                }
                Dialog::CreateFolder {
                    name,
                    bookmark_folder_parent,
                } => {
                    ui.label("Name:");
                    ui.text_edit_singleline(name);

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() && !name.trim().is_empty() {
                            bookmark_fs.insert_into(
                                bookmark_folder_parent,
                                BookmarkNode::Folder {
                                    name: name.clone(),
                                    content: vec![],
                                },
                            );
                            completed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            completed = true;
                        }
                    });
                }
                Dialog::EditNote {
                    name,
                    note_path,
                    bookmark_path,
                } => {
                    ui.label("Name:");
                    ui.text_edit_singleline(name);

                    ui.label("Note path:");
                    let mut s = note_path.to_string_lossy().to_string();
                    if ui.text_edit_singleline(&mut s).changed() {
                        *note_path = PathBuf::from(&s);
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() && !name.trim().is_empty() {
                            let Some(bookmark) =
                                bookmark_fs.get_mut_by_path(bookmark_path.as_path())
                            else {
                                log::error!("No bookmark at the path");
                                return;
                            };

                            if let BookmarkNode::Note {
                                note_path: bookmark_note_path,
                                name: bookmark_name,
                            } = bookmark
                            {
                                *bookmark_note_path = note_path.clone();
                                *bookmark_name = name.clone();
                            } else {
                                log::error!("The bookmark at the path is a folder");
                                return;
                            }

                            completed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            completed = true;
                        }
                    });
                }
            });

        if completed {
            self.0 = None;
        }
    }
}
