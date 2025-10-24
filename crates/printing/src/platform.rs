use crate::display::PrintDisplayList;
use crate::job::{PrintJobId, PrintJobOptions};
#[cfg(test)]
use std::sync::{Arc, Mutex};

/// Represents a rasterized or vector-ready page queued for spooling.
/// 表示待送往列印佇列的點陣化或向量頁面。
#[derive(Debug, Clone)]
pub struct SpoolPage {
    pub job_id: PrintJobId,
    pub page_number: u32,
    pub display_list: PrintDisplayList,
}

/// Handle returned when a platform adapter begins a job.
/// 平台列印介面開始作業時回傳的控制物件。
pub trait PlatformJobHandle {
    type Error;

    fn submit_page(&mut self, page: SpoolPage) -> Result<(), Self::Error>;
    fn finish(self) -> Result<(), Self::Error>;
    fn abort(self, reason: &str);
}

/// Abstraction over platform-specific print APIs.
/// 平台列印 API 的抽象介面。
pub trait PlatformAdapter: Send + Sync {
    type Error;
    type JobHandle: PlatformJobHandle<Error = Self::Error>;

    fn begin_job(&self, options: &PrintJobOptions) -> Result<Self::JobHandle, Self::Error>;
}

/// Recorded job metadata produced by the mock adapter.
/// 模擬介面所記錄的列印作業中繼資料。
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct RecordedJob {
    pub options: PrintJobOptions,
    pub pages: Vec<SpoolPage>,
    pub aborted: bool,
    pub abort_reason: Option<String>,
}

/// In-memory implementation of [`PlatformAdapter`] used for tests.
/// 測試使用的記憶體內部平台介面實作。
#[cfg(test)]
#[derive(Clone, Default)]
pub struct MockPlatformAdapter {
    jobs: Arc<Mutex<Vec<RecordedJob>>>,
}

#[cfg(test)]
impl MockPlatformAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn drain_jobs(&self) -> Vec<RecordedJob> {
        self.jobs.lock().expect("lock poisoned").drain(..).collect()
    }
}

#[cfg(test)]
pub struct MockJobHandle {
    options: PrintJobOptions,
    pages: Vec<SpoolPage>,
    sink: Arc<Mutex<Vec<RecordedJob>>>,
}

#[cfg(test)]
impl PlatformAdapter for MockPlatformAdapter {
    type Error = ();
    type JobHandle = MockJobHandle;

    fn begin_job(&self, options: &PrintJobOptions) -> Result<Self::JobHandle, Self::Error> {
        Ok(MockJobHandle {
            options: options.clone(),
            pages: Vec::new(),
            sink: self.jobs.clone(),
        })
    }
}

#[cfg(test)]
impl PlatformJobHandle for MockJobHandle {
    type Error = ();

    fn submit_page(&mut self, page: SpoolPage) -> Result<(), Self::Error> {
        self.pages.push(page);
        Ok(())
    }

    fn finish(self) -> Result<(), Self::Error> {
        let mut guard = self.sink.lock().expect("lock poisoned");
        guard.push(RecordedJob {
            options: self.options,
            pages: self.pages,
            aborted: false,
            abort_reason: None,
        });
        Ok(())
    }

    fn abort(self, reason: &str) {
        let mut guard = self.sink.lock().expect("lock poisoned");
        guard.push(RecordedJob {
            options: self.options,
            pages: Vec::new(),
            aborted: true,
            abort_reason: Some(reason.to_string()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::Color;
    use crate::job::{
        DuplexMode, Margin, Orientation, PageRange, PaperId, PaperSize, PrintColorMode,
        PrintJobOptions, PrintTarget, TargetCapabilities,
    };
    use crate::layout::{
        HighlightSpan, LayoutInput, LayoutOptions, Paginator, SimplePaginator, WrapMode,
    };
    use crate::template::HeaderFooterTemplate;

    struct StaticInput;

    impl LayoutInput for StaticInput {
        fn line_count(&self) -> usize {
            1
        }

        fn line_text(&self, _: usize) -> Option<&str> {
            Some("mock line")
        }

        fn highlight_spans(&self, _: usize) -> Vec<HighlightSpan> {
            vec![HighlightSpan::new(
                0,
                4,
                Color::new(0.0, 0.0, 0.0, 1.0),
                None,
            )]
        }
    }

    fn make_options() -> LayoutOptions {
        LayoutOptions {
            paper: PaperSize::new(PaperId::A4, 210.0, 297.0),
            orientation: Orientation::Portrait,
            margins: Margin::zero(),
            wrap_mode: WrapMode::NoWrap,
            dpi: 96.0,
            font_family: "JetBrains Mono".into(),
            font_size_pt: 11.0,
            line_height_pt: 12.0,
            average_char_width_pt: 7.0,
        }
    }

    fn make_job_options() -> PrintJobOptions {
        PrintJobOptions::new(
            Some(PrintTarget {
                name: "Mock".into(),
                capabilities: TargetCapabilities::new(true, true, true),
            }),
            PaperSize::new(PaperId::A4, 210.0, 297.0),
            Orientation::Portrait,
            Margin::zero(),
            1,
            PrintColorMode::Color,
            DuplexMode::Off,
            PageRange::All,
            HeaderFooterTemplate::default(),
            HeaderFooterTemplate::default(),
        )
    }

    #[allow(unused_mut)]
    #[test]
    fn mock_adapter_captures_pages() {
        let adapter = MockPlatformAdapter::default();
        let job_options = make_job_options();
        let job_id = job_options.job_id;
        let paginator = SimplePaginator::default();
        let layout = paginator.paginate(&StaticInput, &make_options());

        let mut handle = adapter.begin_job(&job_options).unwrap();
        for page in layout.pages {
            handle
                .submit_page(SpoolPage {
                    job_id,
                    page_number: page.page_number,
                    display_list: page.display_list,
                })
                .unwrap();
        }
        handle.finish().unwrap();

        let jobs = adapter.drain_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].pages.len(), 1);
        assert!(!jobs[0].aborted);
        assert_eq!(jobs[0].options.job_id, job_id);
        assert!(jobs[0].abort_reason.is_none());
    }

    #[test]
    fn mock_adapter_records_abort_reason() {
        let adapter = MockPlatformAdapter::default();
        let job_options = make_job_options();
        let handle = adapter.begin_job(&job_options).unwrap();
        handle.abort("user cancelled");

        let jobs = adapter.drain_jobs();
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].aborted);
        assert_eq!(jobs[0].abort_reason.as_deref(), Some("user cancelled"));
    }
}
