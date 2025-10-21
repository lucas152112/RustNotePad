use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, RwLock};

/// Identifies why a completion lookup was triggered.
/// （說明自動完成查詢被觸發的原因。）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionTrigger {
    Manual,
    Automatic,
    TriggerCharacter(char),
}

/// Contextual flags derived from the editor state.
/// （由編輯器狀態推導的情境旗標。）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionContext {
    pub in_comment: bool,
    pub in_string: bool,
}

/// Request payload provided by the caller.
/// （呼叫端提供的請求載荷。）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionRequest {
    pub document: Option<String>,
    pub prefix: String,
    pub max_items: usize,
    pub case_sensitive: bool,
    pub trigger: CompletionTrigger,
    pub context: CompletionContext,
    pub language: Option<String>,
}

impl CompletionRequest {
    pub fn new(document: Option<String>, prefix: impl Into<String>) -> Self {
        Self {
            document,
            prefix: prefix.into(),
            max_items: 100,
            case_sensitive: false,
            trigger: CompletionTrigger::Automatic,
            context: CompletionContext::default(),
            language: None,
        }
    }

    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items = max;
        self
    }

    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn with_trigger(mut self, trigger: CompletionTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    pub fn with_context(mut self, context: CompletionContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_language(mut self, language: Option<String>) -> Self {
        self.language = language;
        self
    }
}

/// Type of item returned by a completion provider.
/// （補全提供者返回項目的類型描述。）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    Keyword,
    Snippet,
    Symbol,
    Text,
    FilePath,
    Module,
    Custom(String),
}

/// Individual completion entry.
/// （單一補全項目。）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub insert_text: Option<String>,
    pub detail: Option<String>,
    pub kind: CompletionKind,
}

impl CompletionItem {
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        Self {
            label: label.into(),
            insert_text: None,
            detail: None,
            kind,
        }
    }

    pub fn with_insert_text(mut self, insert_text: impl Into<String>) -> Self {
        self.insert_text = Some(insert_text.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Result of a completion request after merging every provider.
/// （合併所有提供者後的補全結果。）
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionSet {
    pub items: Vec<CompletionItem>,
    pub is_incomplete: bool,
}

impl CompletionSet {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            is_incomplete: false,
        }
    }
}

/// Result from a single provider before aggregation.
/// （單一提供者在合併前的輸出。）
#[derive(Debug, Clone)]
pub struct ProviderItem {
    pub item: CompletionItem,
    /// Relevance score in range `[0.0, 1.0]`.
    /// （相關性評分範圍為 `[0.0, 1.0]`。）
    pub relevance: f32,
}

#[derive(Debug, Clone)]
pub struct ProviderResult {
    pub items: Vec<ProviderItem>,
    pub is_incomplete: bool,
}

impl ProviderResult {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            is_incomplete: false,
        }
    }
}

pub trait CompletionProvider: Send + Sync {
    fn complete(&self, request: &CompletionRequest) -> ProviderResult;
}

struct ProviderRegistration {
    _name: &'static str,
    priority: u8,
    provider: Arc<dyn CompletionProvider>,
}

/// Aggregates suggestions from multiple providers and produces a ranked list.
/// （彙整多個提供者的建議並產生排序清單。）
pub struct CompletionEngine {
    providers: Vec<ProviderRegistration>,
}

impl CompletionEngine {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register_provider<P>(&mut self, name: &'static str, priority: u8, provider: P)
    where
        P: CompletionProvider + 'static,
    {
        self.providers.push(ProviderRegistration {
            _name: name,
            priority,
            provider: Arc::new(provider),
        });
        self.providers
            .sort_by_key(|registration| registration.priority);
    }

    pub fn request(&self, request: CompletionRequest) -> CompletionSet {
        if self.providers.is_empty() {
            return CompletionSet::empty();
        }

        let mut aggregated: HashMap<(String, CompletionKindKey), AggregatedItem> = HashMap::new();
        let mut is_incomplete = false;

        for registration in &self.providers {
            let result = registration.provider.complete(&request);
            if result.is_incomplete {
                is_incomplete = true;
            }

            for provider_item in result.items {
                let key = (
                    provider_item.item.label.clone(),
                    CompletionKindKey::from(&provider_item.item.kind),
                );
                let entry = aggregated.entry(key).or_insert_with(|| AggregatedItem {
                    item: provider_item.item.clone(),
                    score: f32::MIN,
                });
                let score = compute_score(
                    registration.priority,
                    &request,
                    &provider_item.item,
                    provider_item.relevance,
                );
                if score > entry.score {
                    entry.score = score;
                    entry.item = provider_item.item;
                }
            }
        }

        let mut items: Vec<_> = aggregated.into_iter().map(|(_, entry)| entry).collect();
        items.sort_by(|a, b| match b.score.partial_cmp(&a.score) {
            Some(Ordering::Equal) | None => a.item.label.cmp(&b.item.label),
            Some(order) => order,
        });

        let mut pruned = Vec::with_capacity(items.len());
        for entry in items {
            if pruned.len() >= request.max_items {
                break;
            }
            pruned.push(entry.item);
        }

        CompletionSet {
            items: pruned,
            is_incomplete,
        }
    }
}

fn compute_score(
    priority: u8,
    request: &CompletionRequest,
    item: &CompletionItem,
    provider_relevance: f32,
) -> f32 {
    let priority_score = 1.0 - (priority.min(9) as f32 / 9.0);
    let match_score = match_quality(&request.prefix, &item.label, request.case_sensitive);
    let relevance_score = provider_relevance.clamp(0.0, 1.0);

    // Weighted blend gives most influence to match quality while keeping provider input.
    // 權重混合讓匹配品質具有最高影響力，同時保留提供者分數的作用。
    (priority_score * 0.35) + (match_score * 0.5) + (relevance_score * 0.15)
}

fn match_quality(prefix: &str, label: &str, case_sensitive: bool) -> f32 {
    if prefix.is_empty() {
        return 0.6;
    }

    if case_sensitive {
        if label.starts_with(prefix) {
            if label == prefix {
                return 1.0;
            }
            return 0.9;
        }
    } else {
        let lower_label = label.to_lowercase();
        let lower_prefix = prefix.to_lowercase();
        if lower_label.starts_with(&lower_prefix) {
            if lower_label == lower_prefix {
                return 1.0;
            }
            return 0.9;
        }
    }

    if fuzzy_match(prefix, label, case_sensitive) {
        0.4
    } else {
        0.0
    }
}

fn fuzzy_match(prefix: &str, label: &str, case_sensitive: bool) -> bool {
    if prefix.is_empty() {
        return true;
    }
    let mut label_chars = label.chars();
    for expected in prefix.chars() {
        if let Some(found) = label_chars.find(|candidate| {
            if case_sensitive {
                *candidate == expected
            } else {
                candidate.to_ascii_lowercase() == expected.to_ascii_lowercase()
            }
        }) {
            // Continue scanning after the found char
            // 在找到的字元之後繼續掃描。
            if found == '\0' {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

#[derive(Debug)]
struct AggregatedItem {
    item: CompletionItem,
    score: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CompletionKindKey {
    kind: CompletionKindDiscriminant,
    custom: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CompletionKindDiscriminant {
    Keyword,
    Snippet,
    Symbol,
    Text,
    FilePath,
    Module,
    Custom,
}

impl CompletionKindKey {
    fn from(kind: &CompletionKind) -> Self {
        match kind {
            CompletionKind::Keyword => Self {
                kind: CompletionKindDiscriminant::Keyword,
                custom: None,
            },
            CompletionKind::Snippet => Self {
                kind: CompletionKindDiscriminant::Snippet,
                custom: None,
            },
            CompletionKind::Symbol => Self {
                kind: CompletionKindDiscriminant::Symbol,
                custom: None,
            },
            CompletionKind::Text => Self {
                kind: CompletionKindDiscriminant::Text,
                custom: None,
            },
            CompletionKind::FilePath => Self {
                kind: CompletionKindDiscriminant::FilePath,
                custom: None,
            },
            CompletionKind::Module => Self {
                kind: CompletionKindDiscriminant::Module,
                custom: None,
            },
            CompletionKind::Custom(name) => Self {
                kind: CompletionKindDiscriminant::Custom,
                custom: Some(name.clone()),
            },
        }
    }
}

/// Shared document index used by the built-in document-word provider.
/// （內建文件字詞提供者共用的索引儲存體。）
#[derive(Default)]
pub struct DocumentIndex {
    inner: RwLock<HashMap<String, DocumentWords>>,
    counter: AtomicU64,
}

impl DocumentIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_document(&self, document: impl Into<String>, contents: &str) {
        let document = document.into();
        let tokens = tokenize(contents);
        let mut guard = self.inner.write().expect("poisoned document index");
        let entry = guard.entry(document).or_insert_with(DocumentWords::default);
        entry.update(tokens, self.counter.fetch_add(1, AtomicOrdering::Relaxed));
    }

    pub fn remove_document(&self, document: &str) {
        let mut guard = self.inner.write().expect("poisoned document index");
        guard.remove(document);
    }

    pub fn collect(
        &self,
        prefix: &str,
        case_sensitive: bool,
        limit: usize,
    ) -> Vec<DocumentWordCandidate> {
        let guard = self.inner.read().expect("poisoned document index");
        if guard.is_empty() {
            return Vec::new();
        }

        let normalised_prefix = if case_sensitive {
            String::new()
        } else {
            prefix.to_lowercase()
        };

        let mut aggregate: HashMap<String, DocumentWordCandidate> = HashMap::new();
        for document in guard.values() {
            for (normalised, stats) in &document.words {
                if !matches_document_prefix(
                    prefix,
                    &normalised_prefix,
                    case_sensitive,
                    normalised,
                    &stats.best_variant,
                ) {
                    continue;
                }

                let entry =
                    aggregate
                        .entry(normalised.clone())
                        .or_insert_with(|| DocumentWordCandidate {
                            label: stats.best_variant.clone(),
                            normalised: normalised.clone(),
                            occurrences: 0,
                            last_seen: 0,
                        });

                entry.occurrences += stats.occurrences;
                if stats.last_seen > entry.last_seen {
                    entry.last_seen = stats.last_seen;
                    entry.label = stats.best_variant.clone();
                }
            }
        }

        let mut candidates: Vec<_> = aggregate.into_values().collect();
        candidates.sort_by(|a, b| {
            b.occurrences
                .cmp(&a.occurrences)
                .then_with(|| b.last_seen.cmp(&a.last_seen))
                .then_with(|| a.label.cmp(&b.label))
        });
        candidates.truncate(limit);
        candidates
    }
}

#[derive(Default)]
struct DocumentWords {
    words: HashMap<String, WordStats>,
}

impl DocumentWords {
    fn update(&mut self, tokens: HashMap<String, WordOccurrence>, tick: u64) {
        // Remove words no longer present.
        // 移除已不存在的字詞。
        self.words.retain(|token, _| tokens.contains_key(token));

        for (normalised, occurrence) in tokens {
            let stats = self
                .words
                .entry(normalised.clone())
                .or_insert_with(WordStats::default);
            stats.occurrences = occurrence.total;
            stats.best_variant = occurrence.best_variant;
            stats.last_seen = tick;
        }
    }
}

#[derive(Default)]
struct WordStats {
    occurrences: u32,
    best_variant: String,
    last_seen: u64,
}

#[derive(Debug, Clone)]
pub struct DocumentWordCandidate {
    pub label: String,
    pub normalised: String,
    pub occurrences: u32,
    pub last_seen: u64,
}

fn matches_document_prefix(
    raw_prefix: &str,
    normalised_prefix: &str,
    case_sensitive: bool,
    normalised_token: &str,
    variant: &str,
) -> bool {
    if raw_prefix.is_empty() {
        return true;
    }
    if case_sensitive {
        variant.starts_with(raw_prefix)
    } else {
        normalised_token.starts_with(normalised_prefix)
    }
}

fn tokenize(input: &str) -> HashMap<String, WordOccurrence> {
    let mut map = HashMap::<String, WordOccurrence>::new();
    let mut current = String::new();

    for ch in input.chars() {
        if is_word_char(ch) {
            current.push(ch);
        } else if !current.is_empty() {
            record_word(&mut map, &current);
            current.clear();
        }
    }

    if !current.is_empty() {
        record_word(&mut map, &current);
    }

    map
}

fn record_word(map: &mut HashMap<String, WordOccurrence>, token: &str) {
    if token.chars().all(|c| c.is_numeric()) {
        return;
    }
    let normalised = token.to_lowercase();
    let entry = map
        .entry(normalised)
        .or_insert_with(|| WordOccurrence::new(token.to_string()));
    entry.total += 1;
    entry.maybe_update_variant(token);
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '$'
}

#[derive(Debug, Clone)]
struct WordOccurrence {
    total: u32,
    best_variant: String,
    best_variant_count: u32,
}

impl WordOccurrence {
    fn new(initial: String) -> Self {
        Self {
            total: 0,
            best_variant: initial.clone(),
            best_variant_count: 0,
        }
    }

    fn maybe_update_variant(&mut self, candidate: &str) {
        if candidate.len() >= self.best_variant.len() {
            self.best_variant = candidate.to_string();
            self.best_variant_count += 1;
        }
    }
}

/// Completion provider that serves words observed in open documents.
/// （提供開啟中文件字詞建議的補全提供者。）
pub struct DocumentWordsProvider {
    index: Arc<DocumentIndex>,
    min_prefix: usize,
    max_items: usize,
}

impl DocumentWordsProvider {
    pub fn new(index: Arc<DocumentIndex>) -> Self {
        Self {
            index,
            min_prefix: 1,
            max_items: 40,
        }
    }

    pub fn with_prefix_minimum(mut self, min_prefix: usize) -> Self {
        self.min_prefix = min_prefix;
        self
    }

    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.max_items = max_items;
        self
    }
}

impl CompletionProvider for DocumentWordsProvider {
    fn complete(&self, request: &CompletionRequest) -> ProviderResult {
        if request.prefix.len() < self.min_prefix {
            return ProviderResult::empty();
        }

        let candidates =
            self.index
                .collect(&request.prefix, request.case_sensitive, self.max_items);

        let items = candidates
            .into_iter()
            .map(|candidate| {
                let relevance = 1.0 - (1.0 / (1.0 + candidate.occurrences as f32));
                ProviderItem {
                    item: CompletionItem::new(candidate.label, CompletionKind::Text)
                        .with_detail("Document word"),
                    relevance,
                }
            })
            .collect();

        ProviderResult {
            items,
            is_incomplete: false,
        }
    }
}

/// Source of static keywords extracted from language definitions or dictionaries.
/// （提供語言定義或字典中靜態關鍵字的補全來源。）
#[derive(Default)]
pub struct LanguageDictionaryProvider {
    dictionaries: HashMap<String, DictionaryEntries>,
    fallback: Option<DictionaryEntries>,
    max_items: usize,
}

impl LanguageDictionaryProvider {
    pub fn new() -> Self {
        Self {
            dictionaries: HashMap::new(),
            fallback: None,
            max_items: 64,
        }
    }

    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.max_items = max_items;
        self
    }

    pub fn register_language(
        &mut self,
        language_id: impl Into<String>,
        keywords: impl IntoIterator<Item = String>,
        case_sensitive: bool,
    ) {
        let mut entries = DictionaryEntries::new(case_sensitive);
        entries.extend(keywords);
        self.dictionaries.insert(language_id.into(), entries);
    }

    pub fn register_fallback(&mut self, keywords: impl IntoIterator<Item = String>) {
        let mut entries = DictionaryEntries::new(false);
        entries.extend(keywords);
        self.fallback = Some(entries);
    }
}

impl CompletionProvider for LanguageDictionaryProvider {
    fn complete(&self, request: &CompletionRequest) -> ProviderResult {
        let dictionary = request
            .language
            .as_ref()
            .and_then(|language| self.dictionaries.get(language))
            .or_else(|| self.fallback.as_ref());

        let Some(dictionary) = dictionary else {
            return ProviderResult::empty();
        };

        let case_sensitive = dictionary.case_sensitive || request.case_sensitive;
        let matches = dictionary.matching(&request.prefix, case_sensitive);
        let mut items = Vec::new();
        for keyword in matches {
            let match_score = match_quality(&request.prefix, keyword, case_sensitive);
            let relevance = 0.75 + (match_score * 0.25);
            items.push(ProviderItem {
                item: CompletionItem::new(keyword.clone(), CompletionKind::Keyword)
                    .with_detail("Language keyword"),
                relevance,
            });
            if items.len() >= self.max_items || items.len() >= request.max_items {
                break;
            }
        }

        ProviderResult {
            items,
            is_incomplete: false,
        }
    }
}

#[derive(Clone)]
struct DictionaryEntries {
    keywords: Vec<String>,
    case_sensitive: bool,
}

impl DictionaryEntries {
    fn new(case_sensitive: bool) -> Self {
        Self {
            keywords: Vec::new(),
            case_sensitive,
        }
    }

    fn extend(&mut self, keywords: impl IntoIterator<Item = String>) {
        for keyword in keywords {
            if keyword.is_empty() {
                continue;
            }
            if !self.keywords.contains(&keyword) {
                self.keywords.push(keyword);
            }
        }
        self.keywords
            .sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));
    }

    fn matching<'a>(&'a self, prefix: &str, case_sensitive: bool) -> Vec<&'a String> {
        let lower_prefix = if case_sensitive {
            None
        } else {
            Some(prefix.to_lowercase())
        };
        self.keywords
            .iter()
            .filter(move |keyword| {
                if prefix.is_empty() {
                    return true;
                }
                if case_sensitive {
                    keyword.starts_with(prefix)
                } else {
                    keyword
                        .to_lowercase()
                        .starts_with(lower_prefix.as_ref().expect("prefix lowered"))
                }
            })
            .collect()
    }
}

/// Describes a reusable user-defined snippet completion.
/// （描述可重複使用的使用者自訂片段補全。）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub trigger: String,
    pub body: String,
    pub description: Option<String>,
    pub language: Option<String>,
}

impl Snippet {
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

/// Supplies snippet completions filtered by language and prefix.
/// （依語言與前綴條件提供片段補全。）
pub struct SnippetProvider {
    snippets: Vec<Snippet>,
    max_items: usize,
}

impl SnippetProvider {
    pub fn new(snippets: Vec<Snippet>) -> Self {
        Self {
            snippets,
            max_items: 32,
        }
    }

    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.max_items = max_items;
        self
    }
}

impl CompletionProvider for SnippetProvider {
    fn complete(&self, request: &CompletionRequest) -> ProviderResult {
        if request.prefix.is_empty() {
            return ProviderResult::empty();
        }

        let mut items = Vec::new();
        for snippet in &self.snippets {
            if let Some(language) = &snippet.language {
                if Some(language) != request.language.as_ref() {
                    continue;
                }
            }

            let matches = if request.case_sensitive {
                snippet.trigger.starts_with(&request.prefix)
            } else {
                snippet
                    .trigger
                    .to_lowercase()
                    .starts_with(&request.prefix.to_lowercase())
            };

            if !matches {
                continue;
            }

            let detail = snippet.description.as_deref().unwrap_or("User snippet");
            items.push(ProviderItem {
                item: CompletionItem::new(snippet.trigger.clone(), CompletionKind::Snippet)
                    .with_insert_text(snippet.body.clone())
                    .with_detail(detail),
                relevance: 0.92,
            });

            if items.len() >= self.max_items || items.len() >= request.max_items {
                break;
            }
        }

        ProviderResult {
            items,
            is_incomplete: false,
        }
    }
}

/// Error surfaced when the LSP bridge cannot return completions.
/// （當 LSP 橋接無法回傳補全時浮現的錯誤。）
#[derive(Debug)]
pub enum LspError {
    Disabled,
    Backend(String),
}

/// Normalised representation of LSP-provided completion data.
/// （LSP 提供之補全資料的正規化表示。）
#[derive(Debug, Clone)]
pub struct LspSuggestion {
    pub label: String,
    pub insert_text: Option<String>,
    pub detail: Option<String>,
    pub kind: Option<CompletionKind>,
    pub relevance: Option<f32>,
}

impl LspSuggestion {
    fn into_completion(self) -> CompletionItem {
        let mut item = CompletionItem::new(self.label, self.kind.unwrap_or(CompletionKind::Symbol));
        if let Some(insert_text) = self.insert_text {
            item = item.with_insert_text(insert_text);
        }
        if let Some(detail) = self.detail {
            item = item.with_detail(detail);
        }
        item
    }
}

/// Parameters passed to the LSP bridge for completion queries.
/// （傳遞給 LSP 橋接以進行補全查詢的參數。）
pub struct LspRequestParams<'a> {
    pub document: Option<&'a str>,
    pub language: Option<&'a str>,
    pub prefix: &'a str,
    pub trigger: CompletionTrigger,
    pub context: &'a CompletionContext,
    pub max_items: usize,
}

/// Trait implemented by backends capable of serving LSP completions.
/// （能夠提供 LSP 補全的後端需實作的 trait。）
pub trait LspBridge: Send + Sync {
    fn is_enabled(&self, language: Option<&str>) -> bool;
    fn complete(&self, params: LspRequestParams<'_>) -> Result<Vec<LspSuggestion>, LspError>;
}

/// Completion provider delegating to an LSP bridge.
/// （委派給 LSP 橋接的補全提供者。）
pub struct LspProvider {
    bridge: Arc<dyn LspBridge>,
    max_items: usize,
}

impl LspProvider {
    pub fn new(bridge: Arc<dyn LspBridge>) -> Self {
        Self {
            bridge,
            max_items: 32,
        }
    }

    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.max_items = max_items;
        self
    }
}

impl CompletionProvider for LspProvider {
    fn complete(&self, request: &CompletionRequest) -> ProviderResult {
        if !self.bridge.is_enabled(request.language.as_deref()) {
            return ProviderResult::empty();
        }

        let params = LspRequestParams {
            document: request.document.as_deref(),
            language: request.language.as_deref(),
            prefix: &request.prefix,
            trigger: request.trigger,
            context: &request.context,
            max_items: self.max_items.min(request.max_items),
        };

        match self.bridge.complete(params) {
            Ok(suggestions) => {
                let mut items = Vec::new();
                for suggestion in suggestions.into_iter().take(self.max_items) {
                    let relevance = suggestion.relevance.unwrap_or(0.9);
                    items.push(ProviderItem {
                        item: suggestion.into_completion(),
                        relevance,
                    });
                }
                ProviderResult {
                    items,
                    is_incomplete: false,
                }
            }
            Err(LspError::Disabled) => ProviderResult::empty(),
            Err(LspError::Backend(_)) => ProviderResult {
                items: Vec::new(),
                is_incomplete: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticProvider {
        item: CompletionItem,
        relevance: f32,
    }

    impl CompletionProvider for StaticProvider {
        fn complete(&self, _request: &CompletionRequest) -> ProviderResult {
            ProviderResult {
                items: vec![ProviderItem {
                    item: self.item.clone(),
                    relevance: self.relevance,
                }],
                is_incomplete: false,
            }
        }
    }

    #[test]
    fn tokenize_splits_identifiers() {
        let tokens = tokenize("fn main() { let sample_value = dataPoint + fooBar; }");
        assert!(tokens.contains_key("sample_value"));
        assert!(tokens.contains_key("datapoint"));
        assert!(tokens.contains_key("foobar"));
    }

    #[test]
    fn document_index_ranks_by_frequency() {
        let index = DocumentIndex::new();
        index.update_document("a.txt", "alpha beta beta beta gamma");
        index.update_document("b.txt", "alpha alpha delta");

        let results = index.collect("", false, 10);
        assert_eq!(results[0].label.to_lowercase(), "alpha");
        assert_eq!(results[1].label.to_lowercase(), "beta");
    }

    #[test]
    fn engine_merges_providers_by_priority() {
        let mut engine = CompletionEngine::new();
        engine.register_provider(
            "snippets",
            0,
            StaticProvider {
                item: CompletionItem::new("println!", CompletionKind::Snippet),
                relevance: 0.9,
            },
        );
        engine.register_provider(
            "dictionary",
            5,
            StaticProvider {
                item: CompletionItem::new("print", CompletionKind::Keyword),
                relevance: 0.1,
            },
        );

        let request = CompletionRequest::new(None, "pr").with_max_items(5);
        let result = engine.request(request);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].label, "println!");
    }

    #[test]
    fn document_provider_filters_by_prefix() {
        let index = Arc::new(DocumentIndex::new());
        index.update_document("buffer", "foo bar fizz buzz foo");

        let provider = DocumentWordsProvider::new(index);
        let request =
            CompletionRequest::new(Some("buffer".into()), "fi").with_case_sensitive(false);
        let result = provider.complete(&request);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].item.label.to_lowercase(), "fizz");
    }

    #[test]
    fn dictionary_provider_matches_language_keywords() {
        let mut provider = LanguageDictionaryProvider::new();
        provider.register_language("rust", ["fn".into(), "let".into(), "loop".into()], true);

        let request = CompletionRequest::new(None, "l")
            .with_language(Some("rust".into()))
            .with_case_sensitive(true)
            .with_max_items(5);
        let result = provider.complete(&request);
        assert!(!result.items.is_empty());
        assert_eq!(result.items[0].item.label, "let");
    }

    #[test]
    fn snippet_provider_filters_by_language() {
        let snippets = vec![
            Snippet::new("log", "console.log($0);")
                .with_description("Console log")
                .with_language("javascript"),
            Snippet::new("test", "#[test]\nfn ${1:name}() {}").with_language("rust"),
        ];
        let provider = SnippetProvider::new(snippets);

        let request = CompletionRequest::new(None, "te")
            .with_language(Some("rust".into()))
            .with_case_sensitive(false);
        let result = provider.complete(&request);
        assert_eq!(result.items.len(), 1);
        assert_eq!(
            result.items[0].item.insert_text.as_deref().unwrap(),
            "#[test]\nfn ${1:name}() {}"
        );
    }

    struct StubLspBridge {
        enabled: bool,
    }

    impl LspBridge for StubLspBridge {
        fn is_enabled(&self, language: Option<&str>) -> bool {
            self.enabled && language == Some("rust")
        }

        fn complete(&self, params: LspRequestParams<'_>) -> Result<Vec<LspSuggestion>, LspError> {
            if !self.enabled {
                return Err(LspError::Disabled);
            }
            assert_eq!(params.language, Some("rust"));
            Ok(vec![LspSuggestion {
                label: "from_lsp".into(),
                insert_text: Some("from_lsp()".into()),
                detail: Some("LSP suggestion".into()),
                kind: Some(CompletionKind::Symbol),
                relevance: Some(0.95),
            }])
        }
    }

    #[test]
    fn lsp_provider_uses_bridge() {
        let bridge = Arc::new(StubLspBridge { enabled: true });
        let provider = LspProvider::new(bridge);
        let request =
            CompletionRequest::new(Some("main.rs".into()), "fr").with_language(Some("rust".into()));
        let result = provider.complete(&request);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].item.label, "from_lsp");
    }
}
