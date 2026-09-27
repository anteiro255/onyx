use crate::models::BookmarkNode;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

#[derive(Serialize, Deserialize)]
pub struct Storage {
    pub bookmarks: Vec<BookmarkNode>,
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("I/O storage error: {0}")]
    Io(#[from] std::io::Error),

    #[error("RON deserialization error: {0}")]
    Ron(#[from] ron::error::SpannedError),

    #[error("RON serialization error: {0}")]
    RonSer(#[from] ron::Error),
}

impl Storage {
    pub fn new() -> Self {
        Self::load_from(crate::constants::VOLUME_STORAGE_FOLDER).unwrap_or(Self::default())
    }
    fn load_from(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let bookmarks = load(path.as_ref().join("bookmarks.ron").as_path())?;

        Ok(Self { bookmarks })
    }

    pub fn save(&self) {
        self.save_to(crate::constants::VOLUME_STORAGE_FOLDER);
    }
    fn save_to(&self, path: impl AsRef<Path>) {
        if let Err(err) = save(
            &self.bookmarks,
            path.as_ref().join("bookmarks.ron").as_path(),
        ) {
            log::error!("Saving bookmarks to the volume storage error: {}", err)
        };
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self { bookmarks: vec![] }
    }
}

fn load<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, StorageError> {
    let content = std::fs::read_to_string(path)?;
    let data = ron::from_str(&content)?;
    Ok(data)
}

fn save<T: serde::Serialize>(data: &T, path: impl AsRef<Path>) -> Result<(), StorageError> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = ron::to_string(data)?;
    std::fs::write(path, content)?;
    Ok(())
}
