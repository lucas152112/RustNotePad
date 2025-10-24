use crate::display::{Color, DisplayCommand, GlyphRun, Point, PrintDisplayList, Rectangle, Size};
use crate::job::{Margin, Orientation, PaperSize};

/// Wrap mode for layout calculations.
/// 分頁佈局時採用的換行模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    NoWrap,
    Word,
    Character,
}

/// Represents the printable area after applying margins.
/// 表示套用邊界後可列印的實際範圍。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrintableArea {
    pub width_pt: f32,
    pub height_pt: f32,
}

impl PrintableArea {
    pub fn from_paper(paper: PaperSize, orientation: Orientation, margin: Margin) -> Self {
        let (width, height) = paper.to_points(orientation);
        Self {
            width_pt: (width - margin.left - margin.right).max(0.0),
            height_pt: (height - margin.top - margin.bottom).max(0.0),
        }
    }
}

/// Options used by the paginator when constructing pages.
/// 分頁器在建立頁面時所依據的選項。
#[derive(Debug, Clone)]
pub struct LayoutOptions {
    pub paper: PaperSize,
    pub orientation: Orientation,
    pub margins: Margin,
    pub wrap_mode: WrapMode,
    pub dpi: f32,
    pub font_family: String,
    pub font_size_pt: f32,
    pub line_height_pt: f32,
    pub average_char_width_pt: f32,
}

/// Lightweight line range descriptor representing the editable buffer span.
/// 用於表示編輯緩衝區行範圍的精簡描述。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    /// Creates a new range with inclusive `start` and exclusive `end`.
    /// 建立一個包含 `start`、排除 `end` 的新行範圍。
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A single page layout containing the display list and original line mapping.
/// 單一頁面的佈局資訊，內含繪圖指令與原始行的對應。
#[derive(Debug, Clone)]
pub struct PageLayout {
    pub page_number: u32,
    pub line_ranges: Vec<LineRange>,
    pub display_list: PrintDisplayList,
}

impl PageLayout {
    pub fn new(
        page_number: u32,
        line_ranges: Vec<LineRange>,
        display_list: PrintDisplayList,
    ) -> Self {
        Self {
            page_number,
            line_ranges,
            display_list,
        }
    }
}

/// Paginator input data describing the document.
/// 分頁器用於描述文件的輸入介面。
pub trait LayoutInput {
    fn line_count(&self) -> usize;
    fn line_text(&self, index: usize) -> Option<&str>;
    fn highlight_spans(&self, index: usize) -> Vec<HighlightSpan> {
        let _ = index;
        Vec::new()
    }
}

/// Summary produced after pagination.
/// 分頁完成後的摘要資訊。
#[derive(Debug, Clone)]
pub struct LayoutSummary {
    pub total_pages: u32,
    pub total_lines: usize,
}

/// Result from running the paginator.
/// 分頁器執行後的整體結果。
#[derive(Debug, Clone)]
pub struct PaginationResult {
    pub pages: Vec<PageLayout>,
    pub summary: LayoutSummary,
}

/// Highlight span metadata used by renderers to colourise glyph runs.
/// 提供渲染器著色字元區段的高亮中繼資料。
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub range: std::ops::Range<usize>,
    pub foreground: Color,
    pub background: Option<Color>,
}

impl HighlightSpan {
    pub fn new(start: usize, end: usize, foreground: Color, background: Option<Color>) -> Self {
        Self {
            range: start..end,
            foreground,
            background,
        }
    }
}

/// Contract implemented by the pagination engine.
/// 分頁引擎需實作的介面契約。
pub trait Paginator {
    fn paginate(&self, input: &dyn LayoutInput, options: &LayoutOptions) -> PaginationResult;
}

/// Basic paginator that performs constant line-height pagination without wrapping.
/// 基礎分頁器：使用固定行高，且不進行換行計算。
#[derive(Debug, Default)]
pub struct SimplePaginator;

impl Paginator for SimplePaginator {
    fn paginate(&self, input: &dyn LayoutInput, options: &LayoutOptions) -> PaginationResult {
        let line_height = options.line_height_pt.max(1.0);
        let char_width = options.average_char_width_pt.max(1.0);
        let printable =
            PrintableArea::from_paper(options.paper, options.orientation, options.margins);
        let height_available = printable.height_pt.max(line_height);
        let lines_per_page = (height_available / line_height).floor().max(1.0) as usize;
        let total_lines = input.line_count();

        let mut pages = Vec::new();
        let mut current_display = PrintDisplayList::default();
        let mut current_line_start = 0usize;
        let mut current_line_count = 0usize;
        let mut current_page_number = 1u32;

        let font_family = options.font_family.clone();
        let font_size = options.font_size_pt;
        let baseline_start = options.margins.top;
        let origin_x = options.margins.left;

        for line_idx in 0..total_lines {
            if current_line_count == lines_per_page {
                pages.push(PageLayout::new(
                    current_page_number,
                    vec![LineRange::new(current_line_start, line_idx)],
                    current_display,
                ));
                current_display = PrintDisplayList::default();
                current_line_start = line_idx;
                current_line_count = 0;
                current_page_number += 1;
            }

            let y = baseline_start + (current_line_count as f32 * line_height);
            let text = input.line_text(line_idx).unwrap_or_default();
            let char_len = text.chars().count();

            let mut spans = input.highlight_spans(line_idx);
            if spans.is_empty() {
                spans.push(HighlightSpan::new(
                    0,
                    char_len,
                    Color::new(0.0, 0.0, 0.0, 1.0),
                    None,
                ));
            }

            for span in spans {
                let start = span.range.start.min(char_len);
                let end = span.range.end.min(char_len);
                if start >= end {
                    continue;
                }

                let (start_byte, end_byte) = char_range_to_byte_range(text, start, end);
                let slice = &text[start_byte..end_byte];
                if slice.is_empty() {
                    continue;
                }

                let x = origin_x + (start as f32) * char_width;
                let width = (end - start) as f32 * char_width;

                if let Some(bg) = span.background {
                    current_display.push(DisplayCommand::BackgroundRect(Rectangle {
                        origin: Point { x, y },
                        size: Size {
                            width,
                            height: line_height,
                        },
                        color: bg,
                    }));
                }

                current_display.push(DisplayCommand::GlyphRun(GlyphRun {
                    text: slice.to_string(),
                    font_family: font_family.clone(),
                    font_size_pt: font_size,
                    position: Point { x, y },
                    color: span.foreground,
                    background: span.background,
                }));
            }

            current_line_count += 1;
        }

        if current_line_count > 0 {
            pages.push(PageLayout::new(
                current_page_number,
                vec![LineRange::new(
                    current_line_start,
                    current_line_start + current_line_count,
                )],
                current_display,
            ));
        } else {
            // Ensure at least one page even for empty documents.
            pages.push(PageLayout::new(1, Vec::new(), PrintDisplayList::default()));
        }

        let summary = LayoutSummary {
            total_pages: pages.len() as u32,
            total_lines,
        };

        PaginationResult { pages, summary }
    }
}

fn char_range_to_byte_range(text: &str, start: usize, end: usize) -> (usize, usize) {
    (
        char_pos_to_byte_index(text, start),
        char_pos_to_byte_index(text, end),
    )
}

fn char_pos_to_byte_index(text: &str, target: usize) -> usize {
    if target == 0 {
        return 0;
    }

    let mut count = 0usize;
    for (idx, _) in text.char_indices() {
        if count == target {
            return idx;
        }
        count += 1;
    }

    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::DisplayCommand;
    use crate::job::{Margin, Orientation, PaperId, PaperSize};
    use ron::de::from_str as ron_from_str;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    struct VecLayoutInput {
        lines: Vec<String>,
        spans: HashMap<usize, Vec<HighlightSpan>>,
    }

    impl VecLayoutInput {
        fn new(lines: Vec<String>) -> Self {
            Self {
                lines,
                spans: HashMap::new(),
            }
        }

        fn with_spans(lines: Vec<String>, spans: HashMap<usize, Vec<HighlightSpan>>) -> Self {
            Self { lines, spans }
        }
    }

    impl LayoutInput for VecLayoutInput {
        fn line_count(&self) -> usize {
            self.lines.len()
        }

        fn line_text(&self, index: usize) -> Option<&str> {
            self.lines.get(index).map(|line| line.as_str())
        }

        fn highlight_spans(&self, index: usize) -> Vec<HighlightSpan> {
            self.spans.get(&index).cloned().unwrap_or_default()
        }
    }

    fn make_options() -> LayoutOptions {
        let height_mm = (60.0 / 72.0) * 25.4;
        LayoutOptions {
            paper: PaperSize::new(PaperId::Custom, 50.0, height_mm as f32),
            orientation: Orientation::Portrait,
            margins: Margin::zero(),
            wrap_mode: WrapMode::NoWrap,
            dpi: 96.0,
            font_family: "JetBrains Mono".to_string(),
            font_size_pt: 11.0,
            line_height_pt: 12.0,
            average_char_width_pt: 7.0,
        }
    }

    #[test]
    fn paginates_into_multiple_pages() {
        let lines: Vec<String> = (0..12).map(|idx| format!("Line {idx}")).collect();
        let input = VecLayoutInput::new(lines);
        let options = make_options();
        let paginator = SimplePaginator::default();

        let result = paginator.paginate(&input, &options);
        assert_eq!(result.summary.total_lines, 12);
        assert_eq!(result.summary.total_pages, 3);

        assert_eq!(result.pages.len(), 3);
        assert_eq!(result.pages[0].line_ranges, vec![LineRange::new(0, 5)]);
        assert_eq!(result.pages[1].line_ranges, vec![LineRange::new(5, 10)]);
        assert_eq!(result.pages[2].line_ranges, vec![LineRange::new(10, 12)]);
        assert_eq!(result.pages[0].display_list.commands.len(), 5);
        assert_eq!(result.pages[1].display_list.commands.len(), 5);
        assert_eq!(result.pages[2].display_list.commands.len(), 2);
    }

    #[test]
    fn empty_document_produces_single_blank_page() {
        let input = VecLayoutInput::new(Vec::new());
        let options = make_options();
        let paginator = SimplePaginator::default();

        let result = paginator.paginate(&input, &options);
        assert_eq!(result.summary.total_lines, 0);
        assert_eq!(result.summary.total_pages, 1);
        assert_eq!(result.pages.len(), 1);
        assert!(result.pages[0].display_list.is_empty());
        assert!(result.pages[0].line_ranges.is_empty());
    }

    #[derive(Debug, Deserialize)]
    struct PaginationFixture {
        lines: Vec<String>,
        expected_ranges: Vec<(usize, usize)>,
    }

    #[test]
    fn pagination_matches_fixture_ranges() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/feature_parity/09-printing/tests/pagination/simple_text.ron");
        let fixture_text = fs::read_to_string(&fixture_path)
            .unwrap_or_else(|err| panic!("Failed to read {:?}: {err}", fixture_path));
        let fixture: PaginationFixture = ron_from_str(&fixture_text)
            .unwrap_or_else(|err| panic!("Failed to parse {:?}: {err}", fixture_path));

        let input = VecLayoutInput::new(fixture.lines);
        let options = make_options();
        let paginator = SimplePaginator::default();
        let result = paginator.paginate(&input, &options);

        let actual_ranges: Vec<(usize, usize)> = result
            .pages
            .iter()
            .flat_map(|page| {
                page.line_ranges
                    .iter()
                    .map(|range| (range.start, range.end))
            })
            .collect();

        assert_eq!(actual_ranges, fixture.expected_ranges);
        assert_eq!(
            result.summary.total_pages as usize,
            fixture.expected_ranges.len()
        );
    }

    #[test]
    fn highlight_spans_are_used_for_colours_and_backgrounds() {
        let mut spans = HashMap::new();
        spans.insert(
            0,
            vec![
                HighlightSpan::new(0, 2, Color::new(1.0, 0.0, 0.0, 1.0), None),
                HighlightSpan::new(
                    2,
                    4,
                    Color::new(0.0, 0.0, 1.0, 1.0),
                    Some(Color::new(0.9, 0.9, 0.2, 1.0)),
                ),
            ],
        );

        let input = VecLayoutInput::with_spans(vec!["rust".into()], spans);
        let options = make_options();
        let paginator = SimplePaginator::default();
        let result = paginator.paginate(&input, &options);

        let commands = &result.pages[0].display_list.commands;
        assert_eq!(commands.len(), 3);

        match &commands[0] {
            DisplayCommand::GlyphRun(run) => {
                assert_eq!(run.text, "ru");
                assert_eq!(run.color.r, 1.0);
                assert!(run.background.is_none());
            }
            other => panic!("Unexpected command: {:?}", other),
        }

        match &commands[1] {
            DisplayCommand::BackgroundRect(rect) => {
                assert_eq!(rect.color.r, 0.9);
                assert!(rect.size.width > 0.0);
            }
            other => panic!("Expected background rect, got {:?}", other),
        }

        match &commands[2] {
            DisplayCommand::GlyphRun(run) => {
                assert_eq!(run.text, "st");
                assert_eq!(run.color.b, 1.0);
                assert!(run.background.is_some());
            }
            other => panic!("Unexpected command: {:?}", other),
        }
    }
}
