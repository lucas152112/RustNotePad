use regex::Regex;
use std::collections::HashMap;

/// Byte range corresponding to a symbol inside the document.
/// （對應文件符號的位元組區間資訊。）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// Classifies the type of function-list entry.
/// （函式清單條目的類型分類。）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FunctionKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Region,
    Custom(String),
}

/// Represents a single entry shown in the function list panel.
/// （在函式清單面板中顯示的單一條目。）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionEntry {
    pub name: String,
    pub kind: FunctionKind,
    pub range: TextRange,
}

impl FunctionEntry {
    pub fn new(name: impl Into<String>, kind: FunctionKind, range: TextRange) -> Self {
        Self {
            name: name.into(),
            kind,
            range,
        }
    }
}

/// Trait implemented by all function-list parsers.
/// （所有函式清單解析器需實作的 trait。）
pub trait FunctionParser: Send + Sync {
    fn parse(&self, source: &str) -> Vec<FunctionEntry>;
}

/// Parser that derives entries using a sequence of regex rules.
/// （使用一組正規表示式規則解析條目的解析器。）
pub struct RegexParser {
    rules: Vec<RegexRule>,
}

impl RegexParser {
    pub fn new(rules: Vec<RegexRule>) -> Self {
        Self { rules }
    }

    pub fn push_rule(&mut self, rule: RegexRule) {
        self.rules.push(rule);
    }
}

impl FunctionParser for RegexParser {
    fn parse(&self, source: &str) -> Vec<FunctionEntry> {
        let mut entries = Vec::new();
        for rule in &self.rules {
            for capture in rule.regex.captures_iter(source) {
                let name = capture
                    .name("name")
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_else(|| capture[0].trim().to_string());
                let span = capture.get(0).expect("regex capture must exist");
                entries.push(FunctionEntry::new(
                    name,
                    rule.kind.clone(),
                    TextRange::new(span.start(), span.end()),
                ));
            }
        }
        entries.sort_by_key(|entry| entry.range.start);
        entries.dedup_by(|a, b| a.range.start == b.range.start && a.name == b.name);
        entries
    }
}

#[derive(Clone)]
pub struct RegexRule {
    pub regex: Regex,
    pub kind: FunctionKind,
}

impl RegexRule {
    pub fn new(pattern: &str, kind: FunctionKind) -> Result<Self, regex::Error> {
        Ok(Self {
            regex: Regex::new(pattern)?,
            kind,
        })
    }
}

/// Registry that maps language identifiers to parser implementations.
/// （將語言識別碼映射到解析器實作的註冊器。）
#[derive(Default)]
pub struct ParserRegistry {
    parsers: HashMap<String, Box<dyn FunctionParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_parser(
        &mut self,
        language_id: impl Into<String>,
        parser: Box<dyn FunctionParser>,
    ) {
        self.parsers.insert(language_id.into(), parser);
    }

    pub fn parse(&self, language_id: &str, source: &str) -> Option<Vec<FunctionEntry>> {
        self.parsers
            .get(language_id)
            .map(|parser| parser.parse(source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_parser_extracts_simple_rust_functions() {
        let rule = RegexRule::new(
            r"(?m)^\s*(?:pub\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
            FunctionKind::Function,
        )
        .unwrap();
        let parser = RegexParser::new(vec![rule]);
        let source = r#"
            pub fn alpha() {}
            fn beta() {}
        "#;
        let entries = parser.parse(source);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "alpha");
        assert_eq!(entries[1].name, "beta");
    }

    #[test]
    fn parser_registry_dispatches_by_language() {
        let rule =
            RegexRule::new(r"(?m)^class\s+(?P<name>[A-Za-z_]\w*)", FunctionKind::Class).unwrap();
        let parser = RegexParser::new(vec![rule]);

        let mut registry = ParserRegistry::new();
        registry.register_parser("python", Box::new(parser));

        let source = "class Example:\n    pass";
        let entries = registry.parse("python", source).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Example");
    }
}
