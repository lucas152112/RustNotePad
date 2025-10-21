use std::collections::HashMap;
use std::sync::RwLock;

use rustnotepad_autocomplete::{LspBridge, LspError, LspRequestParams, LspSuggestion};

/// Severity level for diagnostics emitted by an LSP server.
/// （LSP 伺服器發出的診斷資訊嚴重層級。）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Diagnostic message tracked for the active language.
/// （針對使用中語言追蹤的診斷訊息。）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, severity: DiagnosticSeverity) -> Self {
        Self {
            message: message.into(),
            severity,
        }
    }
}

/// Lightweight LSP client façade that fulfils the autocomplete bridge interface.
/// （滿足自動完成橋接介面的輕量級 LSP 用戶端外觀層。）
pub struct LspClient {
    state: RwLock<InnerState>,
}

impl LspClient {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(InnerState::with_defaults()),
        }
    }

    pub fn set_online(&self, online: bool) {
        let mut guard = self.state.write().expect("LSP state poisoned");
        guard.online = online;
    }

    pub fn is_online(&self) -> bool {
        let guard = self.state.read().expect("LSP state poisoned");
        guard.online
    }

    pub fn set_enabled(&self, language: impl Into<String>, enabled: bool) {
        let mut guard = self.state.write().expect("LSP state poisoned");
        let session = guard.ensure_language(language.into());
        session.enabled = enabled;
    }

    pub fn is_enabled(&self, language: &str) -> bool {
        let guard = self.state.read().expect("LSP state poisoned");
        guard
            .languages
            .get(language)
            .map(|session| session.enabled)
            .unwrap_or(false)
    }

    pub fn update_suggestions(&self, language: impl Into<String>, suggestions: Vec<LspSuggestion>) {
        let mut guard = self.state.write().expect("LSP state poisoned");
        let session = guard.ensure_language(language.into());
        session.suggestions = suggestions;
    }

    pub fn update_diagnostics(&self, language: impl Into<String>, diagnostics: Vec<Diagnostic>) {
        let mut guard = self.state.write().expect("LSP state poisoned");
        let session = guard.ensure_language(language.into());
        session.diagnostics = diagnostics;
    }

    pub fn diagnostics(&self, language: &str) -> Vec<Diagnostic> {
        let guard = self.state.read().expect("LSP state poisoned");
        guard
            .languages
            .get(language)
            .map(|session| session.diagnostics.clone())
            .unwrap_or_default()
    }
}

impl Default for LspClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LspBridge for LspClient {
    fn is_enabled(&self, language: Option<&str>) -> bool {
        let guard = self.state.read().expect("LSP state poisoned");
        if !guard.online {
            return false;
        }
        language
            .and_then(|lang| guard.languages.get(lang))
            .map(|session| session.enabled)
            .unwrap_or(false)
    }

    fn complete(&self, params: LspRequestParams<'_>) -> Result<Vec<LspSuggestion>, LspError> {
        let guard = self.state.read().expect("LSP state poisoned");
        if !guard.online {
            return Err(LspError::Backend("Language server offline".into()));
        }
        let language = match params.language {
            Some(language) => language,
            None => return Err(LspError::Disabled),
        };

        let session = match guard.languages.get(language) {
            Some(session) if session.enabled => session,
            _ => return Err(LspError::Disabled),
        };

        let prefix_lower = params.prefix.to_ascii_lowercase();
        let mut suggestions = Vec::new();
        for suggestion in &session.suggestions {
            if params.prefix.is_empty()
                || suggestion
                    .label
                    .to_ascii_lowercase()
                    .starts_with(&prefix_lower)
            {
                suggestions.push(suggestion.clone());
            }
            if suggestions.len() >= params.max_items {
                break;
            }
        }
        Ok(suggestions)
    }
}

struct InnerState {
    online: bool,
    languages: HashMap<String, LanguageSession>,
}

impl InnerState {
    fn with_defaults() -> Self {
        let mut state = Self {
            online: true,
            languages: HashMap::new(),
        };
        state.languages.insert(
            "rust".into(),
            LanguageSession {
                enabled: true,
                suggestions: vec![
                    LspSuggestion {
                        label: "Result".into(),
                        insert_text: None,
                        detail: Some("From std::result".into()),
                        kind: None,
                        relevance: Some(0.97),
                    },
                    LspSuggestion {
                        label: "match".into(),
                        insert_text: Some(
                            "match ${1:expr} {\n    ${2:pattern} => ${3:result},\n}\n".into(),
                        ),
                        detail: Some("Control flow keyword".into()),
                        kind: None,
                        relevance: Some(0.94),
                    },
                ],
                diagnostics: vec![Diagnostic::new(
                    "No diagnostics from rust-analyzer",
                    DiagnosticSeverity::Information,
                )],
            },
        );

        state.languages.insert(
            "json".into(),
            LanguageSession {
                enabled: true,
                suggestions: vec![LspSuggestion {
                    label: "\"key\"".into(),
                    insert_text: Some("\"${1:key}\": ${2:value}".into()),
                    detail: Some("Insert key/value pair".into()),
                    kind: None,
                    relevance: Some(0.9),
                }],
                diagnostics: vec![Diagnostic::new(
                    "Schema validation not configured",
                    DiagnosticSeverity::Hint,
                )],
            },
        );
        state
    }

    fn ensure_language(&mut self, language: String) -> &mut LanguageSession {
        self.languages
            .entry(language)
            .or_insert_with(|| LanguageSession {
                enabled: true,
                suggestions: Vec::new(),
                diagnostics: Vec::new(),
            })
    }
}

struct LanguageSession {
    enabled: bool,
    suggestions: Vec<LspSuggestion>,
    diagnostics: Vec<Diagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_online_flag() {
        let client = LspClient::new();
        client.set_online(false);
        assert!(!client.is_online());
        let result = client.complete(LspRequestParams {
            document: Some("main.rs"),
            language: Some("rust"),
            prefix: "",
            trigger: rustnotepad_autocomplete::CompletionTrigger::Automatic,
            context: &rustnotepad_autocomplete::CompletionContext::default(),
            max_items: 8,
        });
        assert!(matches!(result, Err(LspError::Backend(_))));
    }

    #[test]
    fn filters_suggestions_by_prefix() {
        let client = LspClient::new();
        let response = client
            .complete(LspRequestParams {
                document: Some("main.rs"),
                language: Some("rust"),
                prefix: "ma",
                trigger: rustnotepad_autocomplete::CompletionTrigger::Automatic,
                context: &rustnotepad_autocomplete::CompletionContext::default(),
                max_items: 8,
            })
            .unwrap();
        assert_eq!(response.len(), 1);
        assert_eq!(response[0].label, "match");
    }

    #[test]
    fn exposes_diagnostics() {
        let client = LspClient::new();
        let diagnostics = client.diagnostics("rust");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "No diagnostics from rust-analyzer");
    }
}
