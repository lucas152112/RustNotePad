use std::fs;
use std::path::PathBuf;

use rustnotepad_core::{BookmarkManager, Document, SearchSession};
use rustnotepad_search::{search_in_files, FileSearchInput, SearchOptions};
use tempfile::tempdir;

#[test]
fn end_to_end_search_and_replace_flow() {
    let dir = tempdir().expect("temp dir");
    let primary_path = dir.path().join("main.txt");
    let secondary_path = dir.path().join("notes.txt");

    fs::write(&primary_path, "alpha beta gamma beta").unwrap();
    fs::write(&secondary_path, "todo: beta\nbeta blockers\n").unwrap();

    let mut primary = Document::open(&primary_path).unwrap();
    let mut options = SearchOptions::new("beta");
    let mut session = SearchSession::new(options.clone()).unwrap();

    // Discover matches in the primary file.
    session.refresh(&primary).unwrap();
    assert_eq!(session.matches().len(), 2);

    // Mark results to bookmarks and ensure round-trip.
    let mut bookmarks = BookmarkManager::default();
    let marked_lines = session.mark_all(&mut bookmarks);
    assert_eq!(marked_lines, 1);
    assert!(bookmarks.is_bookmarked(1));
    session.clear_marks(&mut bookmarks);
    assert_eq!(bookmarks.len(), 0);

    // Replace a single occurrence and verify content.
    session.find_next();
    session.replace_current("delta", &mut primary).unwrap();
    assert_eq!(primary.contents(), "alpha delta gamma beta");

    // Replace the remaining occurrences in scope.
    let replaced = session.replace_all("omega", &mut primary).unwrap();
    assert_eq!(replaced, 1);
    assert_eq!(primary.contents(), "alpha delta gamma omega");

    // Persist the document so multi-file search sees latest contents.
    primary.save_as(&primary_path).unwrap();

    // Multi-file search gathers matches across the workspace.
    let report = search_in_files(
        [
            FileSearchInput::new(primary_path.clone(), fs::read_to_string(&primary_path).unwrap()),
            FileSearchInput::new(
                secondary_path.clone(),
                fs::read_to_string(&secondary_path).unwrap(),
            ),
        ],
        &options,
    )
    .unwrap();

    assert_eq!(report.summary().files_with_matches, 1);
    assert_eq!(report.total_matches, 2);
    let first_entry = &report.results[0];
    assert_eq!(
        first_entry
            .path
            .as_ref()
            .map(PathBuf::as_path)
            .unwrap(),
        secondary_path.as_path()
    );
}
