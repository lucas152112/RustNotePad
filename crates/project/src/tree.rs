use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::serde_path;

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier assigned to each node in the project tree.  
/// 專案樹中每個節點的唯一識別碼。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectNodeId(u64);

impl ProjectNodeId {
    pub fn new() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ProjectNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Metadata describing node-specific annotations.  
/// 節點的附註資訊（顏色標籤、語言覆寫、最後開啟時間等）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct NodeMetadata {
    #[serde(default)]
    pub color_tag: Option<String>,
    #[serde(default)]
    pub language_override: Option<String>,
    #[serde(default)]
    pub last_opened_unix: Option<i64>,
}

/// Filters applied to folders for quick-open or scoped search.  
/// 用於資料夾的篩選規則，支援快速開啟與侷限搜尋。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectFilter {
    Suffix(String),
    Glob(String),
    Regex(String),
}

/// The kind of project-node.  
/// 專案節點的類型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectNodeKind {
    Folder {
        #[serde(
            default,
            with = "crate::serde_path::option",
            skip_serializing_if = "Option::is_none"
        )]
        path: Option<PathBuf>,
        #[serde(default)]
        filters: Vec<ProjectFilter>,
    },
    File {
        #[serde(with = "serde_path")]
        path: PathBuf,
    },
    Virtual {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

impl ProjectNodeKind {
    pub fn is_folder(&self) -> bool {
        matches!(self, ProjectNodeKind::Folder { .. })
    }
}

/// Immutable project node stored inside the tree.  
/// 專案樹內部的不可變節點。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectNode {
    pub id: ProjectNodeId,
    pub name: String,
    pub kind: ProjectNodeKind,
    #[serde(default)]
    pub metadata: NodeMetadata,
    #[serde(default)]
    pub children: Vec<ProjectNode>,
}

impl ProjectNode {
    pub fn is_folder(&self) -> bool {
        self.kind.is_folder()
    }
}

/// Helper to construct a new node with metadata.  
/// 協助建立帶有中繼資料的新節點。
#[derive(Debug, Clone)]
pub struct ProjectNodeDraft {
    pub name: String,
    pub kind: ProjectNodeKind,
    pub metadata: NodeMetadata,
}

impl ProjectNodeDraft {
    pub fn new(name: impl Into<String>, kind: ProjectNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            metadata: NodeMetadata::default(),
        }
    }

    pub fn with_metadata(mut self, metadata: NodeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    fn build(self) -> ProjectNode {
        ProjectNode {
            id: ProjectNodeId::new(),
            name: self.name,
            kind: self.kind,
            metadata: self.metadata,
            children: Vec::new(),
        }
    }
}

/// Immutable project tree root.  
/// 專案樹根節點集合。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectTree {
    pub revision: u64,
    pub root: ProjectNode,
}

impl ProjectTree {
    /// Constructs an empty tree with a single folder root.  
    /// 建立僅含資料夾根節點的空專案樹。
    pub fn empty(root_name: impl Into<String>, root_path: Option<PathBuf>) -> Self {
        let root_node = ProjectNode {
            id: ProjectNodeId::new(),
            name: root_name.into(),
            kind: ProjectNodeKind::Folder {
                path: root_path,
                filters: Vec::new(),
            },
            metadata: NodeMetadata::default(),
            children: Vec::new(),
        };
        Self {
            revision: 0,
            root: root_node,
        }
    }

    /// Returns the identifier of the root node.  
    /// 取得根節點的識別碼。
    pub fn root_id(&self) -> ProjectNodeId {
        self.root.id
    }

    /// Adds a new child node under the specified parent.  
    /// 在指定的父節點下方新增子節點。
    pub fn add_child(
        &self,
        parent_id: ProjectNodeId,
        draft: ProjectNodeDraft,
    ) -> Result<(Self, ProjectTreeDiff), ProjectTreeError> {
        let new_node = draft.build();
        let mut diff = ProjectTreeDiff::default();
        diff.added.push(new_node.id);

        let (root, inserted) = add_child_recursive(&self.root, parent_id, new_node, &mut diff)?;
        if !inserted {
            return Err(ProjectTreeError::NodeNotFound(parent_id));
        }

        let mut next = self.clone();
        next.root = root;
        next.revision = self.revision.wrapping_add(1);
        Ok((next, diff))
    }

    /// Finds a node by identifier.  
    /// 依識別碼尋找節點。
    pub fn find(&self, id: ProjectNodeId) -> Option<&ProjectNode> {
        find_recursive(&self.root, id)
    }
}

fn find_recursive<'a>(node: &'a ProjectNode, id: ProjectNodeId) -> Option<&'a ProjectNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_recursive(child, id) {
            return Some(found);
        }
    }
    None
}

fn add_child_recursive(
    current: &ProjectNode,
    parent_id: ProjectNodeId,
    new_child: ProjectNode,
    diff: &mut ProjectTreeDiff,
) -> Result<(ProjectNode, bool), ProjectTreeError> {
    if current.id == parent_id {
        if !current.is_folder() {
            return Err(ProjectTreeError::InvalidParent(parent_id));
        }
        let mut updated = current.clone();
        updated.children.push(new_child);
        diff.updated.push(parent_id);
        return Ok((updated, true));
    }

    let mut inserted = false;
    let mut children = Vec::with_capacity(current.children.len());
    for child in &current.children {
        if inserted {
            children.push(child.clone());
            continue;
        }
        let (candidate, did_insert) =
            add_child_recursive(child, parent_id, new_child.clone(), diff)?;
        if did_insert {
            inserted = true;
        }
        children.push(candidate);
    }

    if inserted {
        let mut updated = current.clone();
        updated.children = children;
        Ok((updated, true))
    } else {
        Ok((current.clone(), false))
    }
}

/// Captures differences after a tree mutation.  
/// 紀錄樹狀結構變動後的差異。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectTreeDiff {
    pub added: Vec<ProjectNodeId>,
    pub removed: Vec<ProjectNodeId>,
    pub updated: Vec<ProjectNodeId>,
}

/// Tree-manipulation errors.  
/// 專案樹操作錯誤類型。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectTreeError {
    #[error("node {0} not found")]
    NodeNotFound(ProjectNodeId),
    #[error("node {0} cannot accept children")]
    InvalidParent(ProjectNodeId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_child_creates_new_revision() {
        let tree = ProjectTree::empty("Root", None);
        let root_id = tree.root_id();
        let draft = ProjectNodeDraft::new(
            "src",
            ProjectNodeKind::Folder {
                path: None,
                filters: vec![ProjectFilter::Suffix(".rs".into())],
            },
        );

        let (tree, diff) = tree.add_child(root_id, draft).unwrap();
        assert_eq!(tree.revision, 1);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.updated, vec![root_id]);
        let root = tree.find(root_id).unwrap();
        assert_eq!(root.children.len(), 1);
        assert!(root.children[0].is_folder());
    }

    #[test]
    fn add_child_errors_on_non_folder_parent() {
        let tree = ProjectTree::empty("Root", None);
        let root_id = tree.root_id();
        let draft_folder = ProjectNodeDraft::new(
            "docs",
            ProjectNodeKind::Folder {
                path: None,
                filters: Vec::new(),
            },
        );
        let (tree, _) = tree.add_child(root_id, draft_folder).unwrap();
        let child_id = tree
            .find(root_id)
            .and_then(|node| node.children.first())
            .map(|node| node.id)
            .expect("folder child should exist");

        let draft_file = ProjectNodeDraft::new(
            "readme.md",
            ProjectNodeKind::File {
                path: PathBuf::from("README.md"),
            },
        );
        let (tree, _) = tree.add_child(child_id, draft_file).unwrap();

        let file_id = tree
            .find(child_id)
            .and_then(|node| node.children.first())
            .map(|node| node.id)
            .expect("file child should exist");
        let draft_again = ProjectNodeDraft::new(
            "another",
            ProjectNodeKind::File {
                path: PathBuf::from("ANOTHER.md"),
            },
        );
        let err = tree.add_child(file_id, draft_again).unwrap_err();
        assert_eq!(err, ProjectTreeError::InvalidParent(file_id));
    }
}
