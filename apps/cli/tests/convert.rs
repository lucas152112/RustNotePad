use std::error::Error;
use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn convert_single_file_to_utf16_le() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let input = dir.path().join("input.txt");
    fs::write(&input, "Hello\n")?;
    let output = dir.path().join("output.txt");

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "convert",
            input.to_str().unwrap(),
            "--to",
            "utf16le",
            "--line-ending",
            "crlf",
            "--bom",
            "true",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let bytes = fs::read(&output)?;
    assert_eq!(&bytes[..2], b"\xFF\xFE");
    let units: Vec<u16> = bytes[2..]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    let text = String::from_utf16(&units)?;
    assert_eq!(text, "Hello\r\n");

    Ok(())
}

#[test]
fn convert_fails_when_from_encoding_mismatches_detection() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let input = dir.path().join("notes.txt");
    fs::write(&input, "Plain UTF-8")?;
    let output = dir.path().join("notes-converted.txt");

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "convert",
            input.to_str().unwrap(),
            "--from",
            "utf16le",
            "--to",
            "utf16le",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("detected as Utf8"));

    Ok(())
}
