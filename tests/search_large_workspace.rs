use std::fs;

use rustnotepad_search::{search_in_files, FileSearchInput, SearchOptions};
use tempfile::tempdir;

#[test]
fn large_workspace_search_scales() {
    let dir = tempdir().expect("workspace dir");
    let mut inputs = Vec::new();

    for idx in 0..500usize {
        let path = dir.path().join(format!("file_{idx:04}.txt"));
        let contents = if idx % 50 == 0 {
            "needle here\nother text\n".to_string()
        } else {
            "other text\nstill other\n".to_string()
        };
        fs::write(&path, &contents).unwrap();
        inputs.push(FileSearchInput::new(path, contents));
    }

    let options = SearchOptions::new("needle");
    let report = search_in_files(inputs, &options).expect("search report");

    let summary = report.summary();
    assert_eq!(summary.files_with_matches, 10);
    assert_eq!(summary.total_matches, 10);
}
