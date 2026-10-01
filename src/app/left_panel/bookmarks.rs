use std::cell::RefCell;
use std::path::{Path, PathBuf};

use eframe::egui;
use egui_ltreeview::{DirPosition, NodeBuilder, TreeView, TreeViewBuilder};

use crate::constants;
use crate::{
    app::AppState,
    models::{BookmarkFS, BookmarkFSExt, BookmarkNode},
};

mod action_buffer;
mod dialog;
mod renaming;

/// A context-menu click, applied after the tree view has finished rendering.
/// Deferring these lets the tree-view callbacks avoid capturing `self` mutably.
/// All the paths are synthetic, that is, they're not real fs paths, they are bookmarks pseudo fs paths
enum ContextMenuAction {
    Open(PathBuf),
    Edit(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    AddBookmark(PathBuf),
    CreateFolder(PathBuf),
}

#[derive(Default)]
pub struct Bookmarks {
    /// Synthetic path pending a delete confirmation dialog.
    confirm_delete: Option<PathBuf>,
    renaming: Option<renaming::Renaming>,
    dialog: dialog::DialogWindow,

    action_buffer: action_buffer::ActionBuffer,

    // Vector of selected pseudo fs paths
    selection: Vec<PathBuf>,

    // Pseudo bookmark fs path
    to_open: Option<PathBuf>,
}
impl Bookmarks {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.style_mut().interaction.selectable_labels = false;
        // Placeholder: the real id is generated inside the scroll area (it depends
        // on the child ui's id) and captured out via `mut` for the focus check below.
        let mut tree_id = egui::Id::new("");
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                tree_id = ui.make_persistent_id("bookmarks_tree_view");
                // Context-menu clicks are recorded as deferred actions and applied
                // after the tree view has rendered, so the callbacks don't need to
                // capture `self` mutably (which would conflict with `draw_bookmark`).
                let menu_actions: RefCell<Option<ContextMenuAction>> = RefCell::new(None);
                let (_response, actions) = TreeView::new(tree_id)
                    .allow_drag_and_drop(true)
                    .fallback_context_menu(|ui, _| {
                        root_context_menu(ui, &menu_actions);
                    })
                    .show(ui, |builder| {
                        for bookmark in &state.storage.bookmarks {
                            self.draw_bookmark(
                                builder,
                                &bookmark,
                                PathBuf::from("/"),
                                &menu_actions,
                            );
                        }
                    });
                self.handle_actions(&mut state.storage.bookmarks, actions);
                self.handle_menu_actions(&mut state.storage.bookmarks, &menu_actions);
            });
        if self.renaming.as_ref().is_some_and(|r| r.is_to_commit)
            && let Some(renaming) = self.renaming.take()
        {
            renaming.commit(&mut self.action_buffer, &mut state.storage.bookmarks);
        }
        self.show_delete_confirmation(ui.ctx(), &mut state.storage.bookmarks);
        self.dialog.show(
            ui.ctx(),
            &mut self.action_buffer,
            &mut state.storage.bookmarks,
        );
        self.handle_shortcuts(ui, tree_id, &mut state.storage.bookmarks);
    }

    fn draw_bookmark(
        &mut self,
        builder: &mut TreeViewBuilder<'_, PathBuf>,
        bookmark: &BookmarkNode,
        parent_bookmark_fs_path: impl AsRef<Path>,
        menu_actions: &RefCell<Option<ContextMenuAction>>,
    ) {
        let parent_bookmark_fs_path = parent_bookmark_fs_path.as_ref();
        match bookmark {
            BookmarkNode::Note { name, .. } => {
                let current_bookmark_fs_path = parent_bookmark_fs_path.join(name);
                builder.node(
                    NodeBuilder::leaf(current_bookmark_fs_path.clone())
                        .icon(|ui| {
                            ui.label(egui_material_icons::icons::ICON_FILE_COPY);
                        })
                        .label_ui(|ui| {
                            self.node_label(ui, current_bookmark_fs_path.clone(), name.clone());
                        })
                        .context_menu(|ui| {
                            node_context_menu(
                                ui,
                                current_bookmark_fs_path.as_path(),
                                false,
                                menu_actions,
                            );
                        }),
                );
            }
            BookmarkNode::Folder { name, content } => {
                let current_bookmark_fs_path = parent_bookmark_fs_path.join(name);
                let is_open = builder.node(
                    NodeBuilder::dir(current_bookmark_fs_path.clone())
                        .default_open(false)
                        .icon(|ui| {
                            ui.label(egui_material_icons::icons::ICON_FOLDER);
                        })
                        .label_ui(|ui| {
                            self.node_label(ui, current_bookmark_fs_path.clone(), name.clone());
                        })
                        .context_menu(|ui| {
                            node_context_menu(
                                ui,
                                current_bookmark_fs_path.as_path(),
                                true,
                                menu_actions,
                            );
                        }),
                );

                if is_open {
                    for bookmark in content {
                        self.draw_bookmark(
                            builder,
                            &bookmark,
                            current_bookmark_fs_path.as_path(),
                            menu_actions,
                        );
                    }
                }
                builder.close_dir();
            }
        }
    }

    /// Build the label closure for a node, showing a `TextEdit` while it is being renamed.
    fn node_label(&mut self, ui: &mut egui::Ui, path: PathBuf, name: String) {
        let is_node_renaming = self
            .renaming
            .as_ref()
            .is_some_and(|renaming| renaming.path == path);
        if is_node_renaming {
            let renaming = self
                .renaming
                .as_mut()
                .expect("renaming must be Some() if is_node_renaming is true ");

            let response = renaming.ui(ui);

            let lost_focus = response.lost_focus();
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if lost_focus {
                renaming.is_to_commit = true;
            } else if escape {
                self.renaming = None;
            }
        } else {
            ui.label(name.clone());
        }
    }
    /// Render the delete confirmation dialog. Enter confirms, Esc cancels.
    fn show_delete_confirmation(&mut self, ctx: &egui::Context, bookmarks: &mut Vec<BookmarkNode>) {
        let Some(path) = self.confirm_delete.take() else {
            return;
        };
        let mut path = Some(path);
        let mut completed = false;

        egui::Window::new("Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let Some(path) = path.as_mut() else {
                    return;
                };
                ui.label(format!("Delete {}?", path.display()));
                ui.horizontal(|ui| {
                    let delete_response = ui.button("Delete");
                    let cancel_response = ui.button("Cancel");

                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if delete_response.clicked() || enter {
                        bookmarks.remove_bookmark(path.as_path());
                        completed = true;
                    }
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    if cancel_response.clicked() || escape {
                        completed = true;
                    }
                });
            });

        if !completed {
            self.confirm_delete = path;
        }
    }

    /// Apply the context-menu action that was deferred while the tree view rendered.
    fn handle_menu_actions(
        &mut self,
        bookmarks_root: &mut BookmarkFS,
        actions: &RefCell<Option<ContextMenuAction>>,
    ) {
        if let Some(action) = &*actions.borrow() {
            match action {
                ContextMenuAction::Open(path) => self.open(bookmarks_root, path),
                ContextMenuAction::Edit(bookmark_path) => {
                    let Some(note) = bookmarks_root.get_bookmark(bookmark_path.as_path()) else {
                        return;
                    };
                    match note {
                        BookmarkNode::Note { note_path, name } => {
                            self.dialog = dialog::DialogWindow(Some(dialog::Dialog::edit_note(
                                bookmark_path.clone(),
                                name.clone(),
                                note_path.clone(),
                            )));
                        }
                        BookmarkNode::Folder { .. } => {}
                    }
                }
                ContextMenuAction::Rename(path) => {
                    self.renaming = Some(renaming::Renaming::start(path.as_path()))
                }
                ContextMenuAction::Delete(path) => self.confirm_delete = Some(path.clone()),
                ContextMenuAction::AddBookmark(parent) => {
                    self.dialog = dialog::DialogWindow(Some(dialog::Dialog::create_note(
                        dbg!(parent).clone(),
                    )));
                }
                ContextMenuAction::CreateFolder(parent) => {
                    self.dialog =
                        dialog::DialogWindow(Some(dialog::Dialog::create_folder(parent.clone())));
                }
            }
        }
    }

    /// Handle keyboard shortcuts: Ctrl+Z undo, F2 rename, Del delete.
    fn handle_shortcuts(
        &mut self,
        ui: &mut egui::Ui,
        tree_id: egui::Id,
        bookmark_fs: &mut BookmarkFS,
    ) {
        if self.confirm_delete.is_some() {
            return;
        }
        // Only act when the tree itself has focus, so we don't hijack shortcuts
        // meant for other widgets (e.g. a text editor in the central panel).
        if !ui.memory(|m| m.has_focus(tree_id)) {
            return;
        }

        if ui.input_mut(|i| i.consume_shortcut(&constants::REDO_SHORTCUT)) {
            self.action_buffer.redo(bookmark_fs);
        } else if ui.input_mut(|i| i.consume_shortcut(&constants::UNDO_SHORTCUT)) {
            self.action_buffer.undo(bookmark_fs);
        } else if ui.input_mut(|i| i.consume_shortcut(&constants::RENAME_SHORTCUT)) {
            if let Some(sel) = self.selection.last() {
                let path = sel.clone();
                self.renaming = Some(renaming::Renaming::start(path.as_path()));
            }
        } else if ui.input_mut(|i| i.consume_shortcut(&constants::DELETE_SHORTCUT)) {
            if let Some(sel) = self.selection.last() {
                self.confirm_delete = Some(sel.clone());
            }
        }
    }

    fn handle_actions(
        &mut self,
        bookmark_fs_root: &mut BookmarkFS,
        actions: Vec<egui_ltreeview::Action<PathBuf>>,
    ) {
        for action in actions {
            match action {
                egui_ltreeview::Action::Activate(activate) => {
                    for path in activate.selected {
                        self.open(bookmark_fs_root, path.as_ref());
                    }
                }
                egui_ltreeview::Action::Move(dnd) => {
                    let target_dir = match &dnd.position {
                        DirPosition::First | DirPosition::Last => dnd.target.clone(),
                        DirPosition::After(_) | DirPosition::Before(_) => dnd
                            .target
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_default(),
                    };

                    self.move_bookmarks_to(bookmark_fs_root, dnd.source, target_dir);
                }
                egui_ltreeview::Action::MoveExternal(dnd) => {
                    self.move_bookmarks_to(bookmark_fs_root, dnd.source, "/");
                }
                egui_ltreeview::Action::SetSelected(selection) => {
                    self.selection = selection;
                }
                _ => {}
            }
        }
    }

    fn move_bookmarks_to(
        &mut self,
        bookmark_fs_root: &mut BookmarkFS,
        bookmarks: Vec<PathBuf>,
        to_dir: impl AsRef<Path>,
    ) {
        let to_dir = to_dir.as_ref();

        for src in bookmarks {
            self.action_buffer.do_action(
                action_buffer::Action::Move {
                    to: {
                        let Some(name) = src.file_name() else {
                            log::error!(
                                "Can't take the name of the bookmark at the path=\"{}\"",
                                src.to_string_lossy()
                            );
                            continue;
                        };
                        dbg!(to_dir.to_owned().join(name))
                    },
                    from: src,
                },
                bookmark_fs_root,
            );
        }
    }

    /// Open a bookmark by pseudo bookmark fs path
    fn open(&mut self, bookmarks_root: &BookmarkFS, bookmark_path: &Path) {
        let file_path = match bookmarks_root.get_bookmark(bookmark_path) {
            Some(BookmarkNode::Note {
                note_path: path, ..
            }) => path,
            _ => {
                log::error!(
                    "There's non a bookmark with bookmark fs path={}",
                    bookmark_path
                        .to_str()
                        .unwrap_or("<cannot the path convert to a string>")
                );
                return;
            }
        };
        if bookmark_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            self.to_open = Some(file_path.clone());
        } else {
            let _ = opener::open(file_path);
        }
    }
}

/// Render the context menu for a node. A click is recorded as a deferred
/// [`ContextMenuAction`] and applied by [`Bookmarks::handle_menu_actions`] after the
/// tree view has finished rendering.
fn node_context_menu(
    ui: &mut egui::Ui,
    path: &Path,
    is_dir: bool,
    action: &RefCell<Option<ContextMenuAction>>,
) {
    if ui.button("Open").clicked() {
        action.replace_with(|_| Some(ContextMenuAction::Open(path.to_path_buf())));
        ui.close();
    } else if !is_dir && ui.button("Edit").clicked() {
        action.replace_with(|_| Some(ContextMenuAction::Edit(path.to_path_buf())));
        ui.close();
    } else if ui.button("Rename").clicked() {
        action.replace_with(|_| Some(ContextMenuAction::Rename(path.to_path_buf())));
        ui.close();
    } else if ui.button("Delete").clicked() {
        action.replace_with(|_| Some(ContextMenuAction::Delete(path.to_path_buf())));
        ui.close();
    } else if is_dir {
        ui.separator();
        if ui.button("Add a bookmark").clicked() {
            action.replace_with(|_| Some(ContextMenuAction::AddBookmark(path.to_path_buf())));
            ui.close();
        } else if ui.button("Create a folder").clicked() {
            action.replace_with(|_| Some(ContextMenuAction::CreateFolder(path.to_path_buf())));
            ui.close();
        }
    }
}

/// Render the context menu for empty space / the root of the bookmarks tree.
fn root_context_menu(ui: &mut egui::Ui, actions: &RefCell<Option<ContextMenuAction>>) {
    if ui.button("Add a bookmark").clicked() {
        actions.replace_with(|_| Some(ContextMenuAction::AddBookmark(PathBuf::from("/"))));
        ui.close();
    }
    if ui.button("Create a folder").clicked() {
        actions.replace_with(|_| Some(ContextMenuAction::CreateFolder(PathBuf::from("/"))));
        ui.close();
    }
}
