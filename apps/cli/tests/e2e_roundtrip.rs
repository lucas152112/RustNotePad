use std::error::Error;
use std::fs;
use std::io::Read;

use assert_cmd::Command;
use tempfile::tempdir;

/// 驗證 CLI 轉檔流程，涵蓋 BOM / 行尾 / 雙語訊息。 /
/// Verifies end-to-end encoding and line-ending conversion via CLI (BOM + newline handling).
#[test]
fn convert_round_trip_with_bom_and_line_endings() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let source_path = dir.path().join("journal_utf8.txt");
    let target_path = dir.path().join("journal_utf16.txt");
    let reloaded_path = dir.path().join("journal_back.txt");

    let original = "第一行\nSecond Line\n第三行";
    fs::write(&source_path, original)?;

    // Step 1: convert UTF-8 -> UTF-16LE with BOM and Windows 行尾。 / force CRLF
    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "convert",
            source_path.to_str().unwrap(),
            "--to",
            "utf16le",
            "--line-ending",
            "crlf",
            "--bom",
            "true",
            "--output",
            target_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut encoded = Vec::new();
    fs::File::open(&target_path)?.read_to_end(&mut encoded)?;
    assert!(encoded.starts_with(&[0xFF, 0xFE]));

    // Step 2: convert back to UTF-8 with LF。/ ensure round-trip
    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "convert",
            target_path.to_str().unwrap(),
            "--from",
            "utf16le",
            "--to",
            "utf8",
            "--line-ending",
            "lf",
            "--output",
            reloaded_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut round_trip = fs::read_to_string(&reloaded_path)?;
    if let Some(stripped) = round_trip.strip_prefix('\u{feff}') {
        round_trip = stripped.to_string();
    }
    assert_eq!(round_trip, "第一行\nSecond Line\n第三行");

    Ok(())
}
