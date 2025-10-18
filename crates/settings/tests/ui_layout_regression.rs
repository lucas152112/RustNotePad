use rustnotepad_settings::{LayoutConfig, PaneRole, ThemeDefinition, ThemeManager};

/// 驗證預設版面、主題切換與狀態維持。 / Regression check for default layout + theme switching.
#[test]
fn layout_and_theme_regression() {
    let layout = LayoutConfig::default();

    // Primary pane should預設聚焦 search.rs
    let active_primary = layout.active_tab(PaneRole::Primary).expect("primary tab");
    assert_eq!(active_primary.title, "search.rs");
    assert!(active_primary.color.is_some());

    // Secondary pane keeps docs design.md active
    let active_secondary = layout
        .active_tab(PaneRole::Secondary)
        .expect("secondary tab");
    assert_eq!(active_secondary.title, "design.md");

    // Pinned tabs跨 pane 合計至少兩個
    assert!(layout.pinned_tabs().len() >= 2);

    // Theme manager toggling retains names
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
