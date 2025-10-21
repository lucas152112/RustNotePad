/// Defines a reusable code snippet entry.
/// （定義可重複使用的程式碼片段項目。）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetDefinition {
    pub trigger: String,
    pub body: String,
    pub description: Option<String>,
    pub language: Option<String>,
}

impl SnippetDefinition {
    pub fn new(trigger: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            body: body.into(),
            description: None,
            language: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

/// Collection of snippet definitions, usually loaded from user configuration.
/// （通常由使用者設定載入的片段定義集合。）
#[derive(Default)]
pub struct SnippetStore {
    entries: Vec<SnippetDefinition>,
}

impl SnippetStore {
    pub fn new(entries: Vec<SnippetDefinition>) -> Self {
        Self { entries }
    }

    /// Returns built-in snippet samples used for previews and defaults.
    /// （回傳用於預覽與預設的內建片段範例。）
    pub fn builtin() -> Self {
        Self::new(vec![
            SnippetDefinition::new("test", "#[test]\nfn ${1:name}() {\n    ${0}// Arrange\n}")
                .with_description("Rust test function")
                .with_language("rust"),
            SnippetDefinition::new("todo", "// TODO: ${0:detail}")
                .with_description("Insert TODO comment"),
            SnippetDefinition::new("pair", "\"${1:key}\": ${2:value}")
                .with_description("JSON key/value pair")
                .with_language("json"),
        ])
    }

    pub fn entries(&self) -> &[SnippetDefinition] {
        &self.entries
    }
}
