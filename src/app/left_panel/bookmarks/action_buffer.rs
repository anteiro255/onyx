use std::path::PathBuf;

use crate::models::{
    BookmarkFS,
    bookmark::{self, BookmarkFSExt},
};

#[derive(Clone)]
pub enum Action {
    Create {
        bookmark: bookmark::BookmarkNode,
        parent_path: PathBuf,
    },
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Delete {
        bookmark: bookmark::BookmarkNode,
        parent_path: PathBuf,
    },
    Edit {
        path: PathBuf,
        old: bookmark::BookmarkNode,
        new: bookmark::BookmarkNode,
    },
}
impl Action {
    /// the anti action for an action X is the action that should be performed to revert the changes of the action X
    /// anti_action(anti_action(action)) == action
    fn anti_action(self) -> Self {
        match self {
            Self::Create {
                bookmark,
                parent_path,
            } => Self::Delete {
                bookmark,
                parent_path,
            },
            Self::Delete {
                bookmark,
                parent_path,
            } => Self::Create {
                bookmark,
                parent_path,
            },
            Self::Move { from, to } => Self::Move { from: to, to: from },
            Self::Edit { path, old, new } => Self::Edit {
                path,
                old: new,
                new: old,
            },
        }
    }
    fn perform(&self, bookmark_fs: &mut bookmark::BookmarkFS) -> bool {
        match self {
            Self::Delete {
                parent_path,
                bookmark,
            } => bookmark_fs
                .remove_bookmark(parent_path.join(bookmark.name()).as_path())
                .is_some(),
            Self::Move { from, to } => bookmark_fs.move_bookmark(from.as_path(), to.as_path()),
            Self::Create {
                bookmark,
                parent_path,
            } => bookmark_fs.insert_bookmark_into(parent_path.as_path(), bookmark.clone()),
            Self::Edit { path, new, .. } => {
                let Some(bookmark) = bookmark_fs.get_mut_bookmark(path.as_path()) else {
                    log::error!("No bookmark at the path=\"{}\"", path.to_string_lossy());
                    return false;
                };

                *bookmark = new.clone();
                true
            }
        }
    }
}

/// The way ActionBuffer works:
/// 	when we do an action, we register it via `.perform()`, it gets it's anti action and adds it to the undo buffer
/// 	when we undo an action, we call `.undo()`, it performs the anti action from undo buffer
/// 	when we redo an action, we call `.redo()`, it performs the real action from redo buffer
#[derive(Default)]
pub struct ActionBuffer {
    // Instead of storing executed actions, we store the actions needed to undo them.
    undo_stack: Vec<Action>,
    redo_stack: Vec<Action>,
}

impl ActionBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn do_action(&mut self, action: Action, bookmark_fs: &mut BookmarkFS) {
        if action.perform(bookmark_fs) {
            self.register_action(action);
        }
    }

    /// Register the action
    pub fn register_action(&mut self, action: Action) {
        let undo_action = action.anti_action();
        self.undo_stack.push(undo_action);
        self.redo_stack.clear();
    }

    /// Reverse the most recent action, if any.
    pub fn undo(&mut self, bookmark_fs: &mut bookmark::BookmarkFS) {
        let Some(undo_action) = self.undo_stack.pop() else {
            return;
        };

        let redo_action = undo_action.clone().anti_action();

        undo_action.perform(bookmark_fs);
        self.redo_stack.push(redo_action);
    }

    /// Re-apply the most recently undone action, if any.
    pub fn redo(&mut self, bookmark_fs: &mut bookmark::BookmarkFS) {
        let Some(redo_action) = self.redo_stack.pop() else {
            return;
        };

        let undo_action = redo_action.clone().anti_action();

        redo_action.perform(bookmark_fs);
        self.undo_stack.push(undo_action);
    }

    /// Whether there is anything to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether there is anything to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
