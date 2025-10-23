use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::tree::ProjectTree;
use crate::util::write_atomic;

/// Persists `ProjectTree` snapshots to disk using JSON + atomic writes.  
/// 以 JSON 搭配原子寫入方式儲存 `ProjectTree` 快照。
#[derive(Debug)]
pub struct ProjectTreeStore {
    path: PathBuf,
}

impl ProjectTreeStore {
    /// Constructs a store bound to the provided path.  
    /// 建立綁定至指定路徑的儲存器。
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Returns the backing path used for persistence.  
    /// 取得此儲存器使用的檔案路徑。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads a project tree from disk, returning `Ok(None)` when the file is absent.  
    /// 從磁碟載入專案樹；若檔案不存在則回傳 `Ok(None)`。
    pub fn load(&self) -> Result<Option<ProjectTree>, ProjectTreeStoreError> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                let tree = serde_json::from_str(&contents)
                    .map_err(|err| ProjectTreeStoreError::Invalid(err.to_string()))?;
                Ok(Some(tree))
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => Err(ProjectTreeStoreError::Io(err)),
        }
    }

    /// Saves the provided project tree atomically to disk.  
    /// 將傳入的專案樹以原子方式寫入磁碟。
    pub fn save(&self, tree: &ProjectTree) -> Result<(), ProjectTreeStoreError> {
        let payload = serde_json::to_vec_pretty(tree)
            .map_err(|err| ProjectTreeStoreError::Invalid(err.to_string()))?;
        write_atomic(&self.path, &payload).map_err(ProjectTreeStoreError::Io)
    }
}

/// Errors emitted by [`ProjectTreeStore`].  
/// [`ProjectTreeStore`] 可能拋出的錯誤。
#[derive(Debug, Error)]
pub enum ProjectTreeStoreError {
    #[error("project tree IO error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid project tree payload: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{ProjectNodeDraft, ProjectNodeKind};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let store = ProjectTreeStore::new(dir.path().join("tree.json"));

        let tree = ProjectTree::empty("root", None);
        let root_id = tree.root_id();
        let draft = ProjectNodeDraft::new(
            "docs",
            ProjectNodeKind::Folder {
                path: None,
                filters: Vec::new(),
            },
        );
        let (tree, diff) = tree.add_child(root_id, draft).unwrap();
        let folder_id = diff.added[0];
        let draft_file = ProjectNodeDraft::new(
            "notes.md",
            ProjectNodeKind::File {
                path: PathBuf::from("docs/notes.md"),
            },
        );
        let (tree, _) = tree.add_child(folder_id, draft_file).unwrap();

        store.save(&tree).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.revision, tree.revision);
        assert_eq!(loaded.root.children.len(), 1);
        let folder = &loaded.root.children[0];
        assert!(matches!(folder.kind, ProjectNodeKind::Folder { .. }));
        assert_eq!(folder.children.len(), 1);
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = tempdir().unwrap();
        let store = ProjectTreeStore::new(dir.path().join("absent.json"));
        assert!(store.load().unwrap().is_none());
    }
}
