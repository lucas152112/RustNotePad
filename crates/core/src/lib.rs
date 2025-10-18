pub mod bookmarks;
pub mod column_ops;
pub mod document;
pub mod document_map;
pub mod editor;
pub mod file_monitor;
pub mod folding;
pub mod line_ops;
pub mod recovery;
pub mod search_session;
pub mod split_view;

pub use bookmarks::BookmarkManager;
pub use column_ops::ColumnSelection;
pub use document::{Document, DocumentError, Encoding, LegacyEncoding, LineEnding};
pub use document_map::{DocumentMapEntry, DocumentMetrics};
pub use editor::{Caret, EditorBuffer, EditorError, Selection};
pub use file_monitor::{FileEvent, FileMonitor, FileMonitorError, FileMonitorEventKind};
pub use folding::{FoldRegion, FoldTree};
pub use line_ops::{CaseTransform, SortOrder};
pub use recovery::{RecoveryEntry, RecoveryManager};
pub use rustnotepad_search::{
    SearchDirection, SearchError, SearchMatch, SearchMode, SearchOptions, SearchReport, SearchScope,
};
pub use search_session::SearchSession;
pub use split_view::{MultiInstancePolicy, Pane, SplitViewState, TabId, TabRecord};
