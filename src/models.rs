use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub enum Bookmark {
    Note {
        /// Relative path from the volume root
        path: PathBuf,
        name: String,
    },
    Folder {
        name: String,
        content: Vec<Bookmark>,
    },
}
