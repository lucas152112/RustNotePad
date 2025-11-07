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

const SAMPLE_NOTEPAD_XML: &str = r#"<?xml version="1.0"?>
<NotepadPlus name="Solarized Light">
    <GlobalStyles>
        <WidgetStyle name="Default Style" fgColor="002B36" bgColor="FDF6E3" />
        <WidgetStyle name="Selected text colour" bgColor="586E75" />
        <WidgetStyle name="Caret colour" fgColor="DC322F" />
    </GlobalStyles>
</NotepadPlus>
"#;

const SAMPLE_SUBLIME_SCHEME: &str = r##"
{
    "name": "Mariana",
    "author": "Sublime HQ",
    "variables": {
        "accent": "#6699CC"
    },
    "globals": {
        "background": "#1F2430",
        "foreground": "#F8F8F2",
        "caret": "#FFFFFF",
        "selection": "var(accent)"
    },
    "rules": [
        { "scope": "keyword.control", "foreground": "#C5A5C5", "font_style": "bold" },
        { "scope": "string.quoted", "foreground": "#8DC891" },
        { "scope": "comment.line", "foreground": "#5C6773", "font_style": "italic" }
    ]
}
"##;

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

#[test]
fn imports_notepad_xml_theme() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("stylers.xml");
    fs::write(&path, SAMPLE_NOTEPAD_XML).expect("write xml");

    let theme = ThemeDefinition::from_notepad_xml(&path).expect("import");
    assert_eq!(theme.name, "Solarized Light");
    assert_eq!(theme.palette.editor_background, "#FDF6E3");
    assert_eq!(theme.palette.editor_text, "#002B36");
    assert_eq!(theme.palette.accent, "#586E75");
    assert_eq!(theme.palette.status_bar, "#DC322F");
    assert_eq!(ThemeDefinition::slug_for(&theme.name), "solarized-light");
}

#[test]
fn imports_sublime_color_scheme() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("mariana.sublime-color-scheme");
    fs::write(&path, SAMPLE_SUBLIME_SCHEME).expect("write scheme");

    let theme = ThemeDefinition::from_sublime_color_scheme(&path).expect("import");
    assert_eq!(theme.name, "Mariana");
    assert_eq!(theme.palette.accent, "#6699CC");
    assert!(theme.syntax_palette().is_some());
    let json = theme.to_json_string();
    assert!(json.contains("\"name\""));
}
