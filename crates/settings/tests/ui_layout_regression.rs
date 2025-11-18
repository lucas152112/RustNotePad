use rustnotepad_settings::{LayoutConfig, PaneRole, ThemeManager};
use std::path::PathBuf;

/// 驗證預設版面、主題切換與狀態維持。 / Regression check for default layout + theme switching.
#[test]
fn layout_and_theme_regression() {
    let layout = LayoutConfig::default();

    // Primary pane should focus on search.rs by default.
    // 主窗格預設應聚焦於 search.rs。
    let active_primary = layout.active_tab(PaneRole::Primary).expect("primary tab");
    assert_eq!(active_primary.title, "search.rs");
    assert!(active_primary.color.is_some());

    // Pinned tabs should exist in the primary pane.
    // 主要窗格應包含至少一個釘選分頁。
    assert!(layout.pinned_tabs().len() >= 1);

    // Theme manager toggling should preserve the display names.
    // 主題管理器切換後應保留顯示名稱。
    let theme_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/themes");
    let mut manager = ThemeManager::load_from_dirs(vec![theme_dir]).expect("themes");

    manager
        .set_active_by_name("Notepad++ Classic")
        .expect("select default theme");
    assert_eq!(manager.active_theme().name, "Notepad++ Classic");
    manager
        .set_active_by_name("Nordic Daylight")
        .expect("switch theme");
    assert_ne!(
        manager.active_palette().status_bar,
        manager
            .themes()
            .next()
            .unwrap()
            .resolve_palette()
            .unwrap()
            .status_bar
    );
}
