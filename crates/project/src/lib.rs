//! Project/session/workspace management primitives for RustNotePad.
//! 管理 RustNotePad 工作階段、專案樹與工作區的核心模組。

mod serde_path;
mod util;

pub mod session;
pub mod tree;
pub mod tree_store;
pub mod workspace;

pub use session::{
    AutosaveManifest, AutosaveStore, SessionCaret, SessionError, SessionMetadata, SessionScroll,
    SessionSelection, SessionSnapshot, SessionStore, SessionTab, SessionWindow, UnsavedHash,
};
pub use tree::{
    NodeMetadata, ProjectFilter, ProjectNode, ProjectNodeDraft, ProjectNodeId, ProjectNodeKind,
    ProjectTree, ProjectTreeDiff, ProjectTreeError,
};
pub use tree_store::{ProjectTreeStore, ProjectTreeStoreError};
pub use workspace::{
    WorkspaceCache, WorkspaceDescriptor, WorkspaceError, WorkspaceId, WorkspaceIndexEntry,
    WorkspaceStore,
};
