pub mod associations;
mod json;
pub mod layout;
pub mod localization;
pub mod preferences;
pub mod recent;
pub mod snippets;
pub mod storage;
pub mod theme;

pub use associations::{FileAssociation, FileAssociations};
pub use layout::{
    DockLayout, LayoutConfig, LayoutError, PaneLayout, PaneRole, TabColorTag, TabView,
};
pub use localization::{
    LocaleCatalogStats, LocaleSummary, LocalizationError, LocalizationManager, LocalizationParams,
};
pub use preferences::{
    EditorPreferences, Preferences, PreferencesError, PreferencesStore, UiPreferences,
};
pub use recent::RecentFiles;
pub use snippets::{SnippetDefinition, SnippetStore};
pub use storage::{FileAssociationsStore, RecentFilesStore};
pub use theme::{
    Color, ColorParseError, FontSettings, ResolvedPalette, ThemeDefinition, ThemeKind,
    ThemeLoadError, ThemeManager, ThemePalette,
};
