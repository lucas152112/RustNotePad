//! Printing pipeline types and utilities shared by GUI/CLI components.

pub mod controller;
pub mod display;
pub mod job;
pub mod layout;
pub mod platform;
pub mod preview;
pub mod template;

pub use controller::{run_print_job, PreviewConfig, PrintJobError, PrintJobResult};
pub use display::{DisplayCommand, GlyphRun, PrintDisplayList};
pub use job::{
    DuplexMode, Margin, Orientation, PageRange, PaperId, PaperSize, PrintColorMode,
    PrintJobControllerState, PrintJobId, PrintJobOptions, PrintTarget, TargetCapabilities,
};
pub use layout::{
    HighlightSpan, LayoutInput, LayoutOptions, LayoutSummary, LineRange, PageLayout,
    PaginationResult, Paginator, PrintableArea, SimplePaginator, WrapMode,
};
pub use platform::{PlatformAdapter, PlatformJobHandle, SpoolPage};
pub use preview::{PreviewCache, PreviewEntry, PrintPreviewKey};
pub use template::{
    HeaderFooterContext, HeaderFooterTemplate, RenderedHeaderFooter, TemplateError,
    TemplateSegment, TemplateToken,
};
