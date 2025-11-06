use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::json::{self, JsonValue};
use rustnotepad_highlight::{parse_highlight_palette, HighlightPalette, ThemeParseError};
use serde_json::json;

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

#[derive(Debug, Clone)]
pub struct ThemeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub kind: ThemeKind,
    pub palette: ThemePalette,
    pub fonts: FontSettings,
    pub syntax: Option<HighlightPalette>,
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
            syntax: Some(builtin_dark_syntax()),
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
            syntax: Some(builtin_light_syntax()),
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

        let syntax = match map.get("syntax") {
            Some(value) => Some(parse_syntax_palette(value)?),
            None => None,
        };

        Ok(Self {
            name,
            description,
            kind,
            palette,
            fonts,
            syntax,
        })
    }

    pub fn syntax_palette(&self) -> Option<&HighlightPalette> {
        self.syntax.as_ref()
    }

    /// Imports a theme from a TextMate `.tmTheme` file.
    /// 從 TextMate `.tmTheme` 檔案匯入主題。
    pub fn from_tmtheme_file(path: impl AsRef<Path>) -> Result<Self, ThemeLoadError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        let value = parse_tmtheme(&contents)?;
        let fallback = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Imported tmTheme");
        Self::from_tmtheme_value(&value, fallback)
    }

    fn from_tmtheme_value(value: &TmValue, fallback_name: &str) -> Result<Self, ThemeLoadError> {
        let dict = value
            .as_dict()
            .ok_or_else(|| ThemeLoadError::InvalidFormat("tmTheme root must be a dictionary"))?;

        let name = dict
            .get("name")
            .and_then(TmValue::as_string)
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback_name.to_string());

        let settings = dict
            .get("settings")
            .and_then(TmValue::as_array)
            .ok_or_else(|| ThemeLoadError::InvalidFormat("tmTheme missing settings array"))?;

        let general = settings
            .iter()
            .find_map(extract_general_settings)
            .ok_or_else(|| ThemeLoadError::InvalidFormat("tmTheme missing base settings"))?;

        let foreground = general
            .get("foreground")
            .and_then(TmValue::as_string)
            .unwrap_or("#D4D4D4");
        let background = general
            .get("background")
            .and_then(TmValue::as_string)
            .unwrap_or("#1E1E1E");
        let caret = general
            .get("caret")
            .and_then(TmValue::as_string)
            .unwrap_or("#569CD6");
        let selection = general
            .get("selection")
            .and_then(TmValue::as_string)
            .unwrap_or("#264F78");

        let palette = ThemePalette {
            background: background.to_string(),
            panel: background.to_string(),
            accent: selection.to_string(),
            accent_text: foreground.to_string(),
            editor_background: background.to_string(),
            editor_text: foreground.to_string(),
            status_bar: caret.to_string(),
        };

        let kind = infer_kind_from_background(&palette.editor_background)?;
        let description = Some("Imported from TextMate tmTheme".to_string());

        let definition = ThemeDefinition {
            name,
            description,
            kind,
            palette,
            fonts: FontSettings {
                ui_family: "Inter".into(),
                ui_size: 16,
                editor_family: "JetBrains Mono".into(),
                editor_size: 15,
            },
            syntax: None,
        };

        definition.validate()?;
        Ok(definition)
    }
}

// TextMate `.tmTheme` parsing helpers.
// TextMate `.tmTheme` 解析工具集合。
#[derive(Debug, Clone)]
enum TmValue {
    String(String),
    Dict(BTreeMap<String, TmValue>),
    Array(Vec<TmValue>),
}

impl TmValue {
    fn as_string(&self) -> Option<&str> {
        match self {
            TmValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    fn as_dict(&self) -> Option<&BTreeMap<String, TmValue>> {
        match self {
            TmValue::Dict(map) => Some(map),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[TmValue]> {
        match self {
            TmValue::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
enum TmToken<'a> {
    StartDict,
    EndDict,
    StartArray,
    EndArray,
    Key(&'a str),
    Text(&'a str),
}

struct TokenStream<'a> {
    tokens: Vec<TmToken<'a>>,
    index: usize,
}

impl<'a> TokenStream<'a> {
    fn new(tokens: Vec<TmToken<'a>>) -> Self {
        Self { tokens, index: 0 }
    }

    fn peek(&self) -> Option<&TmToken<'a>> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<TmToken<'a>> {
        if let Some(token) = self.tokens.get(self.index) {
            self.index += 1;
            Some(token.clone())
        } else {
            None
        }
    }
}

fn parse_tmtheme(input: &str) -> Result<TmValue, ThemeLoadError> {
    let tokens = tokenize_tmtheme(input)?;
    let mut stream = TokenStream::new(tokens);
    let value = parse_tm_value(&mut stream)?;
    if stream.peek().is_some() {
        return Err(ThemeLoadError::InvalidFormat(
            "tmTheme parse error (unexpected trailing tokens)",
        ));
    }
    Ok(value)
}

fn tokenize_tmtheme(input: &str) -> Result<Vec<TmToken<'_>>, ThemeLoadError> {
    let mut tokens = Vec::new();
    let mut rest = input;

    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        let Some(idx) = rest.find('<') else {
            break;
        };
        if idx > 0 {
            rest = &rest[idx..];
        }

        if rest.starts_with("<?") {
            let end = rest.find("?>").ok_or(ThemeLoadError::InvalidFormat(
                "tmTheme parse error (unterminated processing instruction)",
            ))?;
            rest = &rest[end + 2..];
            continue;
        }

        if rest.starts_with("<!--") {
            let end = rest.find("-->").ok_or(ThemeLoadError::InvalidFormat(
                "tmTheme parse error (unterminated comment)",
            ))?;
            rest = &rest[end + 3..];
            continue;
        }

        if rest.starts_with("</") {
            let end = rest.find('>').ok_or(ThemeLoadError::InvalidFormat(
                "tmTheme parse error (unterminated closing tag)",
            ))?;
            let tag = &rest[2..end];
            match tag {
                "dict" => tokens.push(TmToken::EndDict),
                "array" => tokens.push(TmToken::EndArray),
                _ => {}
            }
            rest = &rest[end + 1..];
            continue;
        }

        if rest.starts_with("<dict>") {
            tokens.push(TmToken::StartDict);
            rest = &rest["<dict>".len()..];
            continue;
        }
        if rest.starts_with("<array>") {
            tokens.push(TmToken::StartArray);
            rest = &rest["<array>".len()..];
            continue;
        }
        if rest.starts_with("<key>") {
            let (value, remainder) = extract_tag_text(rest, "key").ok_or(
                ThemeLoadError::InvalidFormat("tmTheme parse error (unterminated <key>)"),
            )?;
            tokens.push(TmToken::Key(value));
            rest = remainder;
            continue;
        }
        if rest.starts_with("<string>") {
            let (value, remainder) = extract_tag_text(rest, "string").ok_or(
                ThemeLoadError::InvalidFormat("tmTheme parse error (unterminated <string>)"),
            )?;
            tokens.push(TmToken::Text(value));
            rest = remainder;
            continue;
        }
        if rest.starts_with("<integer>") {
            let (value, remainder) = extract_tag_text(rest, "integer").ok_or(
                ThemeLoadError::InvalidFormat("tmTheme parse error (unterminated <integer>)"),
            )?;
            tokens.push(TmToken::Text(value));
            rest = remainder;
            continue;
        }
        if rest.starts_with("<real>") {
            let (value, remainder) = extract_tag_text(rest, "real").ok_or(
                ThemeLoadError::InvalidFormat("tmTheme parse error (unterminated <real>)"),
            )?;
            tokens.push(TmToken::Text(value));
            rest = remainder;
            continue;
        }
        if rest.starts_with("<true/>") {
            tokens.push(TmToken::Text("true"));
            rest = &rest["<true/>".len()..];
            continue;
        }
        if rest.starts_with("<false/>") {
            tokens.push(TmToken::Text("false"));
            rest = &rest["<false/>".len()..];
            continue;
        }

        let end = rest.find('>').ok_or(ThemeLoadError::InvalidFormat(
            "tmTheme parse error (unterminated tag)",
        ))?;
        rest = &rest[end + 1..];
    }

    Ok(tokens)
}

fn parse_tm_value(stream: &mut TokenStream<'_>) -> Result<TmValue, ThemeLoadError> {
    let Some(token) = stream.next() else {
        return Err(ThemeLoadError::InvalidFormat(
            "tmTheme parse error (unexpected end of input)",
        ));
    };
    match token {
        TmToken::StartDict => parse_tm_dict(stream).map(TmValue::Dict),
        TmToken::StartArray => parse_tm_array(stream).map(TmValue::Array),
        TmToken::Text(text) => Ok(TmValue::String(text.to_string())),
        _ => Err(ThemeLoadError::InvalidFormat(
            "tmTheme parse error (unexpected token)",
        )),
    }
}

fn parse_tm_dict(
    stream: &mut TokenStream<'_>,
) -> Result<BTreeMap<String, TmValue>, ThemeLoadError> {
    let mut map = BTreeMap::new();
    loop {
        match stream.peek() {
            Some(TmToken::EndDict) => {
                stream.next();
                break;
            }
            Some(TmToken::Key(key)) => {
                let key = key.to_string();
                stream.next();
                let value = parse_tm_value(stream)?;
                map.insert(key, value);
            }
            None => {
                return Err(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (unexpected end of input)",
                ))
            }
            _ => {
                return Err(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (expected key)",
                ))
            }
        }
    }
    Ok(map)
}

fn parse_tm_array(stream: &mut TokenStream<'_>) -> Result<Vec<TmValue>, ThemeLoadError> {
    let mut items = Vec::new();
    loop {
        match stream.peek() {
            Some(TmToken::EndArray) => {
                stream.next();
                break;
            }
            Some(_) => {
                let value = parse_tm_value(stream)?;
                items.push(value);
            }
            None => {
                return Err(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (unexpected end of input)",
                ))
            }
        }
    }
    Ok(items)
}

fn extract_tag_text<'a>(input: &'a str, tag: &str) -> Option<(&'a str, &'a str)> {
    let open_end = input.find('>')?;
    let close_tag = format!("</{tag}>");
    let close_pos = input.find(&close_tag)?;
    let content = &input[open_end + 1..close_pos];
    let trimmed = content.trim();
    let remainder = &input[close_pos + close_tag.len()..];
    Some((trimmed, remainder))
}

fn builtin_dark_syntax() -> HighlightPalette {
    parse_highlight_palette(&json!({
        "keyword": { "foreground": "#93C5FD", "bold": true },
        "string": { "foreground": "#FBBF24" },
        "comment": { "foreground": "#6B7280", "italic": true },
        "number": { "foreground": "#F97316" },
        "operator": { "foreground": "#A855F7" },
        "identifier": { "foreground": "#E5E7EB" }
    }))
    .expect("builtin syntax palette must be valid")
}

fn builtin_light_syntax() -> HighlightPalette {
    parse_highlight_palette(&json!({
        "keyword": { "foreground": "#1D4ED8", "bold": true },
        "string": { "foreground": "#B45309" },
        "comment": { "foreground": "#9CA3AF", "italic": true },
        "number": { "foreground": "#D97706" },
        "operator": { "foreground": "#6366F1" },
        "identifier": { "foreground": "#1F2937" }
    }))
    .expect("builtin syntax palette must be valid")
}

fn parse_syntax_palette(value: &JsonValue) -> Result<HighlightPalette, ThemeLoadError> {
    let serde_value = to_serde_value(value)?;
    parse_highlight_palette(&serde_value).map_err(ThemeLoadError::from)
}

fn extract_general_settings(entry: &TmValue) -> Option<&BTreeMap<String, TmValue>> {
    let dict = entry.as_dict()?;
    if dict.contains_key("scope") {
        return None;
    }
    dict.get("settings")?.as_dict()
}

fn infer_kind_from_background(hex: &str) -> Result<ThemeKind, ThemeLoadError> {
    let color = Color::from_hex(hex)?;
    let luminance = relative_luminance(&color);
    Ok(if luminance >= 0.6 {
        ThemeKind::Light
    } else {
        ThemeKind::Dark
    })
}

fn relative_luminance(color: &Color) -> f32 {
    fn channel(value: u8) -> f32 {
        let normalized = value as f32 / 255.0;
        if normalized <= 0.03928 {
            normalized / 12.92
        } else {
            ((normalized + 0.055) / 1.055).powf(2.4)
        }
    }

    let r = channel(color.r);
    let g = channel(color.g);
    let b = channel(color.b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn to_serde_value(value: &JsonValue) -> Result<serde_json::Value, ThemeLoadError> {
    Ok(match value {
        JsonValue::Null => serde_json::Value::Null,
        JsonValue::Bool(flag) => serde_json::Value::Bool(*flag),
        JsonValue::Number(num) => serde_json::Number::from_f64(*num)
            .map(serde_json::Value::Number)
            .ok_or(ThemeLoadError::InvalidField("syntax"))?,
        JsonValue::String(text) => serde_json::Value::String(text.clone()),
        JsonValue::Array(values) => {
            let mut array = Vec::with_capacity(values.len());
            for item in values {
                array.push(to_serde_value(item)?);
            }
            serde_json::Value::Array(array)
        }
        JsonValue::Object(map) => {
            let mut object = serde_json::Map::with_capacity(map.len());
            for (key, item) in map {
                object.insert(key.clone(), to_serde_value(item)?);
            }
            serde_json::Value::Object(object)
        }
    })
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
    InvalidFormat(&'static str),
    Empty,
    Syntax(ThemeParseError),
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
            ThemeLoadError::InvalidFormat(reason) => write!(f, "{reason}"),
            ThemeLoadError::Empty => write!(f, "no theme definitions were provided"),
            ThemeLoadError::Syntax(err) => err.fmt(f),
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

impl From<ThemeParseError> for ThemeLoadError {
    fn from(value: ThemeParseError) -> Self {
        ThemeLoadError::Syntax(value)
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
            },
            "syntax": {
                "keyword": { "foreground": "#35A2FF" },
                "comment": { "foreground": "#607090" }
            }
        }
        "##;
        fs::write(&theme_path, json).unwrap();

        let mut manager = ThemeManager::load_from_dir(dir.path()).unwrap();
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_theme().name, "Nightfall");
        manager.set_active_by_name("Nightfall");
        assert_eq!(manager.active_palette().accent.r, 0x3B);
        assert!(manager
            .active_theme()
            .syntax_palette()
            .and_then(|palette| palette.style_for(&rustnotepad_highlight::HighlightKind::Keyword))
            .is_some());
    }

    #[test]
    fn syntax_palette_is_optional() {
        let json = json::parse(
            r##"
            {
                "name": "Minimal",
                "kind": "dark",
                "palette": {
                    "background": "#000000",
                    "panel": "#000000",
                    "accent": "#FFFFFF",
                    "accent_text": "#000000",
                    "editor_background": "#000000",
                    "editor_text": "#FFFFFF",
                    "status_bar": "#000000"
                },
                "fonts": {
                    "ui_family": "Inter",
                    "ui_size": 14,
                    "editor_family": "JetBrains Mono",
                    "editor_size": 13
                }
            }
        "##,
        )
        .unwrap();
        let theme = ThemeDefinition::from_json(&json).unwrap();
        assert!(theme.syntax_palette().is_none());
    }
}
