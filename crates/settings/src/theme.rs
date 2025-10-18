use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::json::{self, JsonValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn from_hex(input: &str) -> Result<Self, ThemeLoadError> {
        parse_hex(input).map_err(|err| ThemeLoadError::InvalidColor {
            value: input.to_string(),
            reason: err,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPalette {
    pub background: Color,
    pub panel: Color,
    pub accent: Color,
    pub accent_text: Color,
    pub editor_background: Color,
    pub editor_text: Color,
    pub status_bar: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub kind: ThemeKind,
    pub palette: ThemePalette,
    pub fonts: FontSettings,
}

impl ThemeDefinition {
    pub fn builtin_dark() -> Self {
        Self {
            name: "Midnight Indigo".into(),
            description: Some("A high contrast dark theme tuned for Rust editing".into()),
            kind: ThemeKind::Dark,
            palette: ThemePalette {
                background: "#111827".into(),
                panel: "#1F2937".into(),
                accent: "#2563EB".into(),
                accent_text: "#F9FAFB".into(),
                editor_background: "#0F172A".into(),
                editor_text: "#E5E7EB".into(),
                status_bar: "#374151".into(),
            },
            fonts: FontSettings {
                ui_family: "Inter".into(),
                ui_size: 16,
                editor_family: "JetBrains Mono".into(),
                editor_size: 15,
            },
        }
    }

    pub fn builtin_light() -> Self {
        Self {
            name: "Nordic Daylight".into(),
            description: Some("Soft light palette with subtle blues".into()),
            kind: ThemeKind::Light,
            palette: ThemePalette {
                background: "#F3F4F6".into(),
                panel: "#FFFFFF".into(),
                accent: "#3B82F6".into(),
                accent_text: "#0F172A".into(),
                editor_background: "#FFFFFF".into(),
                editor_text: "#111827".into(),
                status_bar: "#E5E7EB".into(),
            },
            fonts: FontSettings {
                ui_family: "Inter".into(),
                ui_size: 16,
                editor_family: "Fira Code".into(),
                editor_size: 15,
            },
        }
    }

    pub fn resolve_palette(&self) -> Result<ResolvedPalette, ThemeLoadError> {
        Ok(ResolvedPalette {
            background: Color::from_hex(&self.palette.background)?,
            panel: Color::from_hex(&self.palette.panel)?,
            accent: Color::from_hex(&self.palette.accent)?,
            accent_text: Color::from_hex(&self.palette.accent_text)?,
            editor_background: Color::from_hex(&self.palette.editor_background)?,
            editor_text: Color::from_hex(&self.palette.editor_text)?,
            status_bar: Color::from_hex(&self.palette.status_bar)?,
        })
    }

    pub fn validate(&self) -> Result<(), ThemeLoadError> {
        if self.fonts.ui_size == 0 || self.fonts.editor_size == 0 {
            return Err(ThemeLoadError::InvalidFontSize);
        }
        self.resolve_palette().map(|_| ())
    }

    fn from_json(value: &JsonValue) -> Result<Self, ThemeLoadError> {
        let map = value.as_object().map_err(ThemeLoadError::from)?;

        let name = string_field(map, "name")?;
        let description = match map.get("description") {
            Some(JsonValue::String(text)) => Some(text.clone()),
            Some(JsonValue::Null) | None => None,
            Some(_) => return Err(ThemeLoadError::InvalidField("description")),
        };

        let kind_str = string_field(map, "kind")?;
        let kind = ThemeKind::from_str(&kind_str)?;

        let palette_value = map
            .get("palette")
            .ok_or(ThemeLoadError::MissingField("palette"))?;
        let palette = ThemePalette::from_json(palette_value)?;

        let fonts_value = map
            .get("fonts")
            .ok_or(ThemeLoadError::MissingField("fonts"))?;
        let fonts = FontSettings::from_json(fonts_value)?;

        Ok(Self {
            name,
            description,
            kind,
            palette,
            fonts,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    Dark,
    Light,
}

impl ThemeKind {
    fn from_str(value: &str) -> Result<Self, ThemeLoadError> {
        match value {
            "dark" => Ok(ThemeKind::Dark),
            "light" => Ok(ThemeKind::Light),
            other => Err(ThemeLoadError::InvalidKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePalette {
    pub background: String,
    pub panel: String,
    pub accent: String,
    pub accent_text: String,
    pub editor_background: String,
    pub editor_text: String,
    pub status_bar: String,
}

impl ThemePalette {
    fn from_json(value: &JsonValue) -> Result<Self, ThemeLoadError> {
        let map = value.as_object().map_err(ThemeLoadError::from)?;
        Ok(Self {
            background: string_field(map, "background")?,
            panel: string_field(map, "panel")?,
            accent: string_field(map, "accent")?,
            accent_text: string_field(map, "accent_text")?,
            editor_background: string_field(map, "editor_background")?,
            editor_text: string_field(map, "editor_text")?,
            status_bar: string_field(map, "status_bar")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSettings {
    pub ui_family: String,
    pub ui_size: u16,
    pub editor_family: String,
    pub editor_size: u16,
}

impl FontSettings {
    fn from_json(value: &JsonValue) -> Result<Self, ThemeLoadError> {
        let map = value.as_object().map_err(ThemeLoadError::from)?;
        Ok(Self {
            ui_family: string_field(map, "ui_family")?,
            ui_size: number_field(map, "ui_size")?,
            editor_family: string_field(map, "editor_family")?,
            editor_size: number_field(map, "editor_size")?,
        })
    }
}

struct ThemeEntry {
    definition: ThemeDefinition,
    palette: ResolvedPalette,
}

impl ThemeEntry {
    fn new(definition: ThemeDefinition) -> Result<Self, ThemeLoadError> {
        definition.validate()?;
        let palette = definition.resolve_palette()?;
        Ok(Self {
            definition,
            palette,
        })
    }
}

pub struct ThemeManager {
    entries: Vec<ThemeEntry>,
    active: usize,
}

impl ThemeManager {
    pub fn new(definitions: Vec<ThemeDefinition>) -> Result<Self, ThemeLoadError> {
        if definitions.is_empty() {
            return Err(ThemeLoadError::Empty);
        }
        let mut entries = Vec::with_capacity(definitions.len());
        for definition in definitions {
            entries.push(ThemeEntry::new(definition)?);
        }
        Ok(Self { entries, active: 0 })
    }

    pub fn load_from_dir(path: impl AsRef<Path>) -> Result<Self, ThemeLoadError> {
        let dir = path.as_ref();
        let mut definitions = Vec::new();
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false)
                {
                    let data = fs::read_to_string(&path)?;
                    let json_value = json::parse(&data)?;
                    let definition = ThemeDefinition::from_json(&json_value)?;
                    definitions.push(definition);
                }
            }
        }

        if definitions.is_empty() {
            definitions = vec![Self::fallback_dark(), Self::fallback_light()];
        }

        Self::new(definitions)
    }

    fn fallback_dark() -> ThemeDefinition {
        ThemeDefinition::builtin_dark()
    }

    fn fallback_light() -> ThemeDefinition {
        ThemeDefinition::builtin_light()
    }

    pub fn active_theme(&self) -> &ThemeDefinition {
        &self.entries[self.active].definition
    }

    pub fn active_palette(&self) -> &ResolvedPalette {
        &self.entries[self.active].palette
    }

    pub fn set_active_by_name(&mut self, name: &str) -> Option<&ThemeDefinition> {
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.definition.name == name)
        {
            self.active = index;
            Some(&self.entries[self.active].definition)
        } else {
            None
        }
    }

    pub fn set_active_index(&mut self, index: usize) -> Option<&ThemeDefinition> {
        if index < self.entries.len() {
            self.active = index;
            Some(&self.entries[self.active].definition)
        } else {
            None
        }
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn themes(&self) -> impl Iterator<Item = &ThemeDefinition> {
        self.entries.iter().map(|entry| &entry.definition)
    }

    pub fn theme_names(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .map(|entry| entry.definition.name.as_str())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn theme_paths(path: impl AsRef<Path>) -> Vec<PathBuf> {
        let dir = path.as_ref();
        if !dir.is_dir() {
            return Vec::new();
        }
        let mut paths = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false)
                {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        paths
    }
}

#[derive(Debug)]
pub enum ThemeLoadError {
    Io(std::io::Error),
    Json(json::JsonError),
    InvalidColor {
        value: String,
        reason: ColorParseError,
    },
    InvalidFontSize,
    InvalidField(&'static str),
    MissingField(&'static str),
    InvalidKind(String),
    Empty,
}

impl fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeLoadError::Io(err) => err.fmt(f),
            ThemeLoadError::Json(err) => err.fmt(f),
            ThemeLoadError::InvalidColor { value, reason } => {
                write!(f, "invalid color {value}: {reason}")
            }
            ThemeLoadError::InvalidFontSize => write!(f, "font sizes must be greater than zero"),
            ThemeLoadError::InvalidField(field) => {
                write!(f, "field '{field}' has an unexpected type")
            }
            ThemeLoadError::MissingField(field) => write!(f, "missing field '{field}'"),
            ThemeLoadError::InvalidKind(value) => write!(f, "invalid theme kind '{value}'"),
            ThemeLoadError::Empty => write!(f, "no theme definitions were provided"),
        }
    }
}

impl std::error::Error for ThemeLoadError {}

impl From<std::io::Error> for ThemeLoadError {
    fn from(value: std::io::Error) -> Self {
        ThemeLoadError::Io(value)
    }
}

impl From<json::JsonError> for ThemeLoadError {
    fn from(value: json::JsonError) -> Self {
        ThemeLoadError::Json(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorParseError {
    MissingHashPrefix,
    InvalidLength,
    InvalidHex,
}

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorParseError::MissingHashPrefix => write!(f, "missing leading '#'"),
            ColorParseError::InvalidLength => write!(f, "expected 6 or 8 hexadecimal digits"),
            ColorParseError::InvalidHex => write!(f, "contains non-hexadecimal digits"),
        }
    }
}

fn string_field(
    map: &std::collections::BTreeMap<String, JsonValue>,
    key: &'static str,
) -> Result<String, ThemeLoadError> {
    map.get(key)
        .ok_or(ThemeLoadError::MissingField(key))?
        .as_str()
        .map_err(ThemeLoadError::from)
        .map(|value| value.to_string())
}

fn number_field(
    map: &std::collections::BTreeMap<String, JsonValue>,
    key: &'static str,
) -> Result<u16, ThemeLoadError> {
    let value = map
        .get(key)
        .ok_or(ThemeLoadError::MissingField(key))?
        .as_f64()
        .map_err(ThemeLoadError::from)?;
    if value <= 0.0 {
        return Err(ThemeLoadError::InvalidFontSize);
    }
    Ok(value.round() as u16)
}

fn parse_hex(input: &str) -> Result<Color, ColorParseError> {
    let trimmed = input.trim();
    let hex = trimmed
        .strip_prefix('#')
        .ok_or(ColorParseError::MissingHashPrefix)?;
    if hex.len() != 6 && hex.len() != 8 {
        return Err(ColorParseError::InvalidLength);
    }
    let mut rgba = [0u8; 4];
    for i in 0..(hex.len() / 2) {
        let start = i * 2;
        let slice = &hex[start..start + 2];
        let value = u8::from_str_radix(slice, 16).map_err(|_| ColorParseError::InvalidHex)?;
        rgba[i] = value;
    }
    if hex.len() == 6 {
        rgba[3] = 255;
    }
    Ok(Color {
        r: rgba[0],
        g: rgba[1],
        b: rgba[2],
        a: rgba[3],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_hex_accepts_six_and_eight_digit_values() {
        let color = Color::from_hex("#FFAA33").unwrap();
        assert_eq!(color.r, 0xFF);
        assert_eq!(color.a, 0xFF);

        let color = Color::from_hex("#11223344").unwrap();
        assert_eq!(color.b, 0x33);
        assert_eq!(color.a, 0x44);
    }

    #[test]
    fn parse_hex_rejects_invalid_input() {
        assert!(matches!(
            Color::from_hex("123456").unwrap_err(),
            ThemeLoadError::InvalidColor {
                reason: ColorParseError::MissingHashPrefix,
                ..
            }
        ));

        assert!(matches!(
            Color::from_hex("#123").unwrap_err(),
            ThemeLoadError::InvalidColor {
                reason: ColorParseError::InvalidLength,
                ..
            }
        ));
    }

    #[test]
    fn theme_manager_loads_from_directory() {
        let dir = tempdir().unwrap();
        let theme_path = dir.path().join("nightfall.json");
        let json = r##"
        {
            "name": "Nightfall",
            "description": "Dark blue accent",
            "kind": "dark",
            "palette": {
                "background": "#101420",
                "panel": "#141A29",
                "accent": "#3B82F6",
                "accent_text": "#F8FAFC",
                "editor_background": "#0B1120",
                "editor_text": "#E2E8F0",
                "status_bar": "#1F2937"
            },
            "fonts": {
                "ui_family": "Inter",
                "ui_size": 15,
                "editor_family": "JetBrains Mono",
                "editor_size": 14
            }
        }
        "##;
        fs::write(&theme_path, json).unwrap();

        let mut manager = ThemeManager::load_from_dir(dir.path()).unwrap();
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_theme().name, "Nightfall");
        manager.set_active_by_name("Nightfall");
        assert_eq!(manager.active_palette().accent.r, 0x3B);
    }
}
