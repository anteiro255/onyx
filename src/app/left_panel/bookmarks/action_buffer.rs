use std::path::PathBuf;

use crate::models::bookmark;

pub enum Action {
    Create { path: PathBuf },
    Rename { from: PathBuf, to: PathBuf },
    Move { from: PathBuf, to: PathBuf },
    Delete { bookmark: bookmark::BookmarkNode },
}

#[derive(Default)]
pub struct ActionBuffer {
    buf: Vec<Action>,
}

impl ActionBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn undo(&mut self) {}
}
