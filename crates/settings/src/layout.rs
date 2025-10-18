use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::json::{self, JsonValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneRole {
    Primary,
    Secondary,
}

impl PaneRole {
    fn as_str(self) -> &'static str {
        match self {
            PaneRole::Primary => "primary",
            PaneRole::Secondary => "secondary",
        }
    }

    fn from_str(value: &str) -> Result<Self, LayoutError> {
        match value {
            "primary" => Ok(PaneRole::Primary),
            "secondary" => Ok(PaneRole::Secondary),
            other => Err(LayoutError::InvalidPaneRole(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabColorTag {
    Blue,
    Green,
    Orange,
    Purple,
    Red,
}

impl TabColorTag {
    pub fn hex(self) -> &'static str {
        match self {
            TabColorTag::Blue => "#4F75FF",
            TabColorTag::Green => "#46BE73",
            TabColorTag::Orange => "#F59E0B",
            TabColorTag::Purple => "#A855F7",
            TabColorTag::Red => "#EF4444",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            TabColorTag::Blue => "blue",
            TabColorTag::Green => "green",
            TabColorTag::Orange => "orange",
            TabColorTag::Purple => "purple",
            TabColorTag::Red => "red",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "blue" => Some(TabColorTag::Blue),
            "green" => Some(TabColorTag::Green),
            "orange" => Some(TabColorTag::Orange),
            "purple" => Some(TabColorTag::Purple),
            "red" => Some(TabColorTag::Red),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabView {
    pub id: String,
    pub title: String,
    pub language: Option<String>,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub color: Option<TabColorTag>,
}

impl TabView {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            language: None,
            is_pinned: false,
            is_locked: false,
            color: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneLayout {
    pub role: PaneRole,
    pub tabs: Vec<TabView>,
    pub active: Option<String>,
}

impl PaneLayout {
    pub fn new(role: PaneRole, tabs: Vec<TabView>, active: Option<String>) -> Self {
        Self { role, tabs, active }
    }

    pub fn active_tab(&self) -> Option<&TabView> {
        let active_id = self.active.as_ref()?;
        self.tabs.iter().find(|tab| &tab.id == active_id)
    }

    pub fn pinned_tabs(&self) -> impl Iterator<Item = &TabView> {
        self.tabs.iter().filter(|tab| tab.is_pinned)
    }

    pub fn regular_tabs(&self) -> impl Iterator<Item = &TabView> {
        self.tabs.iter().filter(|tab| !tab.is_pinned)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockLayout {
    pub visible_panels: Vec<String>,
    pub active_panel: Option<String>,
}

impl DockLayout {
    pub fn is_visible(&self, panel_id: &str) -> bool {
        self.visible_panels.iter().any(|panel| panel == panel_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutConfig {
    pub panes: Vec<PaneLayout>,
    pub bottom_dock: DockLayout,
    pub split_ratio: f32,
}

impl LayoutConfig {
    pub fn default() -> Self {
        let primary_tabs = vec![
            TabView {
                id: "core/src/editor.rs".into(),
                title: "editor.rs".into(),
                language: Some("Rust".into()),
                is_pinned: true,
                is_locked: false,
                color: Some(TabColorTag::Purple),
            },
            TabView {
                id: "search/src/lib.rs".into(),
                title: "search.rs".into(),
                language: Some("Rust".into()),
                is_pinned: false,
                is_locked: false,
                color: Some(TabColorTag::Blue),
            },
            TabView {
                id: "README.md".into(),
                title: "README.md".into(),
                language: Some("Markdown".into()),
                is_pinned: false,
                is_locked: false,
                color: None,
            },
        ];

        let secondary_tabs = vec![
            TabView {
                id: "tests/search_workflow.rs".into(),
                title: "search_workflow.rs".into(),
                language: Some("Rust".into()),
                is_pinned: true,
                is_locked: true,
                color: Some(TabColorTag::Green),
            },
            TabView {
                id: "docs/feature_parity/03-search-replace/design.md".into(),
                title: "design.md".into(),
                language: Some("Markdown".into()),
                is_pinned: false,
                is_locked: false,
                color: Some(TabColorTag::Orange),
            },
        ];

        Self {
            panes: vec![
                PaneLayout::new(
                    PaneRole::Primary,
                    primary_tabs,
                    Some("search/src/lib.rs".into()),
                ),
                PaneLayout::new(
                    PaneRole::Secondary,
                    secondary_tabs,
                    Some("docs/feature_parity/03-search-replace/design.md".into()),
                ),
            ],
            bottom_dock: DockLayout {
                visible_panels: vec![
                    "find_results".into(),
                    "console".into(),
                    "notifications".into(),
                    "lsp".into(),
                ],
                active_panel: Some("find_results".into()),
            },
            split_ratio: 0.55,
        }
    }

    pub fn active_tab(&self, role: PaneRole) -> Option<&TabView> {
        self.panes
            .iter()
            .find(|pane| pane.role == role)
            .and_then(|pane| pane.active_tab())
    }

    pub fn set_active_tab(&mut self, role: PaneRole, tab_id: &str) -> bool {
        if let Some(pane) = self.panes.iter_mut().find(|pane| pane.role == role) {
            if pane.tabs.iter().any(|tab| tab.id == tab_id) {
                pane.active = Some(tab_id.to_string());
                return true;
            }
        }
        false
    }

    pub fn pinned_tabs(&self) -> Vec<&TabView> {
        let mut pinned = Vec::new();
        for pane in &self.panes {
            pinned.extend(pane.pinned_tabs());
        }
        pinned
    }

    pub fn validate_split_ratio(split_ratio: f32) -> Result<f32, LayoutError> {
        if (0.1..=0.9).contains(&split_ratio) {
            Ok(split_ratio)
        } else {
            Err(LayoutError::InvalidSplitRatio(split_ratio))
        }
    }

    pub fn validate(&self) -> Result<(), LayoutError> {
        Self::validate_split_ratio(self.split_ratio)?;

        for pane in &self.panes {
            let mut ids = BTreeSet::new();
            for tab in &pane.tabs {
                if !ids.insert(tab.id.clone()) {
                    return Err(LayoutError::DuplicateTabId(tab.id.clone()));
                }
            }

            if let Some(active) = &pane.active {
                if !pane.tabs.iter().any(|tab| &tab.id == active) {
                    return Err(LayoutError::UnknownActiveTab {
                        pane: pane.role,
                        tab: active.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn to_json(&self) -> Result<String, LayoutError> {
        self.validate()?;
        let mut root = BTreeMap::new();
        root.insert(
            "split_ratio".into(),
            JsonValue::Number(self.split_ratio as f64),
        );

        let panes_json: Vec<JsonValue> = self.panes.iter().map(|pane| pane_to_json(pane)).collect();
        root.insert("panes".into(), JsonValue::Array(panes_json));

        let dock_json = dock_to_json(&self.bottom_dock);
        root.insert("bottom_dock".into(), dock_json);

        Ok(json::stringify_pretty(&JsonValue::Object(root), 2))
    }

    pub fn from_json(input: &str) -> Result<Self, LayoutError> {
        let value = json::parse(input)?;
        let root = value.as_object().map_err(LayoutError::from)?;

        let split_ratio = root
            .get("split_ratio")
            .ok_or(LayoutError::MissingField("split_ratio"))?
            .as_f64()
            .map_err(LayoutError::from)? as f32;

        let panes_value = root
            .get("panes")
            .ok_or(LayoutError::MissingField("panes"))?
            .as_array()
            .map_err(LayoutError::from)?;
        let mut panes = Vec::with_capacity(panes_value.len());
        for pane_value in panes_value {
            panes.push(pane_from_json(pane_value)?);
        }

        let dock_value = root
            .get("bottom_dock")
            .ok_or(LayoutError::MissingField("bottom_dock"))?;
        let bottom_dock = dock_from_json(dock_value)?;

        let config = Self {
            panes,
            bottom_dock,
            split_ratio,
        };
        config.validate()?;
        Ok(config)
    }
}

fn pane_to_json(pane: &PaneLayout) -> JsonValue {
    let mut map = BTreeMap::new();
    map.insert(
        "role".into(),
        JsonValue::String(pane.role.as_str().to_string()),
    );
    if let Some(active) = &pane.active {
        map.insert("active".into(), JsonValue::String(active.clone()));
    } else {
        map.insert("active".into(), JsonValue::Null);
    }

    let tabs: Vec<JsonValue> = pane.tabs.iter().map(|tab| tab_to_json(tab)).collect();
    map.insert("tabs".into(), JsonValue::Array(tabs));
    JsonValue::Object(map)
}

fn tab_to_json(tab: &TabView) -> JsonValue {
    let mut map = BTreeMap::new();
    map.insert("id".into(), JsonValue::String(tab.id.clone()));
    map.insert("title".into(), JsonValue::String(tab.title.clone()));
    map.insert("is_pinned".into(), JsonValue::Bool(tab.is_pinned));
    map.insert("is_locked".into(), JsonValue::Bool(tab.is_locked));
    if let Some(lang) = &tab.language {
        map.insert("language".into(), JsonValue::String(lang.clone()));
    } else {
        map.insert("language".into(), JsonValue::Null);
    }
    if let Some(color) = tab.color {
        map.insert(
            "color".into(),
            JsonValue::String(color.as_str().to_string()),
        );
    } else {
        map.insert("color".into(), JsonValue::Null);
    }
    JsonValue::Object(map)
}

fn dock_to_json(dock: &DockLayout) -> JsonValue {
    let mut map = BTreeMap::new();
    let panels = dock
        .visible_panels
        .iter()
        .map(|panel| JsonValue::String(panel.clone()))
        .collect();
    map.insert("visible_panels".into(), JsonValue::Array(panels));
    if let Some(active) = &dock.active_panel {
        map.insert("active_panel".into(), JsonValue::String(active.clone()));
    } else {
        map.insert("active_panel".into(), JsonValue::Null);
    }
    JsonValue::Object(map)
}

fn pane_from_json(value: &JsonValue) -> Result<PaneLayout, LayoutError> {
    let map = value.as_object().map_err(LayoutError::from)?;
    let role_value = map
        .get("role")
        .ok_or(LayoutError::MissingField("role"))?
        .as_str()
        .map_err(LayoutError::from)?;
    let role = PaneRole::from_str(role_value)?;

    let tabs_value = map
        .get("tabs")
        .ok_or(LayoutError::MissingField("tabs"))?
        .as_array()
        .map_err(LayoutError::from)?;
    let mut tabs = Vec::with_capacity(tabs_value.len());
    for value in tabs_value {
        tabs.push(tab_from_json(value)?);
    }

    let active = match map.get("active") {
        Some(JsonValue::String(text)) => Some(text.clone()),
        Some(JsonValue::Null) | None => None,
        Some(_) => return Err(LayoutError::ExpectedString("active")),
    };

    Ok(PaneLayout { role, tabs, active })
}

fn tab_from_json(value: &JsonValue) -> Result<TabView, LayoutError> {
    let map = value.as_object().map_err(LayoutError::from)?;
    let id = map
        .get("id")
        .ok_or(LayoutError::MissingField("id"))?
        .as_str()
        .map_err(LayoutError::from)?
        .to_string();
    let title = map
        .get("title")
        .ok_or(LayoutError::MissingField("title"))?
        .as_str()
        .map_err(LayoutError::from)?
        .to_string();
    let is_pinned = map
        .get("is_pinned")
        .ok_or(LayoutError::MissingField("is_pinned"))?
        .as_bool()
        .map_err(LayoutError::from)?;
    let is_locked = map
        .get("is_locked")
        .ok_or(LayoutError::MissingField("is_locked"))?
        .as_bool()
        .map_err(LayoutError::from)?;

    let language = match map.get("language") {
        Some(JsonValue::String(text)) => Some(text.clone()),
        Some(JsonValue::Null) | None => None,
        Some(_) => return Err(LayoutError::ExpectedString("language")),
    };

    let color = match map.get("color") {
        Some(JsonValue::String(text)) => Some(
            TabColorTag::from_str(text)
                .ok_or_else(|| LayoutError::InvalidColorTag(text.clone()))?,
        ),
        Some(JsonValue::Null) | None => None,
        Some(_) => return Err(LayoutError::ExpectedString("color")),
    };

    Ok(TabView {
        id,
        title,
        language,
        is_pinned,
        is_locked,
        color,
    })
}

fn dock_from_json(value: &JsonValue) -> Result<DockLayout, LayoutError> {
    let map = value.as_object().map_err(LayoutError::from)?;
    let panels_value = map
        .get("visible_panels")
        .ok_or(LayoutError::MissingField("visible_panels"))?
        .as_array()
        .map_err(LayoutError::from)?;
    let mut visible_panels = Vec::with_capacity(panels_value.len());
    for panel in panels_value {
        visible_panels.push(panel.as_str().map_err(LayoutError::from)?.to_string());
    }

    let active_panel = match map.get("active_panel") {
        Some(JsonValue::String(text)) => Some(text.clone()),
        Some(JsonValue::Null) | None => None,
        Some(_) => return Err(LayoutError::ExpectedString("active_panel")),
    };

    Ok(DockLayout {
        visible_panels,
        active_panel,
    })
}

#[derive(Debug)]
pub enum LayoutError {
    InvalidSplitRatio(f32),
    DuplicateTabId(String),
    UnknownActiveTab { pane: PaneRole, tab: String },
    Json(json::JsonError),
    MissingField(&'static str),
    ExpectedString(&'static str),
    InvalidColorTag(String),
    InvalidPaneRole(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::InvalidSplitRatio(value) => {
                write!(f, "split ratio {value} is outside the 0.1..=0.9 range")
            }
            LayoutError::DuplicateTabId(id) => write!(f, "duplicate tab id detected: {id}"),
            LayoutError::UnknownActiveTab { pane, tab } => {
                write!(f, "pane {:?} references missing tab {tab}", pane)
            }
            LayoutError::Json(err) => err.fmt(f),
            LayoutError::MissingField(field) => write!(f, "missing field '{field}'"),
            LayoutError::ExpectedString(field) => {
                write!(f, "field '{field}' must be a string or null")
            }
            LayoutError::InvalidColorTag(value) => {
                write!(f, "unknown color tag '{value}'")
            }
            LayoutError::InvalidPaneRole(value) => {
                write!(f, "unknown pane role '{value}'")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

impl From<json::JsonError> for LayoutError {
    fn from(value: json::JsonError) -> Self {
        LayoutError::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_has_pinned_tabs() {
        let layout = LayoutConfig::default();
        let pinned: Vec<_> = layout.pinned_tabs();
        assert!(!pinned.is_empty());
        assert!(pinned.iter().any(|tab| tab.is_locked));
    }

    #[test]
    fn serialization_round_trip_preserves_state() {
        let layout = LayoutConfig::default();
        let json = layout.to_json().expect("serialize");
        let restored = LayoutConfig::from_json(&json).expect("deserialize");
        assert_eq!(layout, restored);
    }

    #[test]
    fn rejects_invalid_split_ratio() {
        let mut layout = LayoutConfig::default();
        layout.split_ratio = 0.95;
        let result = layout.to_json();
        assert!(matches!(
            result.unwrap_err(),
            LayoutError::InvalidSplitRatio(value) if (value - 0.95).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn set_active_tab_updates_pane() {
        let mut layout = LayoutConfig::default();
        assert!(layout.set_active_tab(PaneRole::Primary, "core/src/editor.rs"));
        let active = layout.active_tab(PaneRole::Primary).unwrap();
        assert_eq!(active.title, "editor.rs");
    }
}
