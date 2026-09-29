use std::path::{Path, PathBuf};

use onyx::models::{BookmarkFS, BookmarkFSExt, BookmarkNode};

fn note(name: &str, path: &str) -> BookmarkNode {
    BookmarkNode::Note {
        path: PathBuf::from(path),
        name: String::from(name),
    }
}

fn folder(name: &str, content: BookmarkFS) -> BookmarkNode {
    BookmarkNode::Folder {
        name: String::from(name),
        content,
    }
}

/// A small tree used by most tests:
/// ```text
/// a (note)
/// f (folder)
/// ├── b (note)
/// └── g (folder)
///     └── c (note)
/// ```
fn sample() -> BookmarkFS {
    vec![
        note("a", "/tmp/a"),
        folder(
            "f",
            vec![note("b", "/tmp/b"), folder("g", vec![note("c", "/tmp/c")])],
        ),
    ]
}

#[test]
fn test_get_by_path() {
    let mut root = sample();

    assert!(root.get_by_path(Path::new("/a")).is_some());
    assert!(root.get_by_path(Path::new("/f/b")).is_some());
    assert!(root.get_by_path(Path::new("/f/g/c")).is_some());
    assert!(root.get_by_path(Path::new("/nope")).is_none());
    // The root itself is the vector, not a node.
    assert!(root.get_by_path(Path::new("/")).is_none());
}

#[test]
fn test_insert_into_root() {
    let mut root = sample();

    assert!(root.insert_into(Path::new("/"), note("d", "/tmp/d")));
    assert_eq!(3, root.len());
    assert!(root.get_by_path(Path::new("/d")).is_some());
}

#[test]
fn test_insert_into_folder() {
    let mut root = sample();

    assert!(root.insert_into(Path::new("/f"), note("d", "/tmp/d")));
    assert!(root.get_by_path(Path::new("/f/d")).is_some());
    assert!(root.get_by_path(Path::new("/f/g/d")).is_none());
}

#[test]
fn test_insert_into_missing_folder_fails() {
    let mut root = sample();

    assert!(!root.insert_into(Path::new("/nope"), note("d", "/tmp/d")));
    assert_eq!(2, root.len());
}

#[test]
fn test_insert_into_note_fails() {
    let mut root = sample();

    // `/a` is a note, not a folder.
    assert!(!root.insert_into(Path::new("/a"), note("d", "/tmp/d")));
    assert_eq!(2, root.len());
}

#[test]
fn test_remove_by_path() {
    let mut root = sample();

    let removed = root.remove_by_path(Path::new("/f/b"));
    assert!(removed.is_some());
    assert_eq!("b", removed.unwrap().name());
    assert!(root.get_by_path(Path::new("/f/b")).is_none());
    // The sibling folder is untouched.
    assert!(root.get_by_path(Path::new("/f/g/c")).is_some());
}

#[test]
fn test_remove_missing() {
    let mut root = sample();

    assert!(root.remove_by_path(Path::new("/nope")).is_none());
    assert!(root.remove_by_path(Path::new("/")).is_none());
    assert_eq!(2, root.len());
}

#[test]
fn test_move_to_folder() {
    let mut root = sample();

    assert!(root.move_to(Path::new("/a"), Path::new("/f")));
    assert!(root.get_by_path(Path::new("/a")).is_none());
    assert!(root.get_by_path(Path::new("/f/a")).is_some());
}

#[test]
fn test_move_into_own_subtree_fails() {
    let mut root = sample();

    assert!(!root.move_to(Path::new("/f"), Path::new("/f/g")));
    assert!(root.get_by_path(Path::new("/f/g/c")).is_some());
}

#[test]
fn test_move_onto_itself_fails() {
    let mut root = sample();

    assert!(!root.move_to(Path::new("/f"), Path::new("/f")));
    assert!(root.get_by_path(Path::new("/f/g/c")).is_some());
}

#[test]
fn test_move_to_note_fails() {
    let mut root = sample();

    // `/a` is a note, so the move must not happen and `/f/b` must survive.
    assert!(!root.move_to(Path::new("/f/b"), Path::new("/a")));
    assert!(root.get_by_path(Path::new("/f/b")).is_some());
}

#[test]
fn test_move_missing_source_fails() {
    let mut root = sample();

    assert!(!root.move_to(Path::new("/nope"), Path::new("/f")));
    assert_eq!(2, root.len());
}
