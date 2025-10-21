use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;

use regex::{Regex, RegexBuilder};
use thiserror::Error;

use crate::udl::{Delimiter, UdlDefinition};

const NUMBER_PATTERN: &str = r"(?x)
    (?P<number>
        (?:
            0[xX][0-9A-Fa-f_]+ |
            0[bB][01_]+ |
            0[oO][0-7_]+ |
            [0-9][0-9_]*(\.[0-9_]+)?([eE][+-]?[0-9_]+)?
        )
    )
";

/// Identifier for a registered language.
/// （註冊語言的識別子。）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageId(Cow<'static, str>);

impl LanguageId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for LanguageId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for LanguageId {
    fn from(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl From<String> for LanguageId {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

#[derive(Debug, Clone)]
pub struct LanguageDefinition {
    pub id: LanguageId,
    pub display_name: String,
    pub extensions: Vec<String>,
    pub case_sensitive: bool,
    pub keywords: Vec<String>,

    keyword_regex: Option<Regex>,
    operator_regex: Option<Regex>,
    number_regex: Regex,
    line_comment: Option<String>,
    block_comment: Option<BlockComment>,
    string_delimiters: Vec<StringDelimiter>,
    additional_rules: Vec<PatternRule>,
}

impl LanguageDefinition {
    pub fn from_udl(udl: UdlDefinition) -> Result<Self, HighlightError> {
        let id = LanguageId::from(udl.identifier.clone().unwrap_or_else(|| udl.name.clone()));
        let keywords = udl.keywords.clone();
        let keyword_regex = build_keyword_regex(&keywords, udl.case_sensitive)?;
        let operator_regex = build_operator_regex(&udl.operators)?;
        let number_regex = build_number_regex(udl.number_pattern.as_deref())?;
        let line_comment = udl.line_comment.clone();
        let block_comment = udl
            .block_comment
            .clone()
            .map(|(start, end)| BlockComment { start, end });
        let string_delimiters = udl
            .delimiters
            .iter()
            .map(|Delimiter { start, end, escape }| StringDelimiter {
                start: start.clone(),
                end: end.clone().unwrap_or_else(|| start.clone()),
                escape: *escape,
            })
            .collect();

        Ok(Self {
            id,
            display_name: udl.name,
            extensions: udl.extensions,
            case_sensitive: udl.case_sensitive,
            keywords,
            keyword_regex,
            operator_regex,
            number_regex,
            line_comment,
            block_comment,
            string_delimiters,
            additional_rules: Vec::new(),
        })
    }

    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    pub fn highlight(&self, input: &str) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        if input.is_empty() {
            return tokens;
        }
        let mut occupied = vec![false; input.len()];

        if let Some(block) = &self.block_comment {
            highlight_block_comments(block, input, &mut tokens, &mut occupied);
        }

        if let Some(line_comment) = &self.line_comment {
            highlight_line_comments(line_comment, input, &mut tokens, &mut occupied);
        }

        for delimiter in &self.string_delimiters {
            highlight_strings(delimiter, input, &mut tokens, &mut occupied);
        }

        if let Some(regex) = &self.keyword_regex {
            highlight_with_regex(
                regex,
                HighlightKind::Keyword,
                input,
                &mut tokens,
                &mut occupied,
            );
        }

        highlight_with_regex(
            &self.number_regex,
            HighlightKind::Number,
            input,
            &mut tokens,
            &mut occupied,
        );

        if let Some(regex) = &self.operator_regex {
            highlight_with_regex(
                regex,
                HighlightKind::Operator,
                input,
                &mut tokens,
                &mut occupied,
            );
        }

        for rule in &self.additional_rules {
            highlight_with_regex(
                &rule.regex,
                rule.kind.clone(),
                input,
                &mut tokens,
                &mut occupied,
            );
        }

        tokens.sort_by_key(|token| token.range.start);
        tokens
    }
}

#[derive(Debug, Clone)]
struct BlockComment {
    start: String,
    end: String,
}

#[derive(Debug, Clone)]
struct StringDelimiter {
    start: String,
    end: String,
    escape: Option<char>,
}

#[derive(Debug, Clone)]
struct PatternRule {
    regex: Regex,
    kind: HighlightKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HighlightKind {
    Keyword,
    Comment,
    String,
    Number,
    Operator,
    Identifier,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightToken {
    pub range: Range<usize>,
    pub kind: HighlightKind,
}

#[derive(Debug, Error)]
pub enum HighlightError {
    #[error("language '{0}' is not registered")]
    LanguageNotRegistered(String),
    #[error("regex compilation failed: {0}")]
    RegexCompilation(String),
    #[error(transparent)]
    Udl(#[from] crate::udl::UdlError),
}

#[derive(Default)]
pub struct LanguageRegistry {
    languages: HashMap<String, LanguageDefinition>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            languages: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        for language in builtin::builtins() {
            registry
                .register(language)
                .expect("built-in language registration must succeed");
        }
        registry
    }

    pub fn register(&mut self, language: LanguageDefinition) -> Result<(), HighlightError> {
        self.languages
            .insert(language.id.as_ref().to_string(), language);
        Ok(())
    }

    pub fn register_udl(&mut self, udl: UdlDefinition) -> Result<(), HighlightError> {
        let definition = LanguageDefinition::from_udl(udl)?;
        self.register(definition)
    }

    pub fn get(&self, id: impl AsRef<str>) -> Option<&LanguageDefinition> {
        self.languages.get(id.as_ref())
    }

    pub fn highlight(
        &self,
        id: impl AsRef<str>,
        input: &str,
    ) -> Result<Vec<HighlightToken>, HighlightError> {
        let language = self
            .get(id.as_ref())
            .ok_or_else(|| HighlightError::LanguageNotRegistered(id.as_ref().to_string()))?;
        Ok(language.highlight(input))
    }
}

pub struct SyntaxHighlighter {
    registry: LanguageRegistry,
}

impl SyntaxHighlighter {
    pub fn new(registry: LanguageRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &LanguageRegistry {
        &self.registry
    }

    pub fn highlight(
        &self,
        language_id: impl AsRef<str>,
        input: &str,
    ) -> Result<Vec<HighlightToken>, HighlightError> {
        self.registry.highlight(language_id, input)
    }
}

fn highlight_block_comments(
    block: &BlockComment,
    input: &str,
    tokens: &mut Vec<HighlightToken>,
    occupied: &mut [bool],
) {
    let mut index = 0;
    while index < input.len() {
        match input[index..].find(&block.start) {
            Some(found) => {
                let start = index + found;
                let content_start = start + block.start.len();
                let mut end = input.len();
                if let Some(end_rel) = input[content_start..].find(&block.end) {
                    end = content_start + end_rel + block.end.len();
                }
                mark_range(occupied, start..end);
                tokens.push(HighlightToken {
                    range: start..end,
                    kind: HighlightKind::Comment,
                });
                index = end;
            }
            None => break,
        }
    }
}

fn highlight_line_comments(
    marker: &str,
    input: &str,
    tokens: &mut Vec<HighlightToken>,
    occupied: &mut [bool],
) {
    if marker.is_empty() {
        return;
    }
    let mut cursor = 0;
    for line in input.split_inclusive('\n') {
        if let Some(position) = line.find(marker) {
            let start = cursor + position;
            if !occupied.get(start).copied().unwrap_or(false) {
                let end = cursor + line.len();
                mark_range(occupied, start..end);
                tokens.push(HighlightToken {
                    range: start..end,
                    kind: HighlightKind::Comment,
                });
            }
        }
        cursor += line.len();
    }
}

fn highlight_strings(
    delimiter: &StringDelimiter,
    input: &str,
    tokens: &mut Vec<HighlightToken>,
    occupied: &mut [bool],
) {
    if delimiter.start.is_empty() || delimiter.end.is_empty() {
        return;
    }

    let bytes = input.as_bytes();
    let mut index = 0;
    while index < input.len() {
        match input[index..].find(&delimiter.start) {
            Some(rel_start) => {
                let start = index + rel_start;
                if occupied.get(start).copied().unwrap_or(false) {
                    index = start + delimiter.start.len();
                    continue;
                }

                let mut cursor = start + delimiter.start.len();
                let mut end = input.len();
                while cursor < input.len() {
                    if input[cursor..].starts_with(&delimiter.end) {
                        if let Some(escape) = delimiter.escape {
                            if cursor > start + delimiter.start.len() {
                                let previous = bytes[cursor - 1] as char;
                                if previous == escape {
                                    cursor += 1;
                                    continue;
                                }
                            }
                        }
                        end = cursor + delimiter.end.len();
                        break;
                    }
                    cursor += 1;
                }
                mark_range(occupied, start..end);
                tokens.push(HighlightToken {
                    range: start..end,
                    kind: HighlightKind::String,
                });
                index = end;
            }
            None => break,
        }
    }
}

fn highlight_with_regex(
    regex: &Regex,
    kind: HighlightKind,
    input: &str,
    tokens: &mut Vec<HighlightToken>,
    occupied: &mut [bool],
) {
    for capture in regex.find_iter(input) {
        let range = capture.start()..capture.end();
        if range
            .clone()
            .any(|index| occupied.get(index).copied().unwrap_or(false))
        {
            continue;
        }
        mark_range(occupied, range.clone());
        tokens.push(HighlightToken {
            range,
            kind: kind.clone(),
        });
    }
}

fn mark_range(occupied: &mut [bool], range: Range<usize>) {
    let start = range.start.min(occupied.len());
    let end = range.end.min(occupied.len());
    for index in start..end {
        occupied[index] = true;
    }
}

fn build_keyword_regex(
    keywords: &[String],
    case_sensitive: bool,
) -> Result<Option<Regex>, HighlightError> {
    if keywords.is_empty() {
        return Ok(None);
    }
    let pattern = keywords
        .iter()
        .map(|keyword| regex::escape(keyword))
        .collect::<Vec<_>>()
        .join("|");
    let pattern = format!(r"\b({pattern})\b");
    let mut builder = RegexBuilder::new(&pattern);
    builder.multi_line(true);
    if !case_sensitive {
        builder.case_insensitive(true);
    }
    builder.build().map(Some).map_err(|err| {
        HighlightError::RegexCompilation(format!("keyword regex compile error: {err}"))
    })
}

fn build_operator_regex(operators: &[String]) -> Result<Option<Regex>, HighlightError> {
    if operators.is_empty() {
        return Ok(None);
    }
    let pattern = operators
        .iter()
        .map(|operator| regex::escape(operator))
        .collect::<Vec<_>>()
        .join("|");
    let pattern = format!("({pattern})");
    Regex::new(&pattern)
        .map(Some)
        .map_err(|err| HighlightError::RegexCompilation(format!("operator regex: {err}")))
}

fn build_number_regex(custom: Option<&str>) -> Result<Regex, HighlightError> {
    let pattern = custom.unwrap_or(NUMBER_PATTERN);
    Regex::new(pattern)
        .map_err(|err| HighlightError::RegexCompilation(format!("number regex: {err}")))
}

pub mod builtin {
    use super::*;

    pub fn builtins() -> Vec<LanguageDefinition> {
        vec![rust(), json(), plaintext()]
    }

    fn rust() -> LanguageDefinition {
        let udl = UdlDefinition {
            name: "Rust".into(),
            identifier: Some("rust".into()),
            extensions: vec!["rs".into(), "rlib".into()],
            keywords: vec![
                "fn", "let", "mut", "pub", "impl", "trait", "struct", "enum", "match", "if",
                "else", "loop", "while", "for", "in", "move", "async", "await", "use", "crate",
                "mod", "const", "static", "where", "return", "break", "continue", "Self", "self",
                "ref", "type", "unsafe", "extern", "dyn",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            delimiters: vec![Delimiter {
                start: "\"".into(),
                end: Some("\"".into()),
                escape: Some('\\'),
            }],
            number_pattern: None,
            operators: vec![
                "::", "->", "=>", "==", "!=", ">=", "<=", "+=", "-=", "*=", "/=", "%=", "&&", "||",
                "+", "-", "*", "/", "%", ">", "<", "&", "|", "^",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            case_sensitive: true,
        };
        LanguageDefinition::from_udl(udl).expect("built-in rust UDL should parse")
    }

    fn json() -> LanguageDefinition {
        let udl = UdlDefinition {
            name: "JSON".into(),
            identifier: Some("json".into()),
            extensions: vec!["json".into()],
            keywords: vec!["true".into(), "false".into(), "null".into()],
            line_comment: None,
            block_comment: None,
            delimiters: vec![Delimiter {
                start: "\"".into(),
                end: Some("\"".into()),
                escape: Some('\\'),
            }],
            number_pattern: None,
            operators: vec![
                ":".into(),
                ",".into(),
                "{".into(),
                "}".into(),
                "[".into(),
                "]".into(),
            ],
            case_sensitive: true,
        };
        LanguageDefinition::from_udl(udl).expect("built-in json UDL should parse")
    }

    fn plaintext() -> LanguageDefinition {
        let udl = UdlDefinition {
            name: "Plain Text".into(),
            identifier: Some("plain_text".into()),
            extensions: Vec::new(),
            keywords: Vec::new(),
            line_comment: None,
            block_comment: None,
            delimiters: Vec::new(),
            number_pattern: None,
            operators: Vec::new(),
            case_sensitive: true,
        };
        LanguageDefinition::from_udl(udl).expect("plain text UDL should parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::udl::UdlDefinition;

    #[test]
    fn highlights_rust_keywords_and_comments() {
        let registry = LanguageRegistry::with_defaults();
        let source = r#"
        fn main() {
            // comment
            let value = "text";
            /* block */
            42usize
        }
        "#;
        let tokens = registry.highlight("rust", source).unwrap();
        assert!(tokens
            .iter()
            .any(|token| token.kind == HighlightKind::Keyword));
        assert!(tokens
            .iter()
            .any(|token| token.kind == HighlightKind::Comment));
        assert!(tokens
            .iter()
            .any(|token| token.kind == HighlightKind::String));
        assert!(tokens
            .iter()
            .any(|token| token.kind == HighlightKind::Number));
    }

    #[test]
    fn registers_udl_definition() {
        let mut registry = LanguageRegistry::new();
        let udl = UdlDefinition {
            name: "Custom".into(),
            identifier: Some("custom".into()),
            extensions: vec!["foo".into()],
            keywords: vec!["alpha".into(), "beta".into()],
            line_comment: Some("#".into()),
            block_comment: None,
            delimiters: vec![Delimiter {
                start: "\"".into(),
                end: Some("\"".into()),
                escape: Some('\\'),
            }],
            number_pattern: None,
            operators: vec!["+".into()],
            case_sensitive: false,
        };
        registry.register_udl(udl).unwrap();
        let tokens = registry
            .highlight("custom", "ALPHA + beta #comment")
            .unwrap();
        let keyword_count = tokens
            .iter()
            .filter(|token| token.kind == HighlightKind::Keyword)
            .count();
        assert!(keyword_count >= 2);
        assert!(tokens
            .iter()
            .any(|token| token.kind == HighlightKind::Comment));
    }
}
