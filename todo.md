### Left panel

#### Files

- [ ] Add Ctrl+Shift+Z(redo) support

#### Bookmarks

- [ ] Instead of saving all the bookmark nodes to `./.onyx_volume/bookmarks`, save them as fs to `./.onyx_volume/bookmarks/`, that is, sort of: `./.onyx_volume/bookmarks/<bookmark_folder1>/<bookmark_folder2>/<bookmark_name>.bookmark.ron`

- [x] Add Ctrl+Z, Ctrl+Shift+Z(redo) support:
  - (tmp): fix the bugs:
    - renaming the moved file to the folder name when undoing a moving action
      - unworling Ctrl+Shift+Z
      - Collapsing entries with the same names(even if they're in different folders)
      - Creating new files in some folder instead of in the root
      - Eliminating bookmarks on moving to the root or just not moving them anywhere

##### Future

- [ ] Add Ctrl+X, Ctrl+C, Ctrl+V support(may be excess)
- [ ] Use HashSets instead of Vectors for bookmark folders
- [ ] Refactor `models/bookmark.rs`:
  - [ ] Implement error types
  - [ ] Make `BookmarkFS` a struct instead of an alias with BookmarkFS

### General

#### Logs

- Implement gui error displaying
