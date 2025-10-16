/// 單一地圖項目。 / Represents one entry in the document minimap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMapEntry {
    pub line: usize,
    pub preview: String,
}

/// 文件統計資料。 / Lightweight statistics for rendering side-bars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMetrics {
    pub line_count: usize,
    pub word_count: usize,
    pub char_count: usize,
}

/// 建構迷你地圖概要。 / Builds a coarse document map using target segments.
pub fn build_document_map(text: &str, target_segments: usize) -> Vec<DocumentMapEntry> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let total_lines = lines.len();
    if total_lines == 0 {
        return vec![DocumentMapEntry {
            line: 0,
            preview: String::new(),
        }];
    }

    let segments = target_segments.max(1);
    let step = (total_lines + segments - 1) / segments;

    let mut entries = Vec::new();
    let mut line_idx = 0usize;
    while line_idx < total_lines {
        let line = lines[line_idx];
        entries.push(DocumentMapEntry {
            line: line_idx,
            preview: summarise_line(line),
        });
        line_idx = line_idx.saturating_add(step);
    }
    entries
}

/// 統計文件資訊。 / Computes document metrics.
pub fn collect_metrics(text: &str) -> DocumentMetrics {
    let mut line_count = 0usize;
    let mut word_count = 0usize;
    let mut char_count = 0usize;
    let mut in_word = false;
    for ch in text.chars() {
        char_count += 1;
        if ch == '\n' {
            line_count += 1;
        }
        if ch.is_whitespace() {
            in_word = false;
        } else if !in_word {
            in_word = true;
            word_count += 1;
        }
    }
    if !text.ends_with('\n') {
        line_count += 1;
    }
    DocumentMetrics {
        line_count,
        word_count,
        char_count,
    }
}

fn summarise_line(line: &str) -> String {
    const MAX_PREVIEW: usize = 32;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.len() <= MAX_PREVIEW {
        trimmed.to_string()
    } else {
        format!("{}…", &trimmed[..MAX_PREVIEW])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_map_generates_even_segments() {
        let text = (0..100)
            .map(|i| format!("Line {i}\n"))
            .collect::<String>();
        let entries = build_document_map(&text, 10);
        assert!(entries.len() <= 10);
        assert_eq!(entries.first().unwrap().line, 0);
    }

    #[test]
    fn metrics_count_words_and_lines() {
        let metrics = collect_metrics("hello world\nsecond line");
        assert_eq!(metrics.line_count, 2);
        assert_eq!(metrics.word_count, 4);
        assert_eq!(metrics.char_count, 23);
    }
}
