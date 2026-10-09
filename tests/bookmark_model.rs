use std::path::{Path, PathBuf};

use onyx::models::bookmark::*;

fn note(name: &str, path: &str) -> BookmarkNode {
    BookmarkNode::Bookmark {
        note_path: PathBuf::from(path),
        name: String::from(name),
    }
}

fn folder(name: &str, content: BookmarkFs) -> BookmarkNode {
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
fn sample() -> BookmarkFs {
    BookmarkFs::from(vec![
        note("a", "/tmp/a"),
        folder(
            "f",
            BookmarkFs::from(vec![
                note("b", "/tmp/b"),
                folder("g", BookmarkFs::from(vec![note("c", "/tmp/c")])),
            ]),
        ),
    ])
}

#[test]
fn test_get_by_path() {
    let mut root = sample();

    assert!(root.get_mut_node(Path::new("/a")).is_ok());
    assert!(root.get_mut_node(Path::new("/f/b")).is_ok());
    assert!(root.get_mut_node(Path::new("/f/g/c")).is_ok());
    assert!(root.get_mut_node(Path::new("/nope")).is_err());
    // The root itself is the vector, not a node.
    assert!(root.get_mut_node(Path::new("/")).is_err());
}

#[test]
fn test_insert_into_root() {
    let mut root = sample();

    assert!(
        root.insert_node_into(note("d", "/tmp/d"), Path::new("/"))
            .is_ok()
    );
    assert_eq!(3, root.nodes.len());
    assert!(root.get_mut_node(Path::new("/d")).is_ok());
}

#[test]
fn test_insert_into_folder() {
    let mut root = sample();

    assert!(
        root.insert_node_into(note("d", "/tmp/d"), Path::new("/f"))
            .is_ok()
    );
    assert!(root.get_mut_node(Path::new("/f/d")).is_ok());
    assert!(root.get_mut_node(Path::new("/f/g/d")).is_err());
}

#[test]
fn test_insert_into_missing_folder_fails() {
    let mut root = sample();

    assert!(
        root.insert_node_into(note("d", "/tmp/d"), Path::new("/nope"))
            .is_err()
    );
    assert_eq!(2, root.nodes.len());
}

#[test]
fn test_insert_into_note_fails() {
    let mut root = sample();

    // `/a` is a note, not a folder.
    assert!(
        root.insert_node_into(note("d", "/tmp/d"), Path::new("/a"))
            .is_err()
    );
    assert_eq!(2, root.nodes.len());
}

#[test]
fn test_remove_by_path() {
    let mut root = sample();

    let removed = root.remove_node(Path::new("/f/b"));
    assert!(removed.is_ok());
    assert_eq!("b", removed.unwrap().name());
    assert!(root.get_mut_node(Path::new("/f/b")).is_err());
    // The sibling folder is untouched.
    assert!(root.get_mut_node(Path::new("/f/g/c")).is_ok());
}

#[test]
fn test_remove_missing() {
    let mut root = sample();

    assert!(root.remove_node(Path::new("/nope")).is_err());
    assert!(root.remove_node(Path::new("/")).is_err());
    assert_eq!(2, root.nodes.len());
}

#[test]
fn test_move_to_folder() {
    let mut root = sample();

    assert!(root.move_to_dir(Path::new("/a"), Path::new("/f")).is_ok());
    assert!(root.get_mut_node(Path::new("/a")).is_err());
    assert!(root.get_mut_node(Path::new("/f/a")).is_ok());
}

#[test]
fn test_move_into_own_subtree_fails() {
    let mut root = sample();

    assert!(
        root.move_to_dir(Path::new("/f"), Path::new("/f/g"))
            .is_err()
    );
    assert!(root.get_mut_node(Path::new("/f/g/c")).is_ok());
}

#[test]
fn test_move_onto_itself_fails() {
    let mut root = sample();

    assert!(root.move_to_dir(Path::new("/f"), Path::new("/f")).is_err());
    assert!(root.get_mut_node(Path::new("/f/g/c")).is_ok());
}

#[test]
fn test_move_to_note_fails() {
    let mut root = sample();

    // `/a` is a note, so the move must not happen and `/f/b` must survive.
    assert!(
        root.move_to_dir(Path::new("/f/b"), Path::new("/a"))
            .is_err()
    );
    assert!(root.get_mut_node(Path::new("/f/b")).is_ok());
}

#[test]
fn test_move_missing_source_fails() {
    let mut root = sample();

    assert!(
        root.move_to_dir(Path::new("/nope"), Path::new("/f"))
            .is_err()
    );
    assert_eq!(2, root.nodes.len());
}
