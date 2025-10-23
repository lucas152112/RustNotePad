use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::util::write_atomic;

/// Current session format version.
pub const SESSION_FORMAT_VERSION: u32 = 1;

/// Represents the entire application session snapshot.  
/// 描述整體應用程式的工作階段資訊。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub format_version: u32,
    #[serde(default)]
    pub metadata: SessionMetadata,
    #[serde(default)]
    pub windows: Vec<SessionWindow>,
}

impl SessionSnapshot {
    /// Creates a new snapshot with the current format version.  
    /// 建立採用最新格式版本的快照。
    pub fn new(windows: Vec<SessionWindow>) -> Self {
        Self {
            format_version: SESSION_FORMAT_VERSION,
            metadata: SessionMetadata::default(),
            windows,
        }
    }

    /// Returns `true` when there are no tracked windows/tabs.  
    /// 若沒有任何視窗或分頁則回傳 `true`。
    pub fn is_empty(&self) -> bool {
        self.windows.iter().all(|window| window.tabs.is_empty())
    }
}

/// Session-level metadata for diagnostics and compatibility checks.  
/// 工作階段元資料，用於除錯與相容性檢測。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    #[serde(default)]
    pub created_at_unix: Option<i64>,
    #[serde(default)]
    pub application_version: Option<String>,
    #[serde(default)]
    pub issues: Vec<CompatibilityIssue>,
}

/// Records an incompatibility encountered during load/restore.  
/// 紀錄載入/還原時遇到的相容性問題。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityIssue {
    pub message: String,
    #[serde(
        default,
        with = "crate::serde_path::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub related_path: Option<PathBuf>,
}

impl CompatibilityIssue {
    pub fn new(message: impl Into<String>, related_path: Option<PathBuf>) -> Self {
        Self {
            message: message.into(),
            related_path,
        }
    }
}

/// Represents an application window containing tabs.  
/// 描述一個應用視窗及其分頁集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWindow {
    #[serde(default)]
    pub tabs: Vec<SessionTab>,
    #[serde(default)]
    pub pane_layout: Option<String>,
    #[serde(default)]
    pub active_tab: Option<usize>,
}

impl SessionWindow {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            pane_layout: None,
            active_tab: None,
        }
    }
}

/// State captured for a single editor tab.  
/// 單一編輯分頁的狀態快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTab {
    #[serde(
        default,
        with = "crate::serde_path::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub caret: SessionCaret,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<SessionSelection>,
    #[serde(default)]
    pub scroll: SessionScroll,
    #[serde(default)]
    pub folds: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsaved_hash: Option<UnsavedHash>,
    #[serde(default)]
    pub dirty_external: bool,
}

impl Default for SessionTab {
    fn default() -> Self {
        Self {
            path: None,
            display_name: None,
            encoding: None,
            caret: SessionCaret::default(),
            selection: None,
            scroll: SessionScroll::default(),
            folds: Vec::new(),
            unsaved_hash: None,
            dirty_external: false,
        }
    }
}

/// Caret position persisted in the session file.  
/// 儲存在工作階段檔案中的游標位置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionCaret {
    pub line: u32,
    pub column: u32,
}

/// Selection bounds within a document.  
/// 文件中的選取範圍。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSelection {
    pub anchor: SessionCaret,
    pub head: SessionCaret,
}

/// Scroll offset (top line and horizontal column).  
/// 捲動偏移量（頂端行與水平欄位）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionScroll {
    pub top_line: u32,
    pub horizontal_offset: u32,
}

/// Hash representing the autosaved buffer contents.  
/// 代表自動儲存緩衝內容的雜湊值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct UnsavedHash(String);

impl UnsavedHash {
    pub fn new(encoded: impl Into<String>) -> Self {
        Self(encoded.into())
    }

    /// Computes an `UnsavedHash` from raw bytes using XXH3-64.  
    /// 使用標準 `DefaultHasher` 雜湊原始位元資料。
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        Self(format!("{hash:016x}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Error type for session persistence.  
/// 工作階段持久化時可能出現的錯誤。
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session file IO error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid session payload: {0}")]
    InvalidPayload(#[from] serde_json::Error),
}

/// Manages session snapshot and autosave content on disk.  
/// 管理工作階段快照與自動儲存內容的儲存。
#[derive(Debug)]
pub struct SessionStore {
    session_path: PathBuf,
    autosave: AutosaveStore,
}

impl SessionStore {
    pub fn new(session_path: impl AsRef<Path>, autosave_dir: impl AsRef<Path>) -> Self {
        Self {
            session_path: session_path.as_ref().to_path_buf(),
            autosave: AutosaveStore::new(autosave_dir),
        }
    }

    /// Returns reference to the autosave store.  
    /// 取得自動儲存存放區的參考。
    pub fn autosave(&self) -> &AutosaveStore {
        &self.autosave
    }

    /// Loads the session snapshot from disk. Missing files return `Ok(None)`.  
    /// 從磁碟載入工作階段快照；若檔案不存在則回傳 `Ok(None)`。
    pub fn load(&self) -> Result<Option<SessionSnapshot>, SessionError> {
        match fs::read_to_string(&self.session_path) {
            Ok(contents) => {
                let snapshot: SessionSnapshot = serde_json::from_str(&contents)?;
                Ok(Some(snapshot))
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => Err(SessionError::Io(err)),
        }
    }

    /// Persists the session snapshot using atomic writes.  
    /// 以原子寫入方式儲存工作階段快照。
    pub fn save(&self, snapshot: &SessionSnapshot) -> Result<(), SessionError> {
        let mut payload = snapshot.clone();
        if payload.metadata.created_at_unix.is_none() {
            payload.metadata.created_at_unix = Some(current_timestamp());
        }
        let json = serde_json::to_vec_pretty(&payload)?;
        write_atomic(&self.session_path, &json)?;
        Ok(())
    }
}

/// Stores autosave buffers alongside a manifest for pruning.  
/// 管理 autosave 緩衝及其清單。
#[derive(Debug)]
pub struct AutosaveStore {
    root: PathBuf,
    manifest_path: PathBuf,
}

impl AutosaveStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join("manifest.json");
        Self {
            root,
            manifest_path,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Loads the manifest, returning an empty one when missing.  
    /// 載入 autosave 清單；若不存在則提供空白結構。
    pub fn load_manifest(&self) -> Result<AutosaveManifest, SessionError> {
        match fs::read_to_string(&self.manifest_path) {
            Ok(contents) => {
                let manifest = serde_json::from_str(&contents)?;
                Ok(manifest)
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(AutosaveManifest::default()),
            Err(err) => Err(SessionError::Io(err)),
        }
    }

    /// Persists the manifest to disk.  
    /// 將清單寫回磁碟。
    pub fn save_manifest(&self, manifest: &AutosaveManifest) -> Result<(), SessionError> {
        let json = serde_json::to_vec_pretty(manifest)?;
        write_atomic(&self.manifest_path, &json)?;
        Ok(())
    }

    /// Writes autosave contents to disk using the hash as filename.  
    /// 以雜湊值作為檔名將 autosave 內容寫入磁碟。
    pub fn write_contents(&self, hash: &UnsavedHash, data: &[u8]) -> Result<PathBuf, SessionError> {
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{}.rna", hash.as_str()));
        write_atomic(&path, data)?;
        Ok(path)
    }

    /// Reads autosave contents back into memory.  
    /// 從磁碟讀取 autosave 內容。
    pub fn read_contents(&self, hash: &UnsavedHash) -> Result<Vec<u8>, SessionError> {
        let path = self.root.join(format!("{}.rna", hash.as_str()));
        match fs::read(&path) {
            Ok(bytes) => Ok(bytes),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(SessionError::Io(err)),
        }
    }

    /// Removes an autosave payload.  
    /// 移除指定的 autosave 檔案。
    pub fn remove(&self, hash: &UnsavedHash) -> Result<(), SessionError> {
        let path = self.root.join(format!("{}.rna", hash.as_str()));
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(SessionError::Io(err)),
        }
    }
}

/// Keeps track of autosave entries for pruning/diagnostics.  
/// 記錄 autosave 項目，供後續清理與除錯使用。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutosaveManifest {
    #[serde(default)]
    pub entries: BTreeMap<String, AutosaveEntry>,
}

impl AutosaveManifest {
    pub fn touch(&mut self, hash: UnsavedHash) {
        let timestamp = current_timestamp();
        self.entries.insert(
            hash.as_str().to_string(),
            AutosaveEntry {
                hash,
                updated_at_unix: timestamp,
            },
        );
    }

    pub fn remove(&mut self, hash: &UnsavedHash) {
        self.entries.remove(hash.as_str());
    }
}

/// Describes a single autosave entry and the last update timestamp.  
/// 描述單一 autosave 項目與最後更新時間。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosaveEntry {
    pub hash: UnsavedHash,
    pub updated_at_unix: i64,
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn snapshot_round_trips() {
        let tmp = tempdir().unwrap();
        let session_path = tmp.path().join("session.json");
        let autosave_dir = tmp.path().join("autosave");
        let store = SessionStore::new(&session_path, &autosave_dir);

        let mut window = SessionWindow::new();
        window.tabs.push(SessionTab {
            path: Some(tmp.path().join("alpha.txt")),
            display_name: Some("alpha.txt".into()),
            encoding: Some("utf-8".into()),
            caret: SessionCaret {
                line: 12,
                column: 4,
            },
            selection: Some(SessionSelection {
                anchor: SessionCaret {
                    line: 12,
                    column: 0,
                },
                head: SessionCaret {
                    line: 14,
                    column: 7,
                },
            }),
            scroll: SessionScroll {
                top_line: 8,
                horizontal_offset: 16,
            },
            folds: vec![5, 20],
            unsaved_hash: Some(UnsavedHash::new("deadbeef")),
            dirty_external: true,
        });

        let snapshot = SessionSnapshot::new(vec![window]);
        store.save(&snapshot).unwrap();

        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.format_version, SESSION_FORMAT_VERSION);
        assert_eq!(loaded.windows.len(), 1);
        let tab = &loaded.windows[0].tabs[0];
        assert_eq!(tab.caret.line, 12);
        assert_eq!(tab.scroll.horizontal_offset, 16);
        assert_eq!(tab.dirty_external, true);
    }

    #[test]
    fn autosave_manifest_tracks_entries() {
        let mut manifest = AutosaveManifest::default();
        let hash = UnsavedHash::new("hash-one");
        manifest.touch(hash.clone());
        assert!(manifest.entries.contains_key(hash.as_str()));
        manifest.remove(&hash);
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn autosave_store_persists_payloads() {
        let tmp = tempdir().unwrap();
        let store = AutosaveStore::new(tmp.path());
        let hash = UnsavedHash::from_bytes(b"hello world");

        let path = store.write_contents(&hash, b"payload").unwrap();
        assert!(path.exists());
        let read_back = store.read_contents(&hash).unwrap();
        assert_eq!(read_back, b"payload");
        store.remove(&hash).unwrap();
        let missing = store.read_contents(&hash).unwrap();
        assert!(missing.is_empty());
    }
}
