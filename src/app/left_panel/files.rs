use eframe::egui;
use egui_ltreeview::{DirPosition, DragAndDrop, NodeBuilder, TreeView, TreeViewBuilder};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// A pending copy/cut operation, used for the paste action.
enum Clipboard {
    Cut(PathBuf),
    Copy(PathBuf),
}

/// An inverse operation, applied by Ctrl+Z to undo a filesystem change.
enum Action {
    /// A file/dir was created at `path`; undo deletes it.
    Create { path: PathBuf },
    /// `from` was renamed to `to`; undo renames back.
    Rename { from: PathBuf, to: PathBuf },
    /// `from` was moved to `to`; undo moves back.
    Move { from: PathBuf, to: PathBuf },
    /// `src` was copied to `dest`; undo deletes the copy.
    Copy { dest: PathBuf },
    /// `path` was deleted; undo restores it from `backup`.
    Delete { trash_item: trash::TrashItem },
}

/// A context-menu click, applied after the tree view has finished rendering.
/// Deferring these lets the tree-view callbacks avoid capturing `self` mutably.
enum MenuAction {
    Open(PathBuf),
    CreateFile(PathBuf),
    CreateDir(PathBuf),
    Rename(PathBuf),
    Copy(PathBuf),
    Cut(PathBuf),
    Paste(PathBuf),
    Delete(PathBuf),
}

pub struct Files {
    root: PathBuf,

    // UI & Action States
    clipboard: Option<Clipboard>,
    /// Path pending a delete confirmation dialog.
    confirm_delete: Option<PathBuf>,
    selection: Vec<PathBuf>,
    renaming: Option<PathBuf>,
    rename_buffer: String,
    rename_focus_requested: bool,
    actions: VecDeque<Action>,

    /// Set when a file that can be rendered in-app (text/image) is opened.
    /// The central panel should consume this and clear it after rendering.
    pub file_to_open: Option<PathBuf>,
}

impl Default for Files {
    fn default() -> Self {
        Self {
            // Default to the current working directory so the tree has
            // something to show before a vault/folder is chosen.
            root: std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
            clipboard: None,
            confirm_delete: None,
            selection: Vec::new(),
            renaming: None,
            rename_buffer: String::new(),
            rename_focus_requested: false,
            actions: VecDeque::new(),
            file_to_open: None,
        }
    }
}

impl Files {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().interaction.selectable_labels = false;
        // Placeholder: the real id is generated inside the scroll area (it depends
        // on the child ui's id) and captured out via `mut` for the focus check below.
        let mut tree_id = egui::Id::new("");
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                tree_id = ui.make_persistent_id("file_tree_view");
                let root = self.root.clone();
                // Context-menu clicks are recorded as deferred actions and applied
                // after the tree view has rendered, so the callbacks don't need to
                // capture `self` mutably (which would conflict with `draw_dir`).
                let menu_actions = RefCell::new(None);

                // the context menu shows "Paste" only if there's a clipboard, that is, something was copied
                let has_clipboard = self.clipboard.is_some();
                let (_response, actions) = TreeView::new(tree_id)
                    .allow_drag_and_drop(true)
                    .fallback_context_menu(|ui, _| {
                        root_context_menu(ui, &root, has_clipboard, &menu_actions);
                    })
                    .show(ui, |builder| {
                        self.draw_dir(builder, &root, &menu_actions);
                    });
                self.handle_actions(&root, actions);
                self.handle_menu_actions(&menu_actions);
            });

        self.show_delete_confirmation(ui.ctx());
        self.handle_shortcuts(ui, &self.root.clone(), tree_id);
    }

    /// Create an `untitledN` file/folder in `dir` and start renaming it inline.
    fn create_untitled(&mut self, dir: &Path, is_folder: bool) {
        let path = next_untitled_path(dir);
        let ok = if is_folder {
            std::fs::create_dir(&path).is_ok()
        } else {
            std::fs::File::create(&path).is_ok()
        };
        if ok {
            self.push_action(Action::Create { path: path.clone() });
            self.start_rename(&path);
        }
    }

    /// Put `path` on the clipboard as a copy (or cut if `is_cut`).
    fn set_clipboard(&mut self, path: &Path, is_cut: bool) {
        self.clipboard = Some(match is_cut {
            true => Clipboard::Cut(path.to_path_buf()),
            false => Clipboard::Copy(path.to_path_buf()),
        });
    }

    /// Begin inline rename of `path`.
    fn start_rename(&mut self, path: &Path) {
        self.renaming = Some(path.to_path_buf());
        self.rename_buffer = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.rename_focus_requested = false;
    }

    /// Commit an inline rename (or cancel if the new name is empty).
    fn commit_rename(&mut self, path: &Path) {
        let new_name = self.rename_buffer.trim().to_string();
        if !new_name.is_empty() {
            if let Some(parent) = path.parent() {
                let new_path = parent.join(new_name);
                if std::fs::rename(path, &new_path).is_ok() {
                    self.push_action(Action::Rename {
                        from: path.to_path_buf(),
                        to: new_path,
                    });
                }
            }
        }
        self.renaming = None;
    }

    /// Paste the current clipboard item into `target_dir` (move for cut, copy for copy).
    fn paste_into(&mut self, target_dir: &Path) {
        let Some(clipboard) = self.clipboard.take() else {
            return;
        };
        match clipboard {
            Clipboard::Cut(src) => {
                let dest = target_dir.join(src.file_name().unwrap_or_default());
                let _ = self.move_item(&src, &dest);
            }
            Clipboard::Copy(src) => {
                let dest = target_dir.join(src.file_name().unwrap_or_default());
                if copy_recursive(&src, &dest) {
                    self.push_action(Action::Copy { dest });
                }
            }
        }
    }

    /// Move the dragged nodes to the drop target directory.
    fn move_dropped(&mut self, dnd: &DragAndDrop<PathBuf>) {
        // Dropping onto a directory (First/Last) moves into it.
        // Dropping before/after a node moves into that node's parent directory.
        let target_dir = match &dnd.position {
            DirPosition::First | DirPosition::Last => dnd.target.clone(),
            DirPosition::After(_) | DirPosition::Before(_) => dnd
                .target
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default(),
        };
        self.move_to_dir(&dnd.source, &target_dir);
    }

    /// Move a set of nodes into `target_dir`.
    fn move_to_dir(&mut self, source: &Vec<PathBuf>, target_dir: &Path) {
        for src in source {
            let dest = target_dir.join(src.file_name().unwrap_or_default());
            let _ = self.move_item(src, &dest);
        }
    }

    /// Move `src` to `dest`, falling back to copy+delete for cross-device moves.
    /// Returns `true` if the item ended up at `dest`.
    fn move_item(&mut self, src: &Path, dest: &Path) -> bool {
        let moved = if std::fs::rename(src, dest).is_ok() {
            true
        } else if copy_recursive(src, dest) {
            delete_permanent(src);
            true
        } else {
            false
        };
        if moved {
            self.push_action(Action::Move {
                from: src.to_path_buf(),
                to: dest.to_path_buf(),
            });
        }
        moved
    }

    /// Push an undo action, keeping the stack bounded.
    fn push_action(&mut self, action: Action) {
        self.actions.push_back(action);
        if self.actions.len() > 100 {
            self.actions.remove(0);
        }
    }

    /// Undo the most recent filesystem operation.
    fn undo_action(&mut self) {
        let Some(action) = self.actions.pop_back() else {
            return;
        };
        match action {
            Action::Create { path } => delete_permanent(&path),
            Action::Rename { from, to } => {
                let _ = std::fs::rename(&to, &from);
            }
            Action::Move { from, to } => {
                let _ = std::fs::rename(&to, &from);
            }
            Action::Copy { dest } => delete_permanent(&dest),
            Action::Delete { trash_item } => {
                let _ = trash::os_limited::restore_all([trash_item]);
            }
        }
    }

    fn draw_dir(
        &mut self,
        builder: &mut TreeViewBuilder<'_, PathBuf>,
        path: &Path,
        menu_actions: &RefCell<Option<MenuAction>>,
    ) {
        let Ok(read) = std::fs::read_dir(path) else {
            return;
        };

        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| {
            let is_file = e.file_type().map(|t| !t.is_dir()).unwrap_or(true);
            (is_file, e.file_name())
        });

        let has_clipboard = self.clipboard.is_some();

        for entry in entries {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            if name.starts_with('.') {
                continue;
            }

            if is_dir {
                let is_open = builder.node(
                    NodeBuilder::dir(entry_path.clone())
                        .default_open(false)
                        .icon(|ui| {
                            ui.label(egui_material_icons::icons::ICON_FOLDER);
                        })
                        .label_ui(|ui| self.node_label(ui, entry_path.clone(), name.clone()))
                        .context_menu(|ui| {
                            node_menu(ui, entry_path.as_path(), true, has_clipboard, menu_actions);
                        }),
                );
                if is_open {
                    self.draw_dir(builder, &entry_path, menu_actions);
                }
                builder.close_dir();
            } else {
                builder.node(
                    NodeBuilder::leaf(entry_path.clone())
                        .icon(|ui| {
                            ui.label(egui_material_icons::icons::ICON_FILE_COPY);
                        })
                        .label_ui(|ui| self.node_label(ui, entry_path.clone(), name.clone()))
                        .context_menu(|ui| {
                            node_menu(ui, entry_path.as_path(), false, has_clipboard, menu_actions);
                        }),
                );
            }
        }
    }

    /// Build the label closure for a node, showing a `TextEdit` while it is being renamed.
    fn node_label(&mut self, ui: &mut egui::Ui, path: PathBuf, name: String) {
        let is_renaming = self.renaming.as_ref() == Some(&path);
        if is_renaming {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.rename_buffer)
                    .desired_width(150.0)
                    .id(ui.make_persistent_id("file_rename")),
            );
            if !self.rename_focus_requested {
                response.request_focus();
                self.rename_focus_requested = true;
            }
            let lost_focus = response.lost_focus();
            let enter = lost_focus && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if enter {
                self.commit_rename(&path);
            } else if escape {
                self.renaming = None;
            } else if lost_focus {
                self.commit_rename(&path);
            }
        } else {
            ui.label(name.clone());
        }
    }

    /// Render the delete confirmation dialog. Enter confirms, Esc cancels.
    fn show_delete_confirmation(&mut self, ctx: &egui::Context) {
        let Some(path) = self.confirm_delete.take() else {
            return;
        };
        let mut path = Some(path);
        let mut open = true;
        let mut completed = false;
        let mut focus_requested = false;

        egui::Window::new("Delete")
            .open(&mut open)
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
                    if !focus_requested {
                        delete_response.request_focus();
                        focus_requested = true;
                    }
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if delete_response.clicked() || enter {
                        match delete(path) {
                            Ok(trash_item) => self.push_action(Action::Delete { trash_item }),
                            Err(err) => log::error!("{}", err),
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
            self.confirm_delete = path;
        }
    }

    /// Handle actions emitted by the tree view (open, drag-and-drop move, selection).
    fn handle_actions(&mut self, root: &Path, actions: Vec<egui_ltreeview::Action<PathBuf>>) {
        for action in actions {
            match action {
                egui_ltreeview::Action::Activate(activate) => {
                    for path in activate.selected {
                        self.open_path(path);
                    }
                }
                egui_ltreeview::Action::Move(dnd) => self.move_dropped(&dnd),
                egui_ltreeview::Action::MoveExternal(dnd) => {
                    // Dropped outside any node (empty space) -> move to root.
                    self.move_to_dir(&dnd.source, root);
                }
                egui_ltreeview::Action::SetSelected(selection) => {
                    self.selection = selection;
                }
                _ => {}
            }
        }
    }

    /// Apply the context-menu action that was deferred while the tree view rendered.
    fn handle_menu_actions(&mut self, actions: &RefCell<Option<MenuAction>>) {
        if let Some(action) = &*actions.borrow() {
            match action {
                MenuAction::Open(path) => self.open_path(path.clone()),
                MenuAction::CreateFile(dir) => self.create_untitled(dir.as_path(), false),
                MenuAction::CreateDir(dir) => self.create_untitled(dir.as_path(), true),
                MenuAction::Rename(path) => self.start_rename(path.as_path()),
                MenuAction::Copy(path) => self.set_clipboard(path.as_path(), false),
                MenuAction::Cut(path) => self.set_clipboard(path.as_path(), true),
                MenuAction::Paste(dir) => self.paste_into(dir.as_path()),
                MenuAction::Delete(path) => self.confirm_delete = Some(path.clone()),
            }
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            self.file_to_open = Some(path);
        } else {
            let _ = opener::open(path);
        }
    }

    /// Handle explorer keyboard shortcuts: Ctrl+C copy, Ctrl+X cut, Ctrl+V paste,
    /// Ctrl+Z undo, F2 rename, Del delete.
    ///
    /// Note: egui's backend converts Ctrl+C/X/V into `Event::Copy`/`Event::Cut`/`Event::Paste`
    /// rather than plain key events, so we detect those instead of `key_pressed`.
    fn handle_shortcuts(&mut self, ui: &mut egui::Ui, root: &Path, tree_id: egui::Id) {
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
        let copy_event = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
        let cut_event = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Cut)));
        let paste_event = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))));

        if copy_event {
            if let Some(sel) = self.selection.last() {
                let path = sel.clone();
                self.set_clipboard(&path, false);
            }
        } else if cut_event {
            if let Some(sel) = self.selection.last() {
                let path = sel.clone();
                self.set_clipboard(&path, true);
            }
        } else if paste_event || (command && ui.input(|i| i.key_pressed(egui::Key::V))) {
            // Paste into the selected directory, or the root if none is selected.
            let target = match self.selection.last() {
                Some(p) if p.is_dir() => p.to_path_buf(),
                _ => root.to_path_buf(),
            };
            self.paste_into(&target);
        } else if command && ui.input(|i| i.key_pressed(egui::Key::Z)) {
            self.undo_action();
        } else if ui.input(|i| i.key_pressed(egui::Key::F2)) {
            if let Some(sel) = self.selection.last() {
                let path = sel.clone();
                self.start_rename(&path);
            }
        } else if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            if let Some(sel) = self.selection.last() {
                self.confirm_delete = Some(sel.clone());
            }
        }
    }
}

/// Render the context menu for a node. A click is recorded as a deferred
/// [`MenuAction`] and applied by [`Files::handle_menu_actions`] after the
/// tree view has finished rendering.
fn node_menu(
    ui: &mut egui::Ui,
    path: &Path,
    is_dir: bool,
    has_clipboard: bool,
    actions: &RefCell<Option<MenuAction>>,
) {
    if ui.button("Open").clicked() {
        actions.replace_with(|_| Some(MenuAction::Open(path.to_path_buf())));
        ui.close();
    }
    if is_dir {
        ui.separator();
        if ui.button("New File").clicked() {
            actions.replace_with(|_| Some(MenuAction::CreateFile(path.to_path_buf())));
            ui.close();
        }
        if ui.button("New Folder").clicked() {
            actions.replace_with(|_| Some(MenuAction::CreateDir(path.to_path_buf())));
            ui.close();
        }
    }
    ui.separator();
    if ui.button("Rename").clicked() {
        actions.replace_with(|_| Some(MenuAction::Rename(path.to_path_buf())));
        ui.close();
    }
    if ui.button("Copy").clicked() {
        actions.replace_with(|_| Some(MenuAction::Copy(path.to_path_buf())));
        ui.close();
    }
    if ui.button("Copy Path").clicked() {
        ui.ctx().copy_text(path.to_string_lossy().into_owned());
        ui.close();
    }
    if ui.button("Cut").clicked() {
        actions.replace_with(|_| Some(MenuAction::Cut(path.to_path_buf())));
        ui.close();
    }
    if is_dir && has_clipboard {
        if ui.button("Paste").clicked() {
            actions.replace_with(|_| Some(MenuAction::Paste(path.to_path_buf())));
            ui.close();
        }
    }
    ui.separator();
    if ui.button("Delete").clicked() {
        actions.replace_with(|_| Some(MenuAction::Delete(path.to_path_buf())));
        ui.close();
    }
}

/// Render the context menu for empty space / the root directory.
fn root_context_menu(
    ui: &mut egui::Ui,
    root: &Path,
    has_clipboard: bool,
    actions: &RefCell<Option<MenuAction>>,
) {
    if ui.button("New File").clicked() {
        actions.replace_with(|_| Some(MenuAction::CreateFile(root.to_path_buf())));
        ui.close();
    }
    if ui.button("New Folder").clicked() {
        actions.replace_with(|_| Some(MenuAction::CreateDir(root.to_path_buf())));
        ui.close();
    }
    if has_clipboard {
        ui.separator();
        if ui.button("Paste").clicked() {
            actions.replace_with(|_| Some(MenuAction::Paste(root.to_path_buf())));
            ui.close();
        }
    }
}

/// Find the next free `untitledN` path in `dir`.
fn next_untitled_path(dir: &Path) -> PathBuf {
    for i in 0.. {
        let candidate = dir.join(format!("untitled{}", i));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

/// Permanently delete a file or directory (no undo).
fn delete_permanent(path: &Path) {
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

/// Find a free `base`, `base_1`, `base_2`, ... path in `trash`.
fn unique_trash_path(trash: &Path, base: &str) -> PathBuf {
    let candidate = trash.join(base);
    if !candidate.exists() {
        return candidate;
    }
    for i in 1.. {
        let candidate = trash.join(format!("{}_{}", base, i));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

/// Recursively copy a file or directory from `src` to `dest`.
///
/// Returns `false` (aborting the copy) if `dest` is inside `src`, which would
/// otherwise recurse into the newly created copy forever.
fn copy_recursive(src: &Path, dest: &Path) -> bool {
    if src == dest {
        // Copying onto itself is a no-op.
        return true;
    }
    if dest.starts_with(src) {
        return false;
    }
    if src.is_dir() {
        if std::fs::create_dir_all(dest).is_err() {
            return false;
        }
        let Ok(read) = std::fs::read_dir(src) else {
            return false;
        };
        for entry in read.flatten() {
            let dest_child = dest.join(entry.file_name());
            if !copy_recursive(&entry.path(), &dest_child) {
                return false;
            }
        }
        true
    } else {
        std::fs::copy(src, dest).is_ok()
    }
}

pub fn delete(path: impl AsRef<std::path::Path>) -> Result<trash::TrashItem, trash::Error> {
    let path = path.as_ref();
    let canonical_path = path.canonicalize().map_err(|err| trash::Error::Unknown {
        description: format!("Failed to canonicalize path {:?}: {err}", path),
    })?;
    trash::delete(&canonical_path)?;
    let items = trash::os_limited::list()?;
    items
        .into_iter()
        .filter(|item| item.original_path() == canonical_path)
        .max_by_key(|item| item.time_deleted)
        .ok_or_else(|| trash::Error::Unknown {
            description: format!(
                "File {:?} was moved to trash, but its TrashItem could not be found",
                canonical_path
            ),
        })
}
