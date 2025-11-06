use std::fs;

use rustnotepad_settings::{ThemeDefinition, ThemeKind, ThemeLoadError};
use tempfile::tempdir;

const SAMPLE_TMTHEME: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>name</key>
    <string>Sample tmTheme</string>
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
                <key>selection</key>
                <string>#3E4451</string>
            </dict>
        </dict>
    </array>
</dict>
</plist>
"#;

#[test]
fn imports_tmtheme_palette() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("sample.tmTheme");
    fs::write(&path, SAMPLE_TMTHEME).expect("write tmTheme");

    let theme = ThemeDefinition::from_tmtheme_file(&path).expect("import");
    assert_eq!(theme.name, "Sample tmTheme");
    assert_eq!(theme.kind, ThemeKind::Dark);
    assert_eq!(theme.palette.editor_background, "#282C34");
    assert_eq!(theme.palette.editor_text, "#DCDCDC");
    assert_eq!(theme.palette.accent, "#3E4451");
    assert_eq!(theme.palette.status_bar, "#61AFEF");
}

#[test]
fn missing_name_uses_filename_fallback() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("fallback.tmTheme");
    fs::write(
        &path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>settings</key>
    <array>
        <dict>
            <key>settings</key>
            <dict>
                <key>foreground</key>
                <string>#FFFFFF</string>
                <key>background</key>
                <string>#101010</string>
            </dict>
        </dict>
    </array>
</dict>
</plist>
"#,
    )
    .expect("write tmTheme");

    let theme = ThemeDefinition::from_tmtheme_file(&path).expect("import");
    assert_eq!(theme.name, "fallback");
}

#[test]
fn error_when_missing_base_settings() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("invalid.tmTheme");
    fs::write(
        &path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>name</key>
    <string>Broken</string>
    <key>settings</key>
    <array>
        <dict>
            <key>scope</key>
            <string>source.rust</string>
            <key>settings</key>
            <dict>
                <key>foreground</key>
                <string>#FFFFFF</string>
            </dict>
        </dict>
    </array>
</dict>
</plist>
"#,
    )
    .expect("write tmTheme");

    let error = ThemeDefinition::from_tmtheme_file(&path).unwrap_err();
    match error {
        ThemeLoadError::InvalidFormat(message) => {
            assert!(message.contains("base settings"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
