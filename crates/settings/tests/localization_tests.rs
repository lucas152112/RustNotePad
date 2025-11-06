use std::fs;

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
