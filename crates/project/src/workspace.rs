use std::collections::{HashMap, VecDeque};
use std::fs;
use std::hash::Hash;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::serde_path;
use crate::util::write_atomic;

static NEXT_WORKSPACE_ID: AtomicU64 = AtomicU64::new(1);

/// Stable identifier for workspaces.  
/// 工作區的穩定代號。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new() -> Self {
        let id = NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed);
        Self(format!("{id:016x}"))
    }

    pub fn from_string(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Binding between a workspace and a project definition file.  
/// 工作區指向的專案定義檔。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectBinding {
    #[serde(with = "serde_path")]
    pub path: PathBuf,
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Default search scope preferences recorded at workspace level.  
/// 工作區層級的預設搜尋偏好。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SearchDefaults {
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub follow_gitignore: bool,
    #[serde(default)]
    pub pattern_history: Vec<String>,
}

/// Complete workspace descriptor.  
/// 完整的工作區描述。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceDescriptor {
    pub id: WorkspaceId,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub projects: Vec<ProjectBinding>,
    #[serde(default)]
    pub search_defaults: SearchDefaults,
    #[serde(default)]
    pub last_used_unix: Option<i64>,
}

impl WorkspaceDescriptor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: WorkspaceId::new(),
            name: name.into(),
            description: None,
            projects: Vec::new(),
            search_defaults: SearchDefaults::default(),
            last_used_unix: None,
        }
    }
}

/// Entry stored inside the workspace index file.  
/// 儲存在索引檔案中的工作區條目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceIndexEntry {
    pub id: WorkspaceId,
    pub name: String,
    #[serde(default)]
    pub last_used_unix: Option<i64>,
    #[serde(with = "serde_path")]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WorkspaceIndexFile {
    #[serde(default)]
    entries: Vec<WorkspaceIndexEntry>,
}

/// Errors raised by workspace persistence.  
/// 工作區儲存相關的錯誤。
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace IO error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid workspace descriptor: {0}")]
    InvalidDescriptor(String),
    #[error("invalid workspace index: {0}")]
    InvalidIndex(String),
    #[error("workspace {0} not found")]
    NotFound(String),
}

/// Manages workspace metadata and descriptor persistence.  
/// 管理工作區索引與定義檔案的存取。
#[derive(Debug)]
pub struct WorkspaceStore {
    root: PathBuf,
    index_path: PathBuf,
}

impl WorkspaceStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let index_path = root.join("workspaces.json");
        Self { root, index_path }
    }

    fn ensure_root(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)
    }

    fn workspace_path(&self, id: &WorkspaceId) -> PathBuf {
        self.root.join(format!("workspace_{}.json", id.as_str()))
    }

    /// Lists all workspace index entries.  
    /// 列出所有工作區索引條目。
    pub fn list(&self) -> Result<Vec<WorkspaceIndexEntry>, WorkspaceError> {
        match fs::read_to_string(&self.index_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map(|file: WorkspaceIndexFile| file.entries)
                .map_err(|err| WorkspaceError::InvalidIndex(err.to_string())),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(WorkspaceError::Io(err)),
        }
    }

    /// Loads a workspace descriptor by identifier.  
    /// 依識別碼載入工作區描述。
    pub fn load(&self, id: &WorkspaceId) -> Result<WorkspaceDescriptor, WorkspaceError> {
        let path = self.workspace_path(id);
        match fs::read_to_string(path) {
            Ok(contents) => {
                let mut descriptor: WorkspaceDescriptor = serde_json::from_str(&contents)
                    .map_err(|err| WorkspaceError::InvalidDescriptor(err.to_string()))?;
                descriptor.last_used_unix = Some(current_timestamp());
                self.update_index_entry(&descriptor)?;
                Ok(descriptor)
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                Err(WorkspaceError::NotFound(id.as_str().to_string()))
            }
            Err(err) => Err(WorkspaceError::Io(err)),
        }
    }

    /// Saves (or updates) the descriptor, recording it in the index.  
    /// 儲存/更新工作區描述並同步到索引。
    pub fn save(&self, descriptor: &WorkspaceDescriptor) -> Result<(), WorkspaceError> {
        self.ensure_root()?;
        let path = self.workspace_path(&descriptor.id);
        let mut copy = descriptor.clone();
        copy.last_used_unix = Some(current_timestamp());
        let json = serde_json::to_vec_pretty(&copy)
            .map_err(|err| WorkspaceError::InvalidDescriptor(err.to_string()))?;
        write_atomic(&path, &json)?;
        self.update_index_entry(&copy)?;
        Ok(())
    }

    /// Updates the last-used timestamp for the given workspace.  
    /// 更新指定工作區的最近使用時間。
    pub fn touch(&self, id: &WorkspaceId) -> Result<(), WorkspaceError> {
        let mut index = self.load_index_file()?;
        if let Some(entry) = index.entries.iter_mut().find(|entry| entry.id == *id) {
            entry.last_used_unix = Some(current_timestamp());
            self.save_index_file(&index)?;
            Ok(())
        } else {
            Err(WorkspaceError::NotFound(id.as_str().to_string()))
        }
    }

    fn update_index_entry(&self, descriptor: &WorkspaceDescriptor) -> Result<(), WorkspaceError> {
        let mut index = self.load_index_file()?;
        let entry_path = self.workspace_path(&descriptor.id);
        if let Some(entry) = index
            .entries
            .iter_mut()
            .find(|entry| entry.id == descriptor.id)
        {
            entry.name = descriptor.name.clone();
            entry.last_used_unix = descriptor.last_used_unix;
            entry.path = entry_path;
        } else {
            index.entries.push(WorkspaceIndexEntry {
                id: descriptor.id.clone(),
                name: descriptor.name.clone(),
                last_used_unix: descriptor.last_used_unix,
                path: entry_path,
            });
        }
        self.save_index_file(&index)?;
        Ok(())
    }

    fn load_index_file(&self) -> Result<WorkspaceIndexFile, WorkspaceError> {
        match fs::read_to_string(&self.index_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|err| WorkspaceError::InvalidIndex(err.to_string())),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(WorkspaceIndexFile::default()),
            Err(err) => Err(WorkspaceError::Io(err)),
        }
    }

    fn save_index_file(&self, file: &WorkspaceIndexFile) -> Result<(), WorkspaceError> {
        self.ensure_root()?;
        let json = serde_json::to_vec_pretty(file)
            .map_err(|err| WorkspaceError::InvalidIndex(err.to_string()))?;
        write_atomic(&self.index_path, &json)?;
        Ok(())
    }
}

/// Simple LRU cache for workspace descriptors.  
/// 工作區描述的簡易 LRU 快取。
#[derive(Debug)]
pub struct WorkspaceCache {
    capacity: usize,
    order: VecDeque<WorkspaceId>,
    entries: HashMap<WorkspaceId, WorkspaceDescriptor>,
}

impl WorkspaceCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    /// Retrieves a descriptor from cache, updating its LRU position.  
    /// 從快取取得描述並更新 LRU 順序。
    pub fn get(&mut self, id: &WorkspaceId) -> Option<&WorkspaceDescriptor> {
        if self.entries.contains_key(id) {
            self.promote(id);
        }
        self.entries.get(id)
    }

    /// Inserts or updates a descriptor in the cache.  
    /// 將描述加入/更新至快取。
    pub fn insert(&mut self, descriptor: WorkspaceDescriptor) {
        let id = descriptor.id.clone();
        self.entries.insert(id.clone(), descriptor);
        self.promote(&id);
        self.evict_if_needed();
    }

    fn promote(&mut self, id: &WorkspaceId) {
        self.order.retain(|existing| existing != id);
        self.order.push_front(id.clone());
    }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > self.capacity {
            if let Some(removed) = self.order.pop_back() {
                self.entries.remove(&removed);
            } else {
                break;
            }
        }
    }
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
    fn save_and_load_workspace_descriptor() {
        let tmp = tempdir().unwrap();
        let store = WorkspaceStore::new(tmp.path());

        let mut descriptor = WorkspaceDescriptor::new("Demo");
        descriptor.projects.push(ProjectBinding {
            path: tmp.path().join("project_a.rnp"),
            display_name: Some("Project A".into()),
        });
        store.save(&descriptor).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Demo");

        let loaded = store.load(&descriptor.id).unwrap();
        assert_eq!(loaded.name, "Demo");
        assert_eq!(loaded.projects.len(), 1);
    }

    #[test]
    fn workspace_cache_evicts_old_entries() {
        let mut cache = WorkspaceCache::new(2);
        let make_descriptor = |name: &str| {
            let mut descriptor = WorkspaceDescriptor::new(name);
            descriptor.id = WorkspaceId::from_string(name.to_string());
            descriptor
        };

        cache.insert(make_descriptor("A"));
        cache.insert(make_descriptor("B"));
        cache.insert(make_descriptor("C"));

        assert!(cache.get(&WorkspaceId::from_string("A")).is_none());
        assert!(cache.get(&WorkspaceId::from_string("B")).is_some());
        assert!(cache.get(&WorkspaceId::from_string("C")).is_some());
    }
}
