use eframe::egui::TextBuffer;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BookmarkNode {
    Note {
        /// Real relative path from the volume root
        note_path: PathBuf,
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

    pub fn is_note_bookmark(&self) -> bool {
        match self {
            BookmarkNode::Note { .. } => true,
            BookmarkNode::Folder { .. } => false,
        }
    }
}

pub trait BookmarkFSExt {
    /// Recursively retrieves a mutable reference to a `Bookmark` by traversing the given relative path.
    fn get_mut_bookmark(&mut self, path: &Path) -> Option<&mut BookmarkNode>;

    /// Recursively retrieves a reference to a `Bookmark` by traversing the given relative path.
    fn get_bookmark(&self, path: &Path) -> Option<&BookmarkNode>;

    /// Retrieves a mutable reference to the parent `Bookmark` (which must be a `Folder`) for the given path.
    fn get_mut_bookmark_parent(&mut self, path: &Path) -> Option<&mut BookmarkNode>;

    /// Retrieves a reference to the parent `Bookmark` (which must be a `Folder`) for the given path.
    fn get_bookmark_parent(&self, path: &Path) -> Option<&BookmarkNode>;

    /// Removes and returns the bookmark node at `path`, or `None` if it doesn't exist.
    fn remove_bookmark(&mut self, path: &Path) -> Option<BookmarkNode>;

    /// Moves the node at `from` into the folder at `to_dir` (the root if `to_dir` is the root path).
    /// Returns `false` if `from` doesn't exist, `to_dir` isn't a folder, or `to_dir` is inside `from`.
    fn move_to_dir(&mut self, from: impl AsRef<Path>, to_dir: impl AsRef<Path>) -> bool;

    fn move_bookmark(&mut self, from: &Path, to: &Path) -> bool;

    /// Inserts `node` into the folder at `dir` (the root if `dir` is the root path).
    /// Returns `false` if the target folder doesn't exist.
    fn insert_bookmark_into(&mut self, dir: &Path, node: BookmarkNode) -> bool;
}

impl BookmarkFSExt for BookmarkFS {
    fn get_mut_bookmark(&mut self, path: &Path) -> Option<&mut BookmarkNode> {
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

    fn get_bookmark(&self, path: &Path) -> Option<&BookmarkNode> {
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

    fn get_mut_bookmark_parent(&mut self, path: &Path) -> Option<&mut BookmarkNode> {
        let parent_path = path.parent()?;

        // If path has no parent component (e.g. "file.txt"), the parent is the root vector itself,
        // which isn't a single `Bookmark::Folder`.
        if parent_path.as_os_str().is_empty() {
            return None;
        }

        let parent = self.get_mut_bookmark(parent_path)?;
        match parent {
            folder @ BookmarkNode::Folder { .. } => Some(folder),
            BookmarkNode::Note { .. } => None,
        }
    }

    fn get_bookmark_parent(&self, path: &Path) -> Option<&BookmarkNode> {
        todo!();
    }

    fn remove_bookmark(&mut self, path: &Path) -> Option<BookmarkNode> {
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
    fn move_bookmark(&mut self, from: &Path, to: &Path) -> bool {
        // Prevent moving into own subtree or onto self
        if to.starts_with(from) {
            return false;
        }

        // Validate that the target parent exists and is a folder (or root)
        let Some(to_parent) = to.parent() else {
            return false;
        };
        let parent_components = components_of(to_parent);
        if !parent_components.is_empty() {
            let target = self.get_bookmark(to_parent);
            match target {
                Some(BookmarkNode::Folder { .. }) => {}
                _ => return false,
            }
        }

        let Some(mut bookmark) = self.remove_bookmark(from) else {
            return false;
        };
        let Some(to_name) = to.file_name() else {
            return false;
        };
        bookmark.set_name(to_name.to_string_lossy().as_str().to_owned());

        if !self.insert_bookmark_into(to_parent, bookmark.clone()) {
            let original_parent = from.parent().unwrap_or_else(|| Path::new("/"));
            let Some(original_name) = from.file_name() else {
                return false;
            };
            bookmark.set_name(original_name.to_string_lossy().as_str().to_owned());
            self.insert_bookmark_into(original_parent, bookmark);
            false
        } else {
            true
        }
    }

    fn move_to_dir(&mut self, from: impl AsRef<Path>, to_dir: impl AsRef<Path>) -> bool {
        let from = from.as_ref();
        let to_dir = to_dir.as_ref();

        // Prevent moving into own subtree or onto self
        if to_dir.starts_with(from) {
            return false;
        }

        // Validate that the target exists and is a folder (or root)
        let components = components_of(to_dir);
        if !components.is_empty() {
            let target = self.get_bookmark(to_dir);
            match target {
                Some(BookmarkNode::Folder { .. }) => {}
                _ => return false,
            }
        }

        let Some(bookmark) = self.remove_bookmark(from) else {
            return false;
        };

        if !self.insert_bookmark_into(to_dir, bookmark.clone()) {
            let original_parent = from.parent().unwrap_or_else(|| Path::new("/"));
            self.insert_bookmark_into(original_parent, bookmark);
            false
        } else {
            true
        }
    }

    fn insert_bookmark_into(&mut self, dir: &Path, node: BookmarkNode) -> bool {
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
    match root.get_mut_bookmark(path) {
        Some(BookmarkNode::Folder { .. }) => true,
        _ => false,
    }
}
