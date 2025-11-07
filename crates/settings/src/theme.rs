use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::io::{BufRead, Cursor};
use std::path::{Path, PathBuf};

use crate::json::{self, JsonValue};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use rustnotepad_highlight::{parse_highlight_palette, HighlightPalette, ThemeParseError};
use serde_json::{json, Value as SerdeValue};

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
    syntax_source: Option<SerdeValue>,
}

impl ThemeDefinition {
    pub fn builtin_dark() -> Self {
        let (syntax_palette, syntax_value) = builtin_dark_syntax();
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
            syntax: Some(syntax_palette),
            syntax_source: Some(syntax_value),
        }
    }

    pub fn builtin_light() -> Self {
        let (syntax_palette, syntax_value) = builtin_light_syntax();
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
            syntax: Some(syntax_palette),
            syntax_source: Some(syntax_value),
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

        let (syntax, syntax_source) = match map.get("syntax") {
            Some(value) => {
                let serde_value = to_serde_value(value)?;
                let palette =
                    parse_highlight_palette(&serde_value).map_err(ThemeLoadError::from)?;
                (Some(palette), Some(serde_value))
            }
            None => (None, None),
        };

        Ok(Self {
            name,
            description,
            kind,
            palette,
            fonts,
            syntax,
            syntax_source,
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
        let dict = value.as_dict().ok_or_else(|| {
            ThemeLoadError::InvalidFormat("tmTheme root must be a dictionary".to_string())
        })?;

        let name = dict
            .get("name")
            .and_then(TmValue::as_string)
            .map(|value| value.to_string())
            .unwrap_or_else(|| fallback_name.to_string());

        let settings = dict
            .get("settings")
            .and_then(TmValue::as_array)
            .ok_or_else(|| {
                ThemeLoadError::InvalidFormat("tmTheme missing settings array".to_string())
            })?;

        let general = settings
            .iter()
            .find_map(extract_general_settings)
            .ok_or_else(|| {
                ThemeLoadError::InvalidFormat("tmTheme missing base settings".to_string())
            })?;

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
            syntax_source: None,
        };

        definition.validate()?;
        Ok(definition)
    }
}

impl ThemeDefinition {
    /// Imports a theme from a Notepad++ `stylers.xml` file.
    /// 從 Notepad++ `stylers.xml` 檔案匯入主題。
    pub fn from_notepad_xml(path: impl AsRef<Path>) -> Result<Self, ThemeLoadError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        let fallback = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Imported Notepad++ Theme");
        Self::from_notepad_xml_contents(&contents, fallback)
    }

    fn from_notepad_xml_contents(
        contents: &str,
        fallback_name: &str,
    ) -> Result<Self, ThemeLoadError> {
        let mut reader = Reader::from_reader(Cursor::new(contents.as_bytes()));
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut default_fg = None;
        let mut default_bg = None;
        let mut selection_bg = None;
        let mut caret_color = None;
        let mut root_name = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref element)) => {
                    if element.name().as_ref().eq_ignore_ascii_case(b"WidgetStyle") {
                        process_widget_style(
                            &reader,
                            element,
                            &mut default_fg,
                            &mut default_bg,
                            &mut selection_bg,
                            &mut caret_color,
                        )?;
                    } else if element.name().as_ref().eq_ignore_ascii_case(b"NotepadPlus") {
                        if root_name.is_none() {
                            root_name = attribute_value(&reader, element, &["name"])?;
                        }
                    }
                }
                Ok(Event::Empty(ref element)) => {
                    if element.name().as_ref().eq_ignore_ascii_case(b"WidgetStyle") {
                        process_widget_style(
                            &reader,
                            element,
                            &mut default_fg,
                            &mut default_bg,
                            &mut selection_bg,
                            &mut caret_color,
                        )?;
                    }
                }
                Ok(Event::Eof) => break,
                Err(err) => {
                    return Err(ThemeLoadError::InvalidFormat(format!(
                        "Notepad++ theme parse error: {err}"
                    )));
                }
                _ => {}
            }
            buf.clear();
        }

        let background = default_bg.unwrap_or_else(|| "#1F1F1F".to_string());
        let foreground = default_fg.unwrap_or_else(|| "#E5E7EB".to_string());
        let selection = selection_bg.unwrap_or_else(|| "#2563EB".to_string());
        let caret = caret_color.unwrap_or_else(|| "#93C5FD".to_string());

        let palette = ThemePalette {
            background: background.clone(),
            panel: adjust_panel_color(&background),
            accent: selection.clone(),
            accent_text: foreground.clone(),
            editor_background: background.clone(),
            editor_text: foreground.clone(),
            status_bar: caret.clone(),
        };

        let name = root_name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| fallback_name.to_string());

        let definition = ThemeDefinition {
            name,
            description: Some("Imported from Notepad++ stylers.xml".to_string()),
            kind: infer_kind_from_background(&palette.editor_background)?,
            palette,
            fonts: FontSettings {
                ui_family: "Inter".into(),
                ui_size: 16,
                editor_family: "JetBrains Mono".into(),
                editor_size: 15,
            },
            syntax: None,
            syntax_source: None,
        };
        definition.validate()?;
        Ok(definition)
    }

    /// Imports a theme from a Sublime Text color scheme (`.sublime-color-scheme`).
    /// 從 Sublime `.sublime-color-scheme` 匯入主題。
    pub fn from_sublime_color_scheme(path: impl AsRef<Path>) -> Result<Self, ThemeLoadError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        let fallback = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Imported Sublime Theme");
        let value = parse_sublime_json(&contents)?;
        Self::from_sublime_value(value, fallback)
    }

    fn from_sublime_value(value: SerdeValue, fallback_name: &str) -> Result<Self, ThemeLoadError> {
        let mut variables = HashMap::new();
        if let Some(map) = value.get("variables").and_then(|value| value.as_object()) {
            for (key, entry) in map {
                if let Some(text) = entry.as_str() {
                    variables.insert(key.clone(), text.to_string());
                }
            }
        }

        let globals = value
            .get("globals")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();

        let background = globals
            .get("background")
            .and_then(|entry| resolve_sublime_color(entry, &variables, 0))
            .unwrap_or_else(|| "#1F1F1F".to_string());
        let foreground = globals
            .get("foreground")
            .and_then(|entry| resolve_sublime_color(entry, &variables, 0))
            .unwrap_or_else(|| "#E5E7EB".to_string());
        let selection = globals
            .get("selection")
            .or_else(|| globals.get("highlight"))
            .and_then(|entry| resolve_sublime_color(entry, &variables, 0))
            .unwrap_or_else(|| "#2563EB".to_string());
        let caret = globals
            .get("caret")
            .and_then(|entry| resolve_sublime_color(entry, &variables, 0))
            .unwrap_or_else(|| "#93C5FD".to_string());

        let syntax_value = value
            .get("rules")
            .and_then(|rules| rules.as_array())
            .and_then(|rules| build_sublime_syntax_value(rules, &variables));
        let syntax = match &syntax_value {
            Some(json) => Some(parse_highlight_palette(json).map_err(ThemeLoadError::from)?),
            None => None,
        };

        let description = value
            .get("author")
            .and_then(|v| v.as_str())
            .map(|author| format!("Imported from Sublime color scheme by {author}"))
            .or_else(|| Some("Imported from Sublime color scheme".to_string()));

        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(fallback_name)
            .to_string();

        let palette = ThemePalette {
            background: background.clone(),
            panel: adjust_panel_color(&background),
            accent: selection.clone(),
            accent_text: foreground.clone(),
            editor_background: background.clone(),
            editor_text: foreground.clone(),
            status_bar: caret.clone(),
        };

        let definition = ThemeDefinition {
            name,
            description,
            kind: infer_kind_from_background(&palette.editor_background)?,
            palette,
            fonts: FontSettings {
                ui_family: "Inter".into(),
                ui_size: 16,
                editor_family: "JetBrains Mono".into(),
                editor_size: 15,
            },
            syntax,
            syntax_source: syntax_value,
        };
        definition.validate()?;
        Ok(definition)
    }

    /// Generates a filesystem-friendly slug for this theme.
    pub fn slug(&self) -> String {
        Self::slug_for(&self.name)
    }

    /// Generates a slug for the provided theme name.
    pub fn slug_for(name: &str) -> String {
        slugify(name)
    }

    /// Converts the theme back to its JSON representation.
    pub(crate) fn to_json_value(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), JsonValue::String(self.name.clone()));
        map.insert(
            "description".to_string(),
            self.description
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        );
        map.insert(
            "kind".to_string(),
            JsonValue::String(self.kind.as_str().to_string()),
        );
        map.insert("palette".to_string(), self.palette.to_json());
        map.insert("fonts".to_string(), self.fonts.to_json());
        if let Some(value) = &self.syntax_source {
            map.insert("syntax".to_string(), from_serde_value(value));
        }
        JsonValue::Object(map)
    }

    /// Serializes the theme into a formatted JSON string.
    pub fn to_json_string(&self) -> String {
        json::stringify_pretty(&self.to_json_value(), 2)
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
            "tmTheme parse error (unexpected trailing tokens)".to_string(),
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
                "tmTheme parse error (unterminated processing instruction)".to_string(),
            ))?;
            rest = &rest[end + 2..];
            continue;
        }

        if rest.starts_with("<!--") {
            let end = rest.find("-->").ok_or(ThemeLoadError::InvalidFormat(
                "tmTheme parse error (unterminated comment)".to_string(),
            ))?;
            rest = &rest[end + 3..];
            continue;
        }

        if rest.starts_with("</") {
            let end = rest.find('>').ok_or(ThemeLoadError::InvalidFormat(
                "tmTheme parse error (unterminated closing tag)".to_string(),
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
            let (value, remainder) =
                extract_tag_text(rest, "key").ok_or(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (unterminated <key>)".to_string(),
                ))?;
            tokens.push(TmToken::Key(value));
            rest = remainder;
            continue;
        }
        if rest.starts_with("<string>") {
            let (value, remainder) =
                extract_tag_text(rest, "string").ok_or(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (unterminated <string>)".to_string(),
                ))?;
            tokens.push(TmToken::Text(value));
            rest = remainder;
            continue;
        }
        if rest.starts_with("<integer>") {
            let (value, remainder) =
                extract_tag_text(rest, "integer").ok_or(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (unterminated <integer>)".to_string(),
                ))?;
            tokens.push(TmToken::Text(value));
            rest = remainder;
            continue;
        }
        if rest.starts_with("<real>") {
            let (value, remainder) =
                extract_tag_text(rest, "real").ok_or(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (unterminated <real>)".to_string(),
                ))?;
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
            "tmTheme parse error (unterminated tag)".to_string(),
        ))?;
        rest = &rest[end + 1..];
    }

    Ok(tokens)
}

fn parse_tm_value(stream: &mut TokenStream<'_>) -> Result<TmValue, ThemeLoadError> {
    let Some(token) = stream.next() else {
        return Err(ThemeLoadError::InvalidFormat(
            "tmTheme parse error (unexpected end of input)".to_string(),
        ));
    };
    match token {
        TmToken::StartDict => parse_tm_dict(stream).map(TmValue::Dict),
        TmToken::StartArray => parse_tm_array(stream).map(TmValue::Array),
        TmToken::Text(text) => Ok(TmValue::String(text.to_string())),
        _ => Err(ThemeLoadError::InvalidFormat(
            "tmTheme parse error (unexpected token)".to_string(),
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
                    "tmTheme parse error (unexpected end of input)".to_string(),
                ))
            }
            _ => {
                return Err(ThemeLoadError::InvalidFormat(
                    "tmTheme parse error (expected key)".to_string(),
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
                    "tmTheme parse error (unexpected end of input)".to_string(),
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

fn builtin_dark_syntax() -> (HighlightPalette, SerdeValue) {
    let value = json!({
        "keyword": { "foreground": "#93C5FD", "bold": true },
        "string": { "foreground": "#FBBF24" },
        "comment": { "foreground": "#6B7280", "italic": true },
        "number": { "foreground": "#F97316" },
        "operator": { "foreground": "#A855F7" },
        "identifier": { "foreground": "#E5E7EB" }
    });
    let palette = parse_highlight_palette(&value).expect("builtin syntax palette must be valid");
    (palette, value)
}

fn builtin_light_syntax() -> (HighlightPalette, SerdeValue) {
    let value = json!({
        "keyword": { "foreground": "#1D4ED8", "bold": true },
        "string": { "foreground": "#B45309" },
        "comment": { "foreground": "#9CA3AF", "italic": true },
        "number": { "foreground": "#D97706" },
        "operator": { "foreground": "#6366F1" },
        "identifier": { "foreground": "#1F2937" }
    });
    let palette = parse_highlight_palette(&value).expect("builtin syntax palette must be valid");
    (palette, value)
}

fn process_widget_style<B: BufRead>(
    reader: &Reader<B>,
    element: &BytesStart<'_>,
    default_fg: &mut Option<String>,
    default_bg: &mut Option<String>,
    selection_bg: &mut Option<String>,
    caret_color: &mut Option<String>,
) -> Result<(), ThemeLoadError> {
    let name = attribute_value(reader, element, &["name"])?.unwrap_or_default();
    let fg = attribute_value(reader, element, &["fgColor", "fgcolor"])?;
    let bg = attribute_value(reader, element, &["bgColor", "bgcolor"])?;
    if matches_notepad_name(&name, &["Default Style", "Default text"]) {
        if default_fg.is_none() {
            if let Some(value) = fg.as_deref().and_then(|text| normalize_hex_color(text)) {
                *default_fg = Some(value);
            }
        }
        if default_bg.is_none() {
            if let Some(value) = bg.as_deref().and_then(|text| normalize_hex_color(text)) {
                *default_bg = Some(value);
            }
        }
    } else if matches_notepad_name(&name, &["Selected text colour", "Selected text color"]) {
        if selection_bg.is_none() {
            if let Some(value) = bg.as_deref().and_then(|text| normalize_hex_color(text)) {
                *selection_bg = Some(value);
            }
        }
    } else if matches_notepad_name(&name, &["Caret colour", "Caret color"]) {
        if caret_color.is_none() {
            if let Some(value) = fg
                .as_deref()
                .and_then(|text| normalize_hex_color(text))
                .or_else(|| bg.as_deref().and_then(|text| normalize_hex_color(text)))
            {
                *caret_color = Some(value);
            }
        }
    }
    Ok(())
}

fn attribute_value<B: BufRead>(
    reader: &Reader<B>,
    element: &BytesStart<'_>,
    keys: &[&str],
) -> Result<Option<String>, ThemeLoadError> {
    for attribute in element.attributes().with_checks(false) {
        let attribute = attribute.map_err(|err| {
            ThemeLoadError::InvalidFormat(format!("XML attribute parse error: {err}"))
        })?;
        for key in keys {
            if attribute.key.as_ref().eq_ignore_ascii_case(key.as_bytes()) {
                let value = attribute
                    .decode_and_unescape_value(reader)
                    .map_err(|err| {
                        ThemeLoadError::InvalidFormat(format!("XML attribute decode error: {err}"))
                    })?
                    .into_owned();
                return Ok(Some(value));
            }
        }
    }
    Ok(None)
}

fn matches_notepad_name(input: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| input.eq_ignore_ascii_case(candidate))
}

fn adjust_panel_color(base: &str) -> String {
    if let Ok(color) = Color::from_hex(base) {
        let average = (color.r as u32 + color.g as u32 + color.b as u32) / 3;
        let delta: i16 = if average > 180 { -12 } else { 12 };
        let adjust = |component: u8| -> u8 {
            let value = component as i16 + delta;
            value.clamp(0, 255) as u8
        };
        format!(
            "#{:02X}{:02X}{:02X}",
            adjust(color.r),
            adjust(color.g),
            adjust(color.b)
        )
    } else {
        base.to_string()
    }
}

fn normalize_hex_color(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(color) = parse_rgb_function(trimmed) {
        return Some(color);
    }
    let mut hex = trimmed
        .trim_start_matches('#')
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .to_string();
    if hex.len() == 3 {
        let mut expanded = String::new();
        for ch in hex.chars() {
            expanded.push(ch);
            expanded.push(ch);
        }
        hex = expanded;
    }
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    if hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Some(format!("#{}", hex.to_uppercase()));
    }
    None
}

fn parse_rgb_function(input: &str) -> Option<String> {
    let normalized = input.trim();
    let (func, rest) = normalized.split_once('(')?;
    let func = func.trim().to_ascii_lowercase();
    if func != "rgb" && func != "rgba" {
        return None;
    }
    let args = rest.strip_suffix(')')?;
    let parts: Vec<_> = args.split(',').map(|part| part.trim()).collect();
    if parts.len() < 3 {
        return None;
    }
    let r = parse_rgb_component(parts[0])?;
    let g = parse_rgb_component(parts[1])?;
    let b = parse_rgb_component(parts[2])?;
    let a = if func == "rgba" && parts.len() >= 4 {
        parse_alpha_component(parts[3])?
    } else {
        255
    };
    if a == 255 {
        Some(format!("#{:02X}{:02X}{:02X}", r, g, b))
    } else {
        Some(format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a))
    }
}

fn parse_rgb_component(value: &str) -> Option<u8> {
    if value.ends_with('%') {
        let perc = value[..value.len() - 1].trim().parse::<f32>().ok()?;
        return Some((perc.clamp(0.0, 100.0) * 2.55).round() as u8);
    }
    let number = value.parse::<f32>().ok()?;
    if number <= 1.0 {
        Some((number.clamp(0.0, 1.0) * 255.0).round() as u8)
    } else {
        Some(number.clamp(0.0, 255.0).round() as u8)
    }
}

fn parse_alpha_component(value: &str) -> Option<u8> {
    if value.ends_with('%') {
        let perc = value[..value.len() - 1].trim().parse::<f32>().ok()?;
        return Some((perc.clamp(0.0, 100.0) * 2.55).round() as u8);
    }
    let number = value.parse::<f32>().ok()?;
    if number <= 1.0 {
        Some((number.clamp(0.0, 1.0) * 255.0).round() as u8)
    } else {
        Some(number.clamp(0.0, 255.0).round() as u8)
    }
}

fn resolve_sublime_color(
    value: &SerdeValue,
    variables: &HashMap<String, String>,
    depth: u8,
) -> Option<String> {
    if depth > 6 {
        return None;
    }
    match value {
        SerdeValue::String(text) => {
            let trimmed = text.trim();
            if let Some(inner) = trimmed
                .strip_prefix("var(")
                .and_then(|rest| rest.strip_suffix(')'))
            {
                let key = inner.trim();
                if let Some(variable) = variables.get(key) {
                    return resolve_sublime_color(
                        &SerdeValue::String(variable.clone()),
                        variables,
                        depth + 1,
                    );
                }
            }
            normalize_hex_color(trimmed)
        }
        _ => None,
    }
}

fn build_sublime_syntax_value(
    rules: &[SerdeValue],
    variables: &HashMap<String, String>,
) -> Option<SerdeValue> {
    let mut map = serde_json::Map::new();
    for rule in rules {
        let obj = match rule.as_object() {
            Some(obj) => obj,
            None => continue,
        };
        let scope = match obj.get("scope").and_then(|value| value.as_str()) {
            Some(scope) => scope,
            None => continue,
        };
        let key = match classify_scope(scope) {
            Some(key) => key,
            None => continue,
        };
        if map.contains_key(key) {
            continue;
        }
        let color = match obj
            .get("foreground")
            .and_then(|entry| resolve_sublime_color(entry, variables, 0))
        {
            Some(color) => color,
            None => continue,
        };
        let mut entry = serde_json::Map::new();
        entry.insert("foreground".to_string(), SerdeValue::String(color));
        if let Some(style) = obj.get("font_style").and_then(|value| value.as_str()) {
            let lower = style.to_ascii_lowercase();
            if lower.contains("bold") {
                entry.insert("bold".to_string(), SerdeValue::Bool(true));
            }
            if lower.contains("italic") {
                entry.insert("italic".to_string(), SerdeValue::Bool(true));
            }
            if lower.contains("underline") {
                entry.insert("underline".to_string(), SerdeValue::Bool(true));
            }
        }
        map.insert(key.to_string(), SerdeValue::Object(entry));
    }
    if map.is_empty() {
        None
    } else {
        Some(SerdeValue::Object(map))
    }
}

fn classify_scope(scope: &str) -> Option<&'static str> {
    for raw in scope.split(|ch| ch == ',' || ch == ' ') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if lower.contains("keyword") {
            return Some("keyword");
        }
        if lower.contains("string") {
            return Some("string");
        }
        if lower.contains("comment") {
            return Some("comment");
        }
        if lower.contains("constant.numeric") || lower.contains("number") {
            return Some("number");
        }
        if lower.contains("punctuation") || lower.contains("operator") {
            return Some("operator");
        }
        if lower.contains("variable") || lower.contains("entity.name") {
            return Some("identifier");
        }
    }
    None
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_whitespace() || ch == '-' || ch == '_' {
            if !slug.ends_with('-') && !slug.is_empty() {
                slug.push('-');
            }
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "theme".to_string()
    } else {
        trimmed
    }
}

fn parse_sublime_json(input: &str) -> Result<SerdeValue, ThemeLoadError> {
    let cleaned = strip_json_comments(input);
    serde_json::from_str(&cleaned).map_err(|err| {
        ThemeLoadError::InvalidFormat(format!("Sublime color scheme parse error: {err}"))
    })
}

fn strip_json_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escape = false;
    while let Some(ch) = chars.next() {
        if in_string {
            if !escape && ch == '"' {
                in_string = false;
            }
            escape = !escape && ch == '\\';
            output.push(ch);
        } else {
            match ch {
                '"' => {
                    in_string = true;
                    output.push(ch);
                }
                '/' => match chars.peek() {
                    Some('/') => {
                        chars.next();
                        while let Some(next) = chars.next() {
                            if next == '\n' || next == '\r' {
                                output.push('\n');
                                break;
                            }
                        }
                    }
                    Some('*') => {
                        chars.next();
                        let mut prev = '\0';
                        while let Some(next) = chars.next() {
                            if prev == '*' && next == '/' {
                                break;
                            }
                            prev = next;
                        }
                    }
                    _ => output.push('/'),
                },
                _ => output.push(ch),
            }
        }
    }
    output
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

fn from_serde_value(value: &SerdeValue) -> JsonValue {
    match value {
        SerdeValue::Null => JsonValue::Null,
        SerdeValue::Bool(flag) => JsonValue::Bool(*flag),
        SerdeValue::Number(num) => {
            if let Some(val) = num.as_f64() {
                JsonValue::Number(val)
            } else if let Some(val) = num.as_i64() {
                JsonValue::Number(val as f64)
            } else if let Some(val) = num.as_u64() {
                JsonValue::Number(val as f64)
            } else {
                JsonValue::Number(0.0)
            }
        }
        SerdeValue::String(text) => JsonValue::String(text.clone()),
        SerdeValue::Array(items) => JsonValue::Array(items.iter().map(from_serde_value).collect()),
        SerdeValue::Object(map) => {
            let mut object = BTreeMap::new();
            for (key, item) in map {
                object.insert(key.clone(), from_serde_value(item));
            }
            JsonValue::Object(object)
        }
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

    fn as_str(&self) -> &'static str {
        match self {
            ThemeKind::Dark => "dark",
            ThemeKind::Light => "light",
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

    fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert(
            "background".into(),
            JsonValue::String(self.background.clone()),
        );
        map.insert("panel".into(), JsonValue::String(self.panel.clone()));
        map.insert("accent".into(), JsonValue::String(self.accent.clone()));
        map.insert(
            "accent_text".into(),
            JsonValue::String(self.accent_text.clone()),
        );
        map.insert(
            "editor_background".into(),
            JsonValue::String(self.editor_background.clone()),
        );
        map.insert(
            "editor_text".into(),
            JsonValue::String(self.editor_text.clone()),
        );
        map.insert(
            "status_bar".into(),
            JsonValue::String(self.status_bar.clone()),
        );
        JsonValue::Object(map)
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

    fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert(
            "ui_family".into(),
            JsonValue::String(self.ui_family.clone()),
        );
        map.insert("ui_size".into(), JsonValue::Number(self.ui_size as f64));
        map.insert(
            "editor_family".into(),
            JsonValue::String(self.editor_family.clone()),
        );
        map.insert(
            "editor_size".into(),
            JsonValue::Number(self.editor_size as f64),
        );
        JsonValue::Object(map)
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
        Self::load_from_dirs(std::iter::once(path))
    }

    pub fn load_from_dirs<I, P>(paths: I) -> Result<Self, ThemeLoadError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut definitions = Vec::new();
        for path in paths {
            Self::collect_definitions(path.as_ref(), &mut definitions)?;
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

    fn collect_definitions(
        dir: &Path,
        definitions: &mut Vec<ThemeDefinition>,
    ) -> Result<(), ThemeLoadError> {
        if !dir.is_dir() {
            return Ok(());
        }
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
        Ok(())
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
    InvalidFormat(String),
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
