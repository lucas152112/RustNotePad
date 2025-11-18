use std::error::Error;
use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::tempdir;

const SAMPLE_TMTHEME: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>name</key>
    <string>CLI Theme</string>
    <key>settings</key>
    <array>
        <dict>
            <key>settings</key>
            <dict>
                <key>foreground</key>
                <string>#DCDCDC</string>
                <key>background</key>
                <string>#282C34</string>
                <key>caret</key>
                <string>#61AFEF</string>
            </dict>
        </dict>
    </array>
</dict>
</plist>
"#;

#[test]
fn localization_install_creates_locale_file() -> Result<(), Box<dyn Error>> {
    let workspace = tempdir()?;
    let source = workspace.path().join("locale.json");
    fs::write(
        &source,
        r#"{ "locale": "zz-AA", "strings": { "menu.file": "File" } }"#,
    )?;

    cli()?
        .args([
            "--workspace",
            workspace.path().to_str().unwrap(),
            "localization",
            "install",
            source.to_str().unwrap(),
        ])
        .assert()
        .success();

    let installed = workspace
        .path()
        .join(".rustnotepad")
        .join("langs")
        .join("zz-AA.json");
    assert!(installed.exists());
    let contents = fs::read_to_string(installed)?;
    assert!(contents.contains("\"menu.file\""));

    Ok(())
}

#[test]
fn themes_import_and_export_round_trip() -> Result<(), Box<dyn Error>> {
    let workspace = tempdir()?;
    let theme_path = workspace.path().join("import.tmTheme");
    fs::write(&theme_path, SAMPLE_TMTHEME)?;

    cli()?
        .args([
            "--workspace",
            workspace.path().to_str().unwrap(),
            "themes",
            "import",
            theme_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let imported = workspace
        .path()
        .join(".rustnotepad")
        .join("themes")
        .join("cli-theme.json");
    assert!(imported.exists());

    let export_path = workspace.path().join("theme-export.json");
    cli()?
        .args([
            "--workspace",
            workspace.path().to_str().unwrap(),
            "themes",
            "export",
            "--name",
            "CLI Theme",
            "--output",
            export_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let export_contents = fs::read_to_string(export_path)?;
    assert!(export_contents.contains("\"name\": \"CLI Theme\""));
    Ok(())
}

#[test]
fn preferences_import_and_export() -> Result<(), Box<dyn Error>> {
    let workspace = tempdir()?;
    let import_path = workspace.path().join("prefs.json");
    fs::write(
        &import_path,
        r#"{
            "version": 1,
            "editor": {
                "autosave_enabled": false,
                "autosave_interval_minutes": 10,
                "show_line_numbers": false,
                "highlight_active_line": false
            },
            "ui": {
                "locale": "en-US",
                "theme": "Notepad++ Classic"
            }
        }"#,
    )?;

    cli()?
        .args([
            "--workspace",
            workspace.path().to_str().unwrap(),
            "preferences",
            "import",
            import_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let export_path = workspace.path().join("prefs-export.json");
    cli()?
        .args([
            "--workspace",
            workspace.path().to_str().unwrap(),
            "preferences",
            "export",
            "--output",
            export_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let exported = fs::read_to_string(export_path)?;
    assert!(exported.contains("\"autosave_enabled\": false"));
    Ok(())
}

fn cli() -> Result<Command, Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("rustnotepad-cli")?;
    cmd.current_dir(repo_root());
    Ok(cmd)
}

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|dir| dir.parent())
        .expect("workspace root")
        .to_path_buf()
}
