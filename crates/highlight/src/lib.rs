mod language;
mod theme;
mod udl;

pub use language::{
    builtin, HighlightError, HighlightKind, HighlightToken, LanguageDefinition, LanguageId,
    LanguageRegistry, SyntaxHighlighter,
};
pub use theme::{
    parse_highlight_palette, Color, HighlightPalette, HighlightStyle, ThemeParseError,
};
pub use udl::{Delimiter, UdlDefinition, UdlError};
