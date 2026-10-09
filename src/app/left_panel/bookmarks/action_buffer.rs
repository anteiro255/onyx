use std::path::PathBuf;

use crate::models::bookmark;

#[derive(Clone)]
pub enum Action {
    Create {
        node: bookmark::BookmarkNode,
        parent_path: PathBuf,
    },
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Delete {
        node: bookmark::BookmarkNode,
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
                node: bookmark,
                parent_path,
            } => Self::Delete {
                node: bookmark,
                parent_path,
            },
            Self::Delete {
                node: bookmark,
                parent_path,
            } => Self::Create {
                node: bookmark,
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
    fn perform(&self, bookmark_fs: &mut bookmark::BookmarkFs) -> Result<(), PerformError> {
        match self {
            Self::Delete { parent_path, node } => {
                match bookmark_fs.remove_node(parent_path.join(node.name()).as_path()) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(PerformError::Delete {
                        parent: parent_path.clone(),
                        node: node.clone(),
                        err,
                    }),
                }
            }
            Self::Move { from, to } => {
                match bookmark_fs.move_bookmark(from.as_path(), to.as_path()) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(PerformError::Move {
                        from: from.clone(),
                        to: to.clone(),
                        err,
                    }),
                }
            }
            Self::Create { node, parent_path } => {
                match bookmark_fs.insert_node_into(node.clone(), parent_path.as_path()) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(PerformError::Create {
                        err,
                        parent_path: parent_path.clone(),
                        node: node.clone(),
                    }),
                }
            }
            Self::Edit { path, .. } => match bookmark_fs.get_mut_node(path.as_path()) {
                Ok(_) => Ok(()),
                Err(err) => Err(PerformError::Edit {
                    err,
                    path: path.clone(),
                }),
            },
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
    pub fn do_action(&mut self, action: Action, bookmark_fs: &mut bookmark::BookmarkFs) {
        if action.perform(bookmark_fs).is_ok() {
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
    pub fn undo(&mut self, bookmark_fs: &mut bookmark::BookmarkFs) {
        let Some(undo_action) = self.undo_stack.pop() else {
            return;
        };

        let redo_action = undo_action.clone().anti_action();

        if let Err(err) = undo_action.perform(bookmark_fs) {
            err.log();
        }
        self.redo_stack.push(redo_action);
    }

    /// Re-apply the most recently undone action, if any.
    pub fn redo(&mut self, bookmark_fs: &mut bookmark::BookmarkFs) {
        let Some(redo_action) = self.redo_stack.pop() else {
            return;
        };

        let undo_action = redo_action.clone().anti_action();

        if let Err(err) = redo_action.perform(bookmark_fs) {
            err.log();
        }
        self.undo_stack.push(undo_action);
    }
}
#[derive(thiserror::Error, Debug)]
pub enum PerformError {
    #[error("can't move from \"{}\" to \"{}\"", from.display(), to.display())]
    Move {
        from: PathBuf,
        to: PathBuf,
        #[source]
        err: bookmark::BookmarkFsError,
    },

    #[error("can't create \"{}\" in \"{}\"", node.fs_name(), parent_path.display())]
    Create {
        parent_path: PathBuf,
        node: bookmark::BookmarkNode,
        #[source]
        err: bookmark::BookmarkFsError,
    },

    #[error("can't delete \"{}\" from \"{}\"", node.fs_name(), parent.display())]
    Delete {
        parent: PathBuf,
        node: bookmark::BookmarkNode,
        #[source]
        err: bookmark::BookmarkFsError,
    },

    #[error("can't edit \"{}\"", path.display())]
    Edit {
        path: PathBuf,
        #[source]
        err: bookmark::BookmarkFsError,
    },
}
impl PerformError {
    pub fn log(self) {
        log::error!("{}", self);
    }
}
