use std::fs;
use std::path::PathBuf;

use rustnotepad_settings::{LocalizationError, LocalizationManager, LocalizationParams};
use tempfile::tempdir;

#[test]
fn fallback_returns_builtin_strings() {
    let manager = LocalizationManager::fallback();
    assert_eq!(manager.text("menu.file"), "File");
    assert_eq!(manager.text("missing.key"), "missing.key");
}

#[test]
fn indexed_placeholders_are_applied() {
    let manager = LocalizationManager::fallback();
    let values = ["7"];
    let params = LocalizationParams::new(&values);
    assert_eq!(
        manager
            .text_with_params("toolbar.pinned_tabs", &params)
            .as_ref(),
        "Pinned tabs: 7"
    );
}

#[test]
fn plural_selection_uses_locale_rules() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("ru-RU.json");
    fs::write(
        &path,
        r#"
        {
            "locale": "ru-RU",
            "strings": {
                "notifications.count": {
                    "type": "plural",
                    "one": "{count} уведомление",
                    "few": "{count} уведомления",
                    "many": "{count} уведомлений",
                    "other": "{count} уведомления"
                }
            }
        }
        "#,
    )
    .expect("write locale");

    let mut manager = LocalizationManager::load_from_dir(temp.path(), "en-US").expect("load");
    let index = manager
        .locale_summaries()
        .iter()
        .position(|summary| summary.code == "ru-RU")
        .expect("locale present");
    assert!(manager.set_active_by_index(index));

    let singular = LocalizationParams::count_only(1);
    assert_eq!(
        manager
            .text_with_params("notifications.count", &singular)
            .as_ref(),
        "1 уведомление"
    );

    let few = LocalizationParams::count_only(3);
    assert_eq!(
        manager
            .text_with_params("notifications.count", &few)
            .as_ref(),
        "3 уведомления"
    );

    let many = LocalizationParams::count_only(5);
    assert_eq!(
        manager
            .text_with_params("notifications.count", &many)
            .as_ref(),
        "5 уведомлений"
    );
}

#[test]
fn plural_requires_other_category() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("xx.json");
    fs::write(
        &path,
        r#"
        {
            "locale": "xx",
            "strings": {
                "sample": {
                    "type": "plural",
                    "one": "only one"
                }
            }
        }
        "#,
    )
    .expect("write locale");

    let error = LocalizationManager::load_from_dir(temp.path(), "en-US").unwrap_err();
    match error {
        LocalizationError::PluralMissingOther { locale, key } => {
            assert_eq!(locale, "xx");
            assert_eq!(key, "sample");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn catalog_stats_report_counts() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("ru-RU.json");
    fs::write(
        &path,
        r#"
        {
            "locale": "ru-RU",
            "strings": {
                "notifications.count": {
                    "type": "plural",
                    "one": "{count} уведомление",
                    "few": "{count} уведомления",
                    "many": "{count} уведомлений",
                    "other": "{count} уведомлений"
                }
            }
        }
        "#,
    )
    .expect("write locale");

    let manager = LocalizationManager::load_from_dir(temp.path(), "en-US").expect("load");
    let stats = manager.catalog_stats();
    assert!(stats
        .iter()
        .any(|locale| locale.code == "ru-RU" && locale.plural_entries == 1));
    assert!(stats
        .iter()
        .any(|locale| locale.code == manager.fallback_code()));
}

#[test]
fn missing_keys_reports_differences() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("fr.json");
    fs::write(
        &path,
        r#"
        {
            "locale": "fr",
            "strings": {
                "menu.file": "Fichier"
            }
        }
        "#,
    )
    .expect("write locale");

    let manager = LocalizationManager::load_from_dir(temp.path(), "en-US").expect("load");
    let missing = manager.missing_keys("fr").expect("locale present");
    assert!(missing.iter().any(|key| key == "menu.edit"));
}

#[test]
fn set_active_by_code_switches_locale() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("fr-FR.json");
    fs::write(
        &path,
        r#"
        {
            "locale": "fr-FR",
            "strings": {
                "menu.file": "Fichier"
            }
        }
        "#,
    )
    .expect("write locale");

    let mut manager = LocalizationManager::load_from_dir(temp.path(), "en-US").expect("load");
    assert!(manager.set_active_by_code("fr-FR"));
    assert_eq!(manager.active_code(), "fr-FR");
    assert_eq!(manager.text("menu.file"), "Fichier");
}

#[test]
fn load_from_dirs_merges_multiple_sources() {
    let dir_a = tempdir().expect("dir a");
    let dir_b = tempdir().expect("dir b");

    fs::write(
        dir_a.path().join("fr-FR.json"),
        r#"
        {
            "locale": "fr-FR",
            "strings": {
                "menu.file": "Fichier"
            }
        }
        "#,
    )
    .expect("write fr locale");

    fs::write(
        dir_b.path().join("ja-JP.json"),
        r#"
        {
            "locale": "ja-JP",
            "strings": {
                "menu.file": "ファイル"
            }
        }
        "#,
    )
    .expect("write ja locale");

    let manager = LocalizationManager::load_from_dirs(
        vec![dir_a.path().to_path_buf(), dir_b.path().to_path_buf()],
        "en-US",
    )
    .expect("load");
    let summaries = manager.locale_summaries();
    let codes: Vec<_> = summaries
        .iter()
        .map(|summary| summary.code.as_str())
        .collect();
    assert!(
        codes.iter().any(|code| *code == "fr-FR"),
        "expected fr-FR locale in merged manager, got {codes:?}"
    );
    assert!(
        codes.iter().any(|code| *code == "ja-JP"),
        "expected ja-JP locale in merged manager, got {codes:?}"
    );
}

#[test]
fn builtin_locales_include_view_toggles() {
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/langs");
    let manager =
        LocalizationManager::load_from_dir(&assets_dir, "en-US").expect("load built-in locales");
    assert_eq!(manager.text("menu.view.status_bar"), "Status Bar");
    assert!(
        manager.locale_has_key("zh-TW", "menu.view.status_bar"),
        "Traditional Chinese locale is missing menu.view.status_bar"
    );
    assert!(
        manager.locale_has_key("zh-TW", "menu.view.bottom_panels"),
        "Traditional Chinese locale is missing menu.view.bottom_panels"
    );
}

#[test]
fn zh_tw_locale_translates_status_bar() {
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/langs");
    let mut manager =
        LocalizationManager::load_from_dir(&assets_dir, "en-US").expect("load built-in locales");
    assert!(manager.set_active_by_code("zh-TW"), "failed to switch to zh-TW");
    assert_eq!(
        manager.text("menu.view.status_bar"),
        "狀態列",
        "menu.view.status_bar did not resolve to zh-TW translation"
    );
}
