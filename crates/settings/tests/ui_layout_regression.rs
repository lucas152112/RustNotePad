use rustnotepad_settings::{LayoutConfig, PaneRole, ThemeDefinition, ThemeManager};

/// 驗證預設版面、主題切換與狀態維持。 / Regression check for default layout + theme switching.
#[test]
fn layout_and_theme_regression() {
    let layout = LayoutConfig::default();

    // Primary pane should focus on search.rs by default.
    // 主窗格預設應聚焦於 search.rs。
    let active_primary = layout.active_tab(PaneRole::Primary).expect("primary tab");
    assert_eq!(active_primary.title, "search.rs");
    assert!(active_primary.color.is_some());

    // Secondary pane keeps docs/design.md active as the preview tab.
    // 次窗格維持 docs/design.md 為預覽分頁。
    let active_secondary = layout
        .active_tab(PaneRole::Secondary)
        .expect("secondary tab");
    assert_eq!(active_secondary.title, "design.md");

    // Pinned tabs across both panes should total at least two.
    // 兩個窗格的釘選分頁總數應至少為兩個。
    assert!(layout.pinned_tabs().len() >= 2);

    // Theme manager toggling should preserve the display names.
    // 主題管理器切換後應保留顯示名稱。
    let mut manager = ThemeManager::new(vec![
        ThemeDefinition::builtin_dark(),
        ThemeDefinition::builtin_light(),
    ])
    .expect("themes");

    assert_eq!(manager.active_theme().name, "Midnight Indigo");
    manager.set_active_index(1).expect("switch theme");
    assert_eq!(manager.active_theme().name, "Nordic Daylight");
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
