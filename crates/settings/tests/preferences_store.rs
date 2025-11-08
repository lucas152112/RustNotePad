use rustnotepad_settings::{Preferences, PreferencesStore};
use std::fs;
use tempfile::tempdir;

#[test]
fn load_missing_file_returns_defaults() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("preferences.json");

    let store = PreferencesStore::load(&path).expect("load defaults");
    assert!(store.preferences().editor.autosave_enabled);
    assert_eq!(store.preferences().editor.autosave_interval_minutes, 5);
    assert_eq!(store.preferences().ui.locale, "en-US");
    assert_eq!(store.preferences().ui.theme, "Midnight Indigo");
}

#[test]
fn save_and_reload_roundtrip() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("preferences.json");

    let mut store = PreferencesStore::new(path.clone(), Preferences::default());
    store
        .update(|prefs| {
            prefs.editor.autosave_enabled = false;
            prefs.editor.autosave_interval_minutes = 12;
            prefs.ui.locale = "zh-TW".to_string();
        })
        .expect("save");

    let reloaded = PreferencesStore::load(&path).expect("reload");
    assert!(!reloaded.preferences().editor.autosave_enabled);
    assert_eq!(reloaded.preferences().editor.autosave_interval_minutes, 12);
    assert_eq!(reloaded.preferences().ui.locale, "zh-TW");
}

#[test]
fn overwrite_replaces_existing_values() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("preferences.json");

    let mut store = PreferencesStore::load(&path).expect("default");
    let mut prefs = store.preferences().clone();
    prefs.editor.autosave_interval_minutes = 0;
    prefs.editor.show_line_numbers = false;
    prefs.ui.theme = String::new();

    store.overwrite(prefs).expect("overwrite");

    let current = store.preferences();
    assert_eq!(current.editor.autosave_interval_minutes, 5);
    assert!(!current.editor.show_line_numbers);
    assert_eq!(current.ui.theme, "Midnight Indigo");
}

#[test]
fn legacy_version_is_upgraded_on_load() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("preferences.json");
    fs::write(
        &path,
        r#"{
            "version": 0,
            "editor": {
                "autosave_enabled": false,
                "autosave_interval_minutes": 0,
                "show_line_numbers": false,
                "highlight_active_line": false
            },
            "ui": {
                "locale": "zh-TW",
                "theme": ""
            }
        }"#,
    )
    .expect("write legacy prefs");

    let store = PreferencesStore::load(&path).expect("load legacy file");
    let prefs = store.preferences();
    assert_eq!(
        prefs.version, 1,
        "legacy preferences should be upgraded to schema version 1"
    );
    assert_eq!(
        prefs.editor.autosave_interval_minutes, 5,
        "autosave interval should fall back to default when legacy data is zero"
    );
    assert_eq!(
        prefs.ui.theme, "Midnight Indigo",
        "empty theme should fall back to default"
    );
    assert_eq!(
        prefs.ui.locale, "zh-TW",
        "specified locale should be preserved during migration"
    );
}
