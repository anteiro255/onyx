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
    /// Recursively retrieves a reference to a `Bookmark` by traversing the given relative path.
    fn get_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode>;

    /// Retrieves a reference to the parent `Bookmark` (which must be a `Folder`) for the given path.
    fn get_parent_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode>;
}

impl BookmarkFSExt for BookmarkFS {
    fn get_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode> {
        let components: Vec<_> = path
            .components()
            .filter_map(|c| match c {
                Component::Normal(os_str) => os_str.to_str(),
                _ => None,
            })
            .collect();

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
    fn get_parent_by_path(&mut self, path: &Path) -> Option<&mut BookmarkNode> {
        let parent_path = path.parent()?;

        // If path has no parent component (e.g. "file.txt"), the parent is the root vector itself,
        // which isn't a single `Bookmark::Folder`.
        if parent_path.as_os_str().is_empty() {
            return None;
        }

        let parent = self.get_by_path(parent_path)?;
        match parent {
            folder @ BookmarkNode::Folder { .. } => Some(folder),
            BookmarkNode::Note { .. } => None,
        }
    }
}
