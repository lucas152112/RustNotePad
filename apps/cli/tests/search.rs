use std::error::Error;
use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn search_reports_matches_across_files() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let file_one = dir.path().join("one.txt");
    let file_two = dir.path().join("two.txt");
    fs::write(&file_one, "Needle in haystack\nAnother line")?;
    fs::write(&file_two, "no matches here\nneedle again")?;

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "search",
            "needle",
            file_one.to_str().unwrap(),
            file_two.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(format!(
                "Search \"needle\" (2 hits in 2 files)\n  {} (1 hits)",
                file_one.display()
            ))
            .and(predicate::str::contains(format!(
                "    Line 1 (Col 1): Needle in haystack"
            )))
            .and(predicate::str::contains(format!(
                "  {} (1 hits)",
                file_two.display()
            )))
            .and(predicate::str::contains("    Line 2 (Col 1): needle again")),
        );

    Ok(())
}

#[test]
fn search_replace_apply_overwrites_files() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let file = dir.path().join("example.txt");
    fs::write(&file, "hello world\nhello world\n")?;

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "search",
            "world",
            file.to_str().unwrap(),
            "--replace",
            "Rust",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Applied 2 replacements"));

    let contents = fs::read_to_string(&file)?;
    assert_eq!(contents, "hello Rust\nhello Rust\n");

    Ok(())
}
