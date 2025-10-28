use std::io::Cursor;
use std::sync::{Arc, Mutex};

use image::io::Reader as ImageReader;
use rustnotepad_printing::job::{DuplexMode, PageRange, PaperId, PaperSize};
use rustnotepad_printing::platform::{PlatformAdapter, PlatformJobHandle, SpoolPage};
use rustnotepad_printing::{
    run_print_job, DisplayCommand, HeaderFooterTemplate, LayoutInput, LayoutOptions, Margin,
    Orientation, PreviewCache, PreviewConfig, PrintColorMode, PrintJobControllerState,
    PrintJobOptions, PrintTarget, SimplePaginator, TargetCapabilities, WrapMode,
};

struct VecLayoutInput {
    lines: Vec<String>,
}

impl VecLayoutInput {
    fn new(lines: Vec<String>) -> Self {
        Self { lines }
    }
}

impl LayoutInput for VecLayoutInput {
    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn line_text(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(|line| line.as_str())
    }

    fn highlight_spans(&self, index: usize) -> Vec<rustnotepad_printing::HighlightSpan> {
        let _ = index;
        Vec::new()
    }
}

#[derive(Clone, Default)]
struct RecordingAdapter {
    jobs: Arc<Mutex<Vec<RecordedJob>>>,
}

#[derive(Clone)]
struct RecordedJob {
    pages: Vec<SpoolPage>,
    finished: bool,
}

struct RecordingHandle {
    pages: Vec<SpoolPage>,
    sink: Arc<Mutex<Vec<RecordedJob>>>,
}

impl PlatformAdapter for RecordingAdapter {
    type Error = String;
    type JobHandle = RecordingHandle;

    fn begin_job(&self, options: &PrintJobOptions) -> Result<Self::JobHandle, Self::Error> {
        let _ = options;
        Ok(RecordingHandle {
            pages: Vec::new(),
            sink: self.jobs.clone(),
        })
    }
}

impl PlatformJobHandle for RecordingHandle {
    type Error = String;

    fn submit_page(&mut self, page: SpoolPage) -> Result<(), Self::Error> {
        self.pages.push(page);
        Ok(())
    }

    fn finish(self) -> Result<(), Self::Error> {
        let mut guard = self.sink.lock().unwrap();
        guard.push(RecordedJob {
            pages: self.pages,
            finished: true,
        });
        Ok(())
    }

    fn abort(self, _reason: &str) {
        let mut guard = self.sink.lock().unwrap();
        guard.push(RecordedJob {
            pages: Vec::new(),
            finished: false,
        });
    }
}

fn make_layout_options() -> LayoutOptions {
    LayoutOptions {
        paper: PaperSize::new(PaperId::A4, 210.0, 297.0),
        orientation: Orientation::Portrait,
        margins: Margin {
            top: 36.0,
            bottom: 36.0,
            left: 36.0,
            right: 36.0,
        },
        wrap_mode: WrapMode::NoWrap,
        dpi: 96.0,
        font_family: "JetBrains Mono".to_string(),
        font_size_pt: 11.0,
        line_height_pt: 14.0,
        average_char_width_pt: 7.0,
    }
}

fn make_job_options() -> PrintJobOptions {
    PrintJobOptions::new(
        Some(PrintTarget {
            name: "Test".into(),
            capabilities: TargetCapabilities::new(true, true, true),
        }),
        PaperSize::new(PaperId::A4, 210.0, 297.0),
        Orientation::Portrait,
        Margin {
            top: 36.0,
            bottom: 36.0,
            left: 36.0,
            right: 36.0,
        },
        1,
        PrintColorMode::Color,
        DuplexMode::Off,
        PageRange::All,
        HeaderFooterTemplate::parse("&lRustNotePad&c&p&r&P").unwrap(),
        HeaderFooterTemplate::parse("&l&cSample Footer&r").unwrap(),
    )
}

#[test]
fn pipeline_generates_pdf_and_previews() {
    let lines: Vec<String> = (0..30)
        .map(|n| format!("fn main_{n}() {{ println!(\"{n}\"); }}"))
        .collect();
    let input = VecLayoutInput::new(lines);
    let layout_options = make_layout_options();
    let job_options = make_job_options();
    let adapter = RecordingAdapter::default();
    let mut cache = PreviewCache::with_capacity(16);
    let preview = PreviewConfig {
        cache: &mut cache,
        zoom_levels: &[100, 150],
        base_dpi: 96,
    };

    let result = run_print_job(
        &SimplePaginator::default(),
        &input,
        &layout_options,
        &job_options,
        &adapter,
        Some(preview),
    )
    .expect("print job");

    assert_eq!(result.state, PrintJobControllerState::Completed);
    assert!(result.summary.total_pages >= 1);
    assert!(result.pdf_data.starts_with(b"%PDF"));

    let jobs = adapter.jobs.lock().unwrap();
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].finished);
    assert!(!jobs[0].pages.is_empty());

    let mut found_rule = false;
    let mut found_header_text = false;
    for page in &jobs[0].pages {
        for command in &page.display_list.commands {
            match command {
                DisplayCommand::HorizontalRule { .. } => found_rule = true,
                DisplayCommand::GlyphRun(run) => {
                    if run.text.contains("RustNotePad") {
                        found_header_text = true;
                    }
                }
                _ => {}
            }
        }
    }
    assert!(found_rule, "expected horizontal rules for header/footer");
    assert!(found_header_text, "expected header glyph run");

    let expected_previews = jobs[0].pages.len() * 2;
    assert!(
        cache.len() >= expected_previews,
        "expected at least {expected_previews} preview entries, got {}",
        cache.len()
    );

    for page_idx in 1..=jobs[0].pages.len() as u32 {
        for zoom in [100, 150] {
            let key = rustnotepad_printing::PrintPreviewKey {
                job_id: job_options.job_id,
                page: page_idx,
                zoom_percent: zoom,
            };
            let entry = cache
                .get(&key)
                .unwrap_or_else(|| panic!("missing preview for page {page_idx} @ {zoom}%"));
            ImageReader::new(Cursor::new(&entry.data))
                .with_guessed_format()
                .expect("is png")
                .decode()
                .expect("decode preview");
        }
    }
}

#[test]
fn pdf_contains_expected_tokens() {
    let lines = vec![
        "fn main() {".to_string(),
        "    println!(\"hello\");".to_string(),
        "}".to_string(),
    ];
    let input = VecLayoutInput::new(lines);
    let layout_options = make_layout_options();
    let job_options = make_job_options();
    let adapter = RecordingAdapter::default();

    let result = run_print_job(
        &SimplePaginator::default(),
        &input,
        &layout_options,
        &job_options,
        &adapter,
        None,
    )
    .expect("print job");

    let pdf_text = String::from_utf8_lossy(&result.pdf_data);
    assert!(pdf_text.contains("/Type /Catalog"));
    assert!(pdf_text.contains("/Type /Page"));
    assert!(pdf_text.contains("RustNotePad"));
    assert!(pdf_text.contains("hello"));
}
