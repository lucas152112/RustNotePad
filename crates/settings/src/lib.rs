pub mod associations;
mod json;
pub mod layout;
pub mod recent;
pub mod storage;
pub mod theme;

pub use associations::{FileAssociation, FileAssociations};
pub use layout::{
    DockLayout, LayoutConfig, LayoutError, PaneLayout, PaneRole, TabColorTag, TabView,
};
pub use recent::RecentFiles;
pub use storage::{FileAssociationsStore, RecentFilesStore};
pub use theme::{
    Color, ColorParseError, FontSettings, ResolvedPalette, ThemeDefinition, ThemeKind,
    ThemeLoadError, ThemeManager, ThemePalette,
};
