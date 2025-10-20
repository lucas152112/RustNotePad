use std::collections::HashMap;

use crate::language::HighlightKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightStyle {
    pub foreground: Color,
    pub background: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Default, Clone)]
pub struct HighlightPalette {
    keyword: Option<HighlightStyle>,
    string: Option<HighlightStyle>,
    comment: Option<HighlightStyle>,
    number: Option<HighlightStyle>,
    operator: Option<HighlightStyle>,
    identifier: Option<HighlightStyle>,
    custom: HashMap<String, HighlightStyle>,
}

impl HighlightPalette {
    pub fn style_for(&self, kind: &HighlightKind) -> Option<&HighlightStyle> {
        match kind {
            HighlightKind::Keyword => self.keyword.as_ref(),
            HighlightKind::String => self.string.as_ref(),
            HighlightKind::Comment => self.comment.as_ref(),
            HighlightKind::Number => self.number.as_ref(),
            HighlightKind::Operator => self.operator.as_ref(),
            HighlightKind::Identifier => self.identifier.as_ref(),
            HighlightKind::Custom(name) => self.custom.get(name),
        }
    }

    pub fn insert_standard(&mut self, key: &str, style: HighlightStyle) {
        match key {
            "keyword" => self.keyword = Some(style),
            "string" => self.string = Some(style),
            "comment" => self.comment = Some(style),
            "number" => self.number = Some(style),
            "operator" => self.operator = Some(style),
            "identifier" => self.identifier = Some(style),
            _ => {
                self.custom.insert(key.to_string(), style);
            }
        }
    }

    pub fn insert_custom(&mut self, name: impl Into<String>, style: HighlightStyle) {
        self.custom.insert(name.into(), style);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeParseError {
    #[error("syntax section missing")]
    MissingSyntax,
    #[error("syntax entries must be objects")]
    InvalidSyntax,
    #[error("invalid color '{value}': {reason}")]
    InvalidColor {
        value: String,
        reason: ColorParseError,
    },
    #[error("entries must contain foreground color")]
    MissingForeground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorParseError {
    MissingHash,
    InvalidLength,
    InvalidHex,
}

impl std::fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorParseError::MissingHash => write!(f, "missing leading '#'"),
            ColorParseError::InvalidLength => write!(f, "expected 6 or 8 hex digits"),
            ColorParseError::InvalidHex => write!(f, "contains non-hex digits"),
        }
    }
}

pub fn parse_highlight_palette(
    syntax_value: &serde_json::Value,
) -> Result<HighlightPalette, ThemeParseError> {
    let map = syntax_value
        .as_object()
        .ok_or(ThemeParseError::InvalidSyntax)?;
    let mut palette = HighlightPalette::default();
    for (name, entry) in map {
        let style = parse_style(entry)?;
        palette.insert_standard(name, style);
    }
    Ok(palette)
}

fn parse_style(value: &serde_json::Value) -> Result<HighlightStyle, ThemeParseError> {
    let map = value.as_object().ok_or(ThemeParseError::InvalidSyntax)?;

    let foreground = map
        .get("foreground")
        .and_then(|value| value.as_str())
        .ok_or(ThemeParseError::MissingForeground)?;
    let foreground = parse_color(foreground).map_err(|reason| ThemeParseError::InvalidColor {
        value: foreground.to_string(),
        reason,
    })?;

    let background = map
        .get("background")
        .and_then(|value| value.as_str())
        .map(|value| {
            parse_color(value).map_err(|reason| ThemeParseError::InvalidColor {
                value: value.to_string(),
                reason,
            })
        })
        .transpose()?;

    let bold = map
        .get("bold")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let italic = map
        .get("italic")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let underline = map
        .get("underline")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    Ok(HighlightStyle {
        foreground,
        background,
        bold,
        italic,
        underline,
    })
}

fn parse_color(input: &str) -> Result<Color, ColorParseError> {
    let trimmed = input.trim();
    let hex = trimmed
        .strip_prefix('#')
        .ok_or(ColorParseError::MissingHash)?;
    if hex.len() != 6 && hex.len() != 8 {
        return Err(ColorParseError::InvalidLength);
    }
    let mut components = [0u8; 4];
    for index in 0..(hex.len() / 2) {
        let slice = &hex[index * 2..index * 2 + 2];
        components[index] =
            u8::from_str_radix(slice, 16).map_err(|_| ColorParseError::InvalidHex)?;
    }
    if hex.len() == 6 {
        components[3] = 255;
    }
    Ok(Color {
        r: components[0],
        g: components[1],
        b: components[2],
        a: components[3],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_palette() {
        let value = json!({
            "keyword": {
                "foreground": "#FFAA00",
                "bold": true
            },
            "string": {
                "foreground": "#11AAFF",
                "italic": true
            },
            "custom.debug": {
                "foreground": "#CCCCCC"
            }
        });

        let palette = parse_highlight_palette(&value).unwrap();
        assert!(palette.style_for(&HighlightKind::Keyword).is_some());
        assert!(palette.style_for(&HighlightKind::String).is_some());
        let custom = HighlightKind::Custom("custom.debug".into());
        assert!(palette.style_for(&custom).is_some());
    }
}
