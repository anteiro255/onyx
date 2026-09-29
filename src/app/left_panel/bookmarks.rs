use std::cell::RefCell;
use std::path::{Path, PathBuf};

use eframe::egui;
use egui_ltreeview::{DirPosition, DragAndDrop, NodeBuilder, TreeView, TreeViewBuilder};

use crate::{
    app::AppState,
    models::{BookmarkFS, BookmarkFSExt, BookmarkNode},
};

mod action_buffer;
mod renaming;

/// A context-menu click, applied after the tree view has finished rendering.
/// Deferring these lets the tree-view callbacks avoid capturing `self` mutably.
/// All the paths are synthetic, that is, they're not real fs paths, they are bookmarks pseudo fs paths
enum ContextMenuAction {
    Open(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    /// Create a note bookmark inside the folder at this (synthetic) path.
    AddBookmark(PathBuf),
    /// Create a folder bookmark inside the folder at this (synthetic) path.
    CreateFolder(PathBuf),
}

/// What kind of bookmark is being created in the create dialog.
enum CreateKind {
    Note,
    Folder,
}

/// State of the "create bookmark" dialog.
struct CreateDialog {
    kind: CreateKind,
    name: String,

    /// Synthetic bookmarks pseudo fs path of the folder the new bookmark is added into.
    parent_path: PathBuf,
    /// The real path of the note file (only used for [`CreateKind::Note`]).
    path: String,
}
impl CreateDialog {
    fn note(parent_path: PathBuf) -> Self {
        Self {
            kind: CreateKind::Note,
            parent_path,
            name: String::new(),
            path: String::new(),
        }
    }
    fn folder(parent_path: PathBuf) -> Self {
        Self {
            kind: CreateKind::Folder,
            parent_path,
            name: String::new(),
            path: String::new(),
        }
    }
}

#[derive(Default)]
pub struct Bookmarks {
    /// Synthetic path pending a delete confirmation dialog.
    confirm_delete: Option<PathBuf>,
    renaming: Option<renaming::Renaming>,
    /// State of the create dialog, if open.
    create: Option<CreateDialog>,

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
            renaming.commit(&mut state.storage.bookmarks);
        }
        self.show_delete_confirmation(ui.ctx(), &mut state.storage.bookmarks);
        self.show_create_dialog(ui.ctx(), &mut state.storage.bookmarks);
        self.handle_shortcuts(ui, tree_id);
    }

    fn draw_bookmark(
        &mut self,
        builder: &mut TreeViewBuilder<'_, PathBuf>,
        bookmark: &BookmarkNode,
        parent_bookmark_fs_path: impl AsRef<Path>,
        menu_actions: &RefCell<Option<ContextMenuAction>>,
    ) {
        let bookmark_fs_path = parent_bookmark_fs_path.as_ref();
        match bookmark {
            BookmarkNode::Note { name, .. } => {
                let note_bookmark_fs_path = bookmark_fs_path.join(name);
                builder.node(
                    NodeBuilder::leaf(note_bookmark_fs_path.clone())
                        .icon(|ui| {
                            ui.label(egui_material_icons::icons::ICON_FILE_COPY);
                        })
                        .label_ui(|ui| {
                            self.node_label(ui, note_bookmark_fs_path.clone(), name.clone());
                        })
                        .context_menu(|ui| {
                            node_context_menu(
                                ui,
                                note_bookmark_fs_path.as_path(),
                                false,
                                menu_actions,
                            );
                        }),
                );
            }
            BookmarkNode::Folder { name, content } => {
                let folder_bookmark_fs_path = bookmark_fs_path.join(name);
                let is_open = builder.node(
                    NodeBuilder::dir(folder_bookmark_fs_path.clone())
                        .default_open(false)
                        .icon(|ui| {
                            ui.label(egui_material_icons::icons::ICON_FOLDER);
                        })
                        .label_ui(|ui| {
                            self.node_label(ui, folder_bookmark_fs_path.clone(), name.clone());
                        })
                        .context_menu(|ui| {
                            node_context_menu(
                                ui,
                                folder_bookmark_fs_path.as_path(),
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
                            folder_bookmark_fs_path.as_path(),
                            menu_actions,
                        );
                    }
                }
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
                        bookmarks.remove_by_path(path.as_path());
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

    /// Render the create bookmark dialog. Enter confirms, Esc cancels.
    fn show_create_dialog(&mut self, ctx: &egui::Context, bookmarks: &mut Vec<BookmarkNode>) {
        let Some(create) = self.create.take() else {
            return;
        };
        let title = match &create.kind {
            CreateKind::Note => "Add a bookmark",
            CreateKind::Folder => "Create a folder",
        };
        let mut create = Some(create);
        let mut open = true;
        let mut completed = false;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let Some(create) = create.as_mut() else {
                    return;
                };
                match create.kind {
                    CreateKind::Note => {
                        ui.label("Path to note:");
                        ui.add(egui::TextEdit::singleline(&mut create.path).desired_width(300.0));
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut create.name).desired_width(300.0));
                    }
                    CreateKind::Folder => {
                        ui.label("Folder name:");
                        ui.add(egui::TextEdit::singleline(&mut create.name).desired_width(300.0));
                    }
                }
                ui.horizontal(|ui| {
                    let ok_response = ui.button("Create");
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ok_response.clicked() || enter {
                        let name = create.name.trim().to_string();
                        if !name.is_empty() {
                            match create.kind {
                                CreateKind::Note => {
                                    let path = create.path.trim().to_string();
                                    if !path.is_empty() {
                                        add_note(bookmarks, &create.parent_path, &name, &path);
                                    }
                                }
                                CreateKind::Folder => {
                                    add_folder(bookmarks, &create.parent_path, &name);
                                }
                            }
                        }
                        completed = true;
                    }
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    if ui.button("Cancel").clicked() || escape {
                        completed = true;
                    }
                });
            });

        if open && !completed {
            self.create = create;
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
                ContextMenuAction::Rename(path) => {
                    self.renaming = Some(renaming::Renaming::start(path.as_path()))
                }
                ContextMenuAction::Delete(path) => self.confirm_delete = Some(path.clone()),
                ContextMenuAction::AddBookmark(parent) => {
                    self.create = Some(CreateDialog::note(parent.clone()));
                }
                ContextMenuAction::CreateFolder(parent) => {
                    self.create = Some(CreateDialog::folder(parent.clone()));
                }
            }
        }
    }

    /// Handle keyboard shortcuts: Ctrl+Z undo, F2 rename, Del delete.
    fn handle_shortcuts(&mut self, ui: &mut egui::Ui, tree_id: egui::Id) {
        if self.confirm_delete.is_some() {
            return;
        }
        // Only act when the tree itself has focus, so we don't hijack shortcuts
        // meant for other widgets (e.g. a text editor in the central panel).
        if !ui.memory(|m| m.has_focus(tree_id)) {
            return;
        }
        let modifiers = ui.input(|i| i.modifiers);
        let command = modifiers.command_only();

        if command && ui.input(|i| i.key_pressed(egui::Key::Z)) {
            self.action_buffer.undo();
        } else if ui.input(|i| i.key_pressed(egui::Key::F2)) {
            if let Some(sel) = self.selection.last() {
                let path = sel.clone();
                self.renaming = Some(renaming::Renaming::start(path.as_path()));
            }
        } else if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            if let Some(sel) = self.selection.last() {
                self.confirm_delete = Some(sel.clone());
            }
        }
    }

    fn handle_actions(
        &mut self,
        bookmarks_root: &mut BookmarkFS,
        actions: Vec<egui_ltreeview::Action<PathBuf>>,
    ) {
        for action in actions {
            match action {
                egui_ltreeview::Action::Activate(activate) => {
                    for path in activate.selected {
                        self.open(bookmarks_root, path.as_ref());
                    }
                }
                egui_ltreeview::Action::Move(dnd) => move_dropped(bookmarks_root, &dnd),
                egui_ltreeview::Action::MoveExternal(dnd) => {
                    // Dropped outside any node (empty space) -> move to root.
                    move_to_dir(bookmarks_root, &dnd.source, Path::new("/"));
                }
                egui_ltreeview::Action::SetSelected(selection) => {
                    self.selection = selection;
                }
                _ => {}
            }
        }
    }

    /// Open a bookmark by pseudo bookmark fs path
    fn open(&mut self, bookmarks_root: &BookmarkFS, bookmark_path: &Path) {
        let file_path = match bookmarks_root.get_by_path(bookmark_path) {
            Some(BookmarkNode::Note { path, .. }) => path,
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
    actions: &RefCell<Option<ContextMenuAction>>,
) {
    if ui.button("Open").clicked() {
        actions.replace_with(|_| Some(ContextMenuAction::Open(path.to_path_buf())));
        ui.close();
    } else if ui.button("Rename").clicked() {
        actions.replace_with(|_| Some(ContextMenuAction::Rename(path.to_path_buf())));
        ui.close();
    } else if ui.button("Delete").clicked() {
        actions.replace_with(|_| Some(ContextMenuAction::Delete(path.to_path_buf())));
        ui.close();
    } else if is_dir {
        ui.separator();
        if ui.button("Add a bookmark").clicked() {
            actions.replace_with(|_| Some(ContextMenuAction::AddBookmark(path.to_path_buf())));
            ui.close();
        } else if ui.button("Create a folder").clicked() {
            actions.replace_with(|_| Some(ContextMenuAction::CreateFolder(path.to_path_buf())));
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

/// Move the dragged nodes to the drop target folder.
///
/// Dropping onto a folder (First/Last) moves into it; dropping before/after a
/// node moves into that node's parent folder.
fn move_dropped(bookmarks: &mut BookmarkFS, dnd: &DragAndDrop<PathBuf>) {
    let target_dir = match &dnd.position {
        DirPosition::First | DirPosition::Last => dnd.target.clone(),
        DirPosition::After(_) | DirPosition::Before(_) => dnd
            .target
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default(),
    };
    move_to_dir(bookmarks, &dnd.source, &target_dir);
}

/// Move a set of nodes into `target_dir`.
fn move_to_dir(bookmarks: &mut BookmarkFS, source: &Vec<PathBuf>, target_dir: &Path) {
    for src in source {
        let _ = bookmarks.move_to(src.as_path(), target_dir);
    }
}

/// Add a note bookmark to the folder at the synthetic `parent` path.
fn add_note(root: &mut BookmarkFS, parent: &Path, name: &String, path: &String) {
    add_bookmark(
        root,
        parent,
        BookmarkNode::Note {
            path: PathBuf::from(path.as_str()),
            name: name.clone(),
        },
    );
}

/// Add a folder bookmark to the folder at the synthetic `parent` path.
fn add_folder(root: &mut BookmarkFS, parent: &Path, name: &String) {
    add_bookmark(
        root,
        parent,
        BookmarkNode::Folder {
            name: name.clone(),
            content: vec![],
        },
    );
}

/// Add `bookmark` to the folder at the synthetic `parent` path (or the root if
/// `parent` is the root path). Does nothing if the parent folder is not found.
fn add_bookmark(bookmarks: &mut BookmarkFS, parent: &Path, bookmark: BookmarkNode) {
    bookmarks.insert_into(parent, bookmark);
}
