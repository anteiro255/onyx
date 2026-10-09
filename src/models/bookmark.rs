use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Component, Path, PathBuf},
};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BookmarkNode {
    Bookmark {
        /// Real relative path from the volume root
        note_path: PathBuf,
        name: String,
    },
    Folder {
        name: String,
        content: BookmarkFs,
    },
}
impl BookmarkNode {
    /// Returns the name of the bookmark item regardless of its variant.
    pub fn name(&self) -> &str {
        match self {
            BookmarkNode::Bookmark { name, .. } => name,
            BookmarkNode::Folder { name, .. } => name,
        }
    }
    /// Like .name() but "/" is added to the back for folders
    pub fn fs_name<'a>(&'a self) -> Cow<'a, str> {
        match self {
            BookmarkNode::Bookmark { name, .. } => Cow::Borrowed(name),
            BookmarkNode::Folder { name, .. } => Cow::Owned(format!("{}/", name)),
        }
    }

    pub fn set_name(&mut self, new_name: String) -> &str {
        match self {
            BookmarkNode::Bookmark { name, .. } => {
                *name = new_name;
                name.as_str()
            }
            BookmarkNode::Folder { name, .. } => {
                *name = new_name;
                name.as_str()
            }
        }
    }

    pub fn is_bookmark(&self) -> bool {
        match self {
            BookmarkNode::Bookmark { .. } => true,
            BookmarkNode::Folder { .. } => false,
        }
    }
    pub fn is_folder(&self) -> bool {
        !self.is_bookmark()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BookmarkFsError {
    #[error("No bookmark node exists at path \"{}\"", .0.display())]
    NoBookmarkNode(PathBuf),
    #[error("The folder can't be moved from \"{}\" inside itself in \"{}\"", .from.display(), .to.display())]
    CannotMoveFolderInsideItself { from: PathBuf, to: PathBuf },
    #[error("The node at path \"{}\" have no parent", .0.display())]
    NodeHasNoParent(PathBuf),
    #[error("Can't get the name of the node \"{}\"", .0.display())]
    CannotGetNameOfNode(PathBuf),
    #[error("Some components of the path \"{}\" don't exist or some components except of the last are not folders", .0.display())]
    NoSuchPath(PathBuf),
    #[error("The path \"{}\" has no components", .0.display())]
    PathHasNoComponents(PathBuf),
}
impl BookmarkFsError {
    pub fn log(&self) {
        log::error!("{}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BookmarkFs {
    pub nodes: Vec<BookmarkNode>,
}

impl BookmarkFs {
    /// Recursively retrieves a mutable reference to a `Bookmark` by traversing the given relative path.
    /// Returns None if the there's no such path
    pub fn get_mut_node(&mut self, path: &Path) -> Result<&mut BookmarkNode, BookmarkFsError> {
        let components = components_of(path);

        if components.is_empty() {
            return Err(BookmarkFsError::PathHasNoComponents(path.to_owned()));
        }

        let mut current_fs = &mut self.nodes;

        for (i, segment) in components.iter().enumerate() {
            let Some(found) = current_fs.iter_mut().find(|item| item.name() == *segment) else {
                return Err(BookmarkFsError::NoSuchPath(path.to_owned()));
            };

            if i == components.len() - 1 {
                return Ok(found);
            }

            match found {
                BookmarkNode::Folder { content, .. } => {
                    current_fs = &mut content.nodes;
                }
                BookmarkNode::Bookmark { .. } => {
                    return Err(BookmarkFsError::NoSuchPath(path.to_owned()));
                }
            }
        }

        Err(BookmarkFsError::NoSuchPath(path.to_owned()))
    }

    /// Recursively retrieves a reference to a `Bookmark` by traversing the given relative path.
    pub fn get_node(&self, path: &Path) -> Result<&BookmarkNode, BookmarkFsError> {
        let components = components_of(path);

        if components.is_empty() {
            return Err(BookmarkFsError::PathHasNoComponents(path.to_owned()));
        }

        let mut current_fs = &self.nodes;

        for (i, segment) in components.iter().enumerate() {
            let Some(found) = current_fs.iter().find(|item| item.name() == *segment) else {
                return Err(BookmarkFsError::NoSuchPath(path.to_owned()));
            };

            if i == components.len() - 1 {
                return Ok(found);
            }

            match found {
                BookmarkNode::Folder { content, .. } => {
                    current_fs = &content.nodes;
                }
                BookmarkNode::Bookmark { .. } => {
                    return Err(BookmarkFsError::NoSuchPath(path.to_owned()));
                }
            }
        }

        Err(BookmarkFsError::NoSuchPath(path.to_owned()))
    }

    /// Retrieves a reference to the parent `Bookmark` (which must be a `Folder`) for the given path.
    pub fn get_mut_node_parent(
        &mut self,
        path: &Path,
    ) -> Result<&mut BookmarkNode, BookmarkFsError> {
        let Some(parent_path) = path.parent() else {
            return Err(BookmarkFsError::NodeHasNoParent(path.to_owned()));
        };

        if parent_path.as_os_str().is_empty() {
            return Err(BookmarkFsError::NodeHasNoParent(path.to_owned()));
        }

        let parent = self.get_mut_node(parent_path)?;
        match parent {
            folder @ BookmarkNode::Folder { .. } => Ok(folder),

            BookmarkNode::Bookmark { .. } => {
                // If self.get_mut_node(parent_path) returned a bookmark instead of folder,
                // it means that there's not a note at the path `path` at all because
                // the note can't be inside a note, it can be only inside a folder
                Err(BookmarkFsError::NoSuchPath(path.to_owned()))
            }
        }
    }

    /// Removes and returns the bookmark node at `path`
    pub fn remove_node(&mut self, path: &Path) -> Result<BookmarkNode, BookmarkFsError> {
        let components = components_of(path);

        if components.is_empty() {
            return Err(BookmarkFsError::NoSuchPath(path.to_owned()));
        }

        let mut current = &mut self.nodes;
        for (i, segment) in components.iter().enumerate() {
            let idx = current.iter().position(|item| item.name() == *segment);
            if let Some(idx) = idx {
                if i == components.len() - 1 {
                    return Ok(current.remove(idx));
                }
                let folder = &mut current[idx];
                if let BookmarkNode::Folder { content, .. } = folder {
                    current = &mut content.nodes;
                } else {
                    return Err(BookmarkFsError::NoSuchPath(path.to_owned()));
                }
            } else {
                return Err(BookmarkFsError::NoSuchPath(path.to_owned()));
            }
        }
        Err(BookmarkFsError::NoSuchPath(path.to_owned()))
    }

    /// Moves or renames a bookmark, like bash `mv`.
    /// Returns `false` and leaves the tree unchanged if the move is invalid.
    pub fn move_bookmark(
        &mut self,
        old_path: &Path,
        new_path: &Path,
    ) -> Result<(), BookmarkFsError> {
        // Reject moving onto itself or into its own subtree.
        if new_path.starts_with(old_path) {
            return Err(BookmarkFsError::CannotMoveFolderInsideItself {
                from: old_path.to_owned(),
                to: new_path.to_owned(),
            });
        }

        let Some(old_parent) = old_path.parent() else {
            return Err(BookmarkFsError::NodeHasNoParent(old_path.to_owned()));
        };
        let Some(to_parent) = new_path.parent() else {
            return Err(BookmarkFsError::NodeHasNoParent(new_path.to_owned()));
        };
        let Some(to_name) = new_path.file_name().and_then(|n| n.to_str()) else {
            return Err(BookmarkFsError::CannotGetNameOfNode(new_path.to_owned()));
        };

        let node = self.remove_node(old_path)?;

        // Insert a renamed copy. Keep the original so we can put it back if insertion fails.
        let mut moved = node.clone();
        moved.set_name(to_name.to_owned());

        match self.insert_node_into(moved, to_parent) {
            Err(err) => {
                // Inserting to the new path failure: restore the original node under its original path.
                let _ = self.insert_node_into(node, old_parent);
                Err(err)
            }
            Ok(()) => Ok(()),
        }
    }

    /// Moves a bookmark into the given directory, keeping its file name.
    /// Returns an error and leaves the tree unchanged if the move is invalid.
    pub fn move_to_dir(
        &mut self,
        from_path: impl AsRef<Path>,
        to_dir: impl AsRef<Path>,
    ) -> Result<(), BookmarkFsError> {
        let from_path = from_path.as_ref();
        let to_dir = to_dir.as_ref();

        // Reject moving onto itself or into its own subtree.
        if to_dir.starts_with(from_path) {
            return Err(BookmarkFsError::CannotMoveFolderInsideItself {
                from: from_path.to_owned(),
                to: to_dir.to_owned(),
            });
        }

        let Some(original_parent) = from_path.parent() else {
            return Err(BookmarkFsError::NodeHasNoParent(from_path.to_owned()));
        };

        let node = self.remove_node(from_path)?;

        // Insert into the new directory. Keep the original so we can put it back if insertion fails.
        match self.insert_node_into(node.clone(), to_dir) {
            Err(err) => {
                // Inserting to the new path failed: restore the original node under its original path.
                let _ = self.insert_node_into(node, original_parent);
                Err(err)
            }
            Ok(()) => Ok(()),
        }
    }

    pub fn insert_node_into(
        &mut self,
        node: BookmarkNode,
        dir: &Path,
    ) -> Result<(), BookmarkFsError> {
        let mut current = &mut self.nodes;
        for segment in components_of(dir) {
            let idx = current.iter().position(|item| item.name() == segment);
            if let Some(idx) = idx {
                let folder = &mut current[idx];
                if let BookmarkNode::Folder { content, .. } = folder {
                    current = &mut content.nodes;
                } else {
                    return Err(BookmarkFsError::NoSuchPath(dir.to_owned()));
                }
            } else {
                return Err(BookmarkFsError::NoSuchPath(dir.to_owned()));
            }
        }
        current.push(node);
        Ok(())
    }
}

impl From<Vec<BookmarkNode>> for BookmarkFs {
    fn from(value: Vec<BookmarkNode>) -> Self {
        Self { nodes: value }
    }
}

impl FromIterator<BookmarkNode> for BookmarkFs {
    fn from_iter<I: IntoIterator<Item = BookmarkNode>>(iter: I) -> Self {
        Self {
            nodes: iter.into_iter().collect(),
        }
    }
}
impl IntoIterator for BookmarkFs {
    type Item = BookmarkNode;
    type IntoIter = std::vec::IntoIter<BookmarkNode>;
    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}
impl<'a> IntoIterator for &'a BookmarkFs {
    type Item = &'a BookmarkNode;
    type IntoIter = std::slice::Iter<'a, BookmarkNode>;
    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter()
    }
}
impl<'a> IntoIterator for &'a mut BookmarkFs {
    type Item = &'a mut BookmarkNode;
    type IntoIter = std::slice::IterMut<'a, BookmarkNode>;
    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter_mut()
    }
}

impl AsRef<[BookmarkNode]> for BookmarkFs {
    fn as_ref(&self) -> &[BookmarkNode] {
        &self.nodes
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
