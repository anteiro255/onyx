// Strings
pub const WINDOW_NAME: &str = "Onyx";

// Paths
pub const VOLUME_STORAGE_FOLDER: &str = "./.onyx_volume/";

// Durations
use std::time::Duration;

pub const VOLUME_STORAGE_AUTOSAVE_INTERVAL: Duration = Duration::from_secs(5);

// Combinations
use eframe::egui::{Key, KeyboardShortcut, Modifiers};

pub const DELETE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
pub const RENAME_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F2);
pub const UNDO_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Z);
pub const REDO_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Z);
