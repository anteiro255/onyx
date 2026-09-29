use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BookmarkNode {
    Note {
        /// Relative path from the volume root
        path: PathBuf,
        name: String,
    },
    Folder {
        name: String,
        content: BookmarkFS,
    },
}

pub type BookmarkFS = Vec<BookmarkNode>;

impl BookmarkNode {
    /// Returns the name of the bookmark item regardless of its variant.
    pub fn name(&self) -> &str {
        match self {
            BookmarkNode::Note { name, .. } => name,
            BookmarkNode::Folder { name, .. } => name,
        }
    }
    pub fn set_name(&mut self, new_name: String) -> &str {
        match self {
            BookmarkNode::Note { name, .. } => {
                *name = new_name;
                name.as_str()
            }
            BookmarkNode::Folder { name, .. } => {
                *name = new_name;
                name.as_str()
            }
        }
    }
}

pub trait BookmarkFSExt {
    /// Recursively retrieves a mutable reference to a `Bookmark` by traversing the given relative path.
    fn get_mut_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode>;

    /// Recursively retrieves a reference to a `Bookmark` by traversing the given relative path.
    fn get_by_path(&self, path: &Path) -> Option<&BookmarkNode>;

    /// Retrieves a mutable reference to the parent `Bookmark` (which must be a `Folder`) for the given path.
    fn get_mut_parent_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode>;

    /// Retrieves a reference to the parent `Bookmark` (which must be a `Folder`) for the given path.
    fn get_parent_by_path(&self, path: &Path) -> Option<&BookmarkNode>;

    /// Removes and returns the bookmark node at `path`, or `None` if it doesn't exist.
    fn remove_by_path(&mut self, path: &Path) -> Option<BookmarkNode>;

    /// Moves the node at `from` into the folder at `to_dir` (the root if `to_dir` is the root path).
    /// Returns `false` if `from` doesn't exist, `to_dir` isn't a folder, or `to_dir` is inside `from`.
    fn move_to(&mut self, from: &Path, to_dir: &Path) -> bool;

    /// Inserts `node` into the folder at `dir` (the root if `dir` is the root path).
    /// Returns `false` if the target folder doesn't exist.
    fn insert_into(&mut self, dir: &Path, node: BookmarkNode) -> bool;
}

impl BookmarkFSExt for BookmarkFS {
    fn get_mut_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode> {
        let components = components_of(path);

        if components.is_empty() {
            return None;
        }

        let mut current_fs = self;

        for (i, segment) in components.iter().enumerate() {
            let found = current_fs.iter_mut().find(|item| item.name() == *segment)?;

            if i == components.len() - 1 {
                return Some(found);
            }

            match found {
                BookmarkNode::Folder { content, .. } => {
                    current_fs = content;
                }
                BookmarkNode::Note { .. } => return None,
            }
        }

        None
    }

    fn get_by_path(&self, path: &Path) -> Option<&BookmarkNode> {
        let components = components_of(path);

        if components.is_empty() {
            return None;
        }

        let mut current_fs = self;

        for (i, segment) in components.iter().enumerate() {
            let found = current_fs.iter().find(|item| item.name() == *segment)?;

            if i == components.len() - 1 {
                return Some(found);
            }

            match found {
                BookmarkNode::Folder { content, .. } => {
                    current_fs = content;
                }
                BookmarkNode::Note { .. } => return None,
            }
        }

        None
    }

    fn get_mut_parent_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode> {
        let parent_path = path.parent()?;

        // If path has no parent component (e.g. "file.txt"), the parent is the root vector itself,
        // which isn't a single `Bookmark::Folder`.
        if parent_path.as_os_str().is_empty() {
            return None;
        }

        let parent = self.get_mut_by_path(parent_path)?;
        match parent {
            folder @ BookmarkNode::Folder { .. } => Some(folder),
            BookmarkNode::Note { .. } => None,
        }
    }

    fn get_parent_by_path(&self, path: &Path) -> Option<&BookmarkNode> {
        todo!();
    }

    fn remove_by_path(&mut self, path: &Path) -> Option<BookmarkNode> {
        let components = components_of(path);

        if components.is_empty() {
            return None;
        }

        let mut current = self;
        for (i, segment) in components.iter().enumerate() {
            let idx = current.iter().position(|item| item.name() == *segment);
            if let Some(idx) = idx {
                if i == components.len() - 1 {
                    return Some(current.remove(idx));
                }
                let folder = &mut current[idx];
                if let BookmarkNode::Folder { content, .. } = folder {
                    current = content;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        None
    }
    fn move_to(&mut self, from: &Path, to_dir: &Path) -> bool {
        if to_dir == from || to_dir.starts_with(from) {
            return false;
        }
        // Validate the target folder before removing the source so a failed
        // move never loses a node.
        if !is_folder(self, to_dir) {
            return false;
        }
        let Some(node) = self.remove_by_path(from) else {
            return false;
        };
        self.insert_into(to_dir, node)
    }
    fn insert_into(&mut self, dir: &Path, node: BookmarkNode) -> bool {
        let mut current = self;
        for segment in components_of(dir) {
            let idx = current.iter().position(|item| item.name() == segment);
            if let Some(idx) = idx {
                let folder = &mut current[idx];
                if let BookmarkNode::Folder { content, .. } = folder {
                    current = content;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        current.push(node);
        true
    }
}

/// Splits `path` into its normal components (the root, `.` and `..` are skipped).
fn components_of(path: &Path) -> Vec<&str> {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(os_str) => os_str.to_str(),
            _ => None,
        })
        .collect()
}

/// Returns `true` if `path` refers to a folder bookmark (or is the root path).
fn is_folder(root: &mut BookmarkFS, path: &Path) -> bool {
    if components_of(path).is_empty() {
        return true;
    }
    match root.get_mut_by_path(path) {
        Some(BookmarkNode::Folder { .. }) => true,
        _ => false,
    }
}
