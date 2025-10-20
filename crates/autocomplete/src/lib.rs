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
}
