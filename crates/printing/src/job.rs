use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::template::HeaderFooterTemplate;

/// Opaque identifier for a print job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrintJobId(u64);

impl PrintJobId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for PrintJobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "print-job-{}", self.0)
    }
}

/// Orientation of a print page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Duplex (two-sided) printing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplexMode {
    Off,
    LongEdge,
    ShortEdge,
}

/// Colour mode for the printer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintColorMode {
    Color,
    Grayscale,
}

/// Inclusive page range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageRange {
    All,
    Range { start: u32, end: u32 },
    Selection(Vec<u32>),
}

impl PageRange {
    pub fn contains(&self, page: u32) -> bool {
        match self {
            PageRange::All => true,
            PageRange::Range { start, end } => *start <= page && page <= *end,
            PageRange::Selection(set) => set.contains(&page),
        }
    }
}

/// Margin values expressed in points (1/72").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margin {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Margin {
    pub const fn zero() -> Self {
        Self {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: 0.0,
        }
    }
}

/// Supported paper identifiers for quick selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaperId {
    A4,
    Letter,
    Legal,
    A3,
    Custom,
}

/// Represents a paper size in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperSize {
    pub id: PaperId,
    pub width_mm: f32,
    pub height_mm: f32,
}

impl PaperSize {
    pub const fn new(id: PaperId, width_mm: f32, height_mm: f32) -> Self {
        Self {
            id,
            width_mm,
            height_mm,
        }
    }

    pub const fn to_points(&self, orientation: Orientation) -> (f32, f32) {
        const MM_PER_INCH: f32 = 25.4;
        let width_in = self.width_mm / MM_PER_INCH;
        let height_in = self.height_mm / MM_PER_INCH;
        let width_pt = width_in * 72.0;
        let height_pt = height_in * 72.0;
        match orientation {
            Orientation::Portrait => (width_pt, height_pt),
            Orientation::Landscape => (height_pt, width_pt),
        }
    }
}

/// Printer target metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintTarget {
    pub name: String,
    pub capabilities: TargetCapabilities,
}

/// Feature capabilities exposed by a printer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetCapabilities {
    pub supports_color: bool,
    pub supports_duplex: bool,
    pub supports_vector: bool,
}

impl TargetCapabilities {
    pub const fn new(supports_color: bool, supports_duplex: bool, supports_vector: bool) -> Self {
        Self {
            supports_color,
            supports_duplex,
            supports_vector,
        }
    }
}

impl Default for TargetCapabilities {
    fn default() -> Self {
        Self::new(true, true, true)
    }
}

/// Options supplied when requesting a print job.
#[derive(Debug, Clone)]
pub struct PrintJobOptions {
    pub job_id: PrintJobId,
    pub target: Option<PrintTarget>,
    pub paper: PaperSize,
    pub orientation: Orientation,
    pub margins: Margin,
    pub copies: u32,
    pub color_mode: PrintColorMode,
    pub duplex: DuplexMode,
    pub page_range: PageRange,
    pub header_template: Arc<HeaderFooterTemplate>,
    pub footer_template: Arc<HeaderFooterTemplate>,
}

impl PrintJobOptions {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        target: Option<PrintTarget>,
        paper: PaperSize,
        orientation: Orientation,
        margins: Margin,
        copies: u32,
        color_mode: PrintColorMode,
        duplex: DuplexMode,
        page_range: PageRange,
        header_template: HeaderFooterTemplate,
        footer_template: HeaderFooterTemplate,
    ) -> Self {
        Self {
            job_id: PrintJobId::new(),
            target,
            paper,
            orientation,
            margins,
            copies,
            color_mode,
            duplex,
            page_range,
            header_template: Arc::new(header_template),
            footer_template: Arc::new(footer_template),
        }
    }
}

/// Controller state for analytics/UI markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintJobControllerState {
    Idle,
    Layout,
    Render,
    Spooling,
    Completed,
    Failed,
}
