use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::{FileAssociation, FileAssociations, RecentFiles};

/// 管理最近檔案清單的持久化儲存。 / Provides persistence for the recent-files history.
#[derive(Debug)]
pub struct RecentFilesStore {
    path: PathBuf,
    history: RecentFiles,
}

impl RecentFilesStore {
    /// 從指定路徑載入最近檔案清單；若檔案不存在則回傳空集合。 / Loads history from disk, returning an empty set when missing.
    pub fn load(path: impl AsRef<Path>, default_capacity: usize) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Ok(Self {
                path,
                history: RecentFiles::new(default_capacity),
            });
        }

        let contents = fs::read_to_string(&path)?;
        let mut lines = contents.lines();
        let mut capacity = default_capacity.max(1);
        if let Some(first_line) = lines.next() {
            if let Some(value) = first_line.trim().strip_prefix("capacity=") {
                if let Ok(parsed) = value.parse::<usize>() {
                    capacity = parsed.max(1);
                }
            }
        }

        let mut entries = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            entries.push(decode_path(trimmed)?);
        }

        Ok(Self {
            path,
            history: RecentFiles::with_entries(capacity, entries),
        })
    }

    /// 取得內部的最近檔案清單。 / Returns the underlying history.
    pub fn history(&self) -> &RecentFiles {
        &self.history
    }

    /// 目前追蹤的檔案條目列舉。 / Iterator over recorded entries.
    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.history.iter()
    }

    /// 新增或提升項目並立即寫回檔案。 / Adds or promotes an entry and persists it.
    pub fn add(&mut self, path: impl Into<PathBuf>) -> io::Result<()> {
        self.history.add(path);
        self.persist()
    }

    /// 移除項目並同步儲存。 / Removes an entry and persists the change.
    pub fn remove(&mut self, path: &Path) -> io::Result<bool> {
        let removed = self.history.remove(path);
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    /// 清空紀錄並同步儲存。 / Clears the history and persists immediately.
    pub fn clear(&mut self) -> io::Result<()> {
        self.history.clear();
        self.persist()
    }

    /// 調整容量並同步儲存。 / Updates the capacity and persists the state.
    pub fn set_capacity(&mut self, capacity: usize) -> io::Result<()> {
        self.history.set_capacity(capacity);
        self.persist()
    }

    fn persist(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut payload = format!("capacity={}\n", self.history.capacity().max(1));
        for entry in self.history.iter() {
            payload.push_str(&encode_path(entry));
            payload.push('\n');
        }
        write_atomic(&self.path, payload.as_bytes())
    }
}

/// 檔案關聯的持久化管理。 / Persists file-association mappings.
#[derive(Debug)]
pub struct FileAssociationsStore {
    path: PathBuf,
    associations: FileAssociations,
}

impl FileAssociationsStore {
    /// 從檔案載入關聯設定；若檔案不存在則建立空集合。 / Loads associations from disk, returning an empty set when missing.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Ok(Self {
                path,
                associations: FileAssociations::new(),
            });
        }

        let contents = fs::read_to_string(&path)?;
        let mut associations = FileAssociations::new();
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (ext, encoded) = trimmed.split_once('=').ok_or_else(|| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("malformed association entry: {trimmed}"),
                )
            })?;
            let command_bytes = BASE64
                .decode(encoded.as_bytes())
                .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
            let command = String::from_utf8(command_bytes)
                .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
            associations.set(ext, command);
        }

        Ok(Self { path, associations })
    }

    /// 讀取目前的關聯表。 / Returns the current associations.
    pub fn associations(&self) -> &FileAssociations {
        &self.associations
    }

    /// 設定或更新檔案關聯並立即寫回。 / Inserts or updates an association and persists.
    pub fn set(
        &mut self,
        extension: impl AsRef<str>,
        command: impl Into<String>,
    ) -> io::Result<()> {
        self.associations.set(extension.as_ref(), command);
        self.persist()
    }

    /// 移除關聯，若存在則同步儲存。 / Removes an association and persists when changed.
    pub fn remove(&mut self, extension: impl AsRef<str>) -> io::Result<bool> {
        let removed = self.associations.remove(extension.as_ref());
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    /// 清空全部關聯並立即儲存。 / Clears all associations and persists.
    pub fn clear(&mut self) -> io::Result<()> {
        self.associations.clear();
        self.persist()
    }

    fn persist(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut entries: Vec<FileAssociation> = self.associations.iter().collect();
        entries.sort_by(|a, b| a.extension.cmp(&b.extension));

        let mut payload = String::new();
        for entry in entries {
            let encoded = BASE64.encode(entry.command.as_bytes());
            payload.push_str(&format!("{}={}\n", entry.extension, encoded));
        }
        write_atomic(&self.path, payload.as_bytes())
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

fn encode_path(path: &Path) -> String {
    BASE64.encode(path_to_bytes(path))
}

fn decode_path(encoded: &str) -> io::Result<PathBuf> {
    let bytes = BASE64
        .decode(encoded.as_bytes())
        .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
    bytes_to_path(bytes)
}

#[cfg(unix)]
fn path_to_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(unix)]
fn bytes_to_path(bytes: Vec<u8>) -> io::Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

#[cfg(windows)]
fn path_to_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .flat_map(|unit| unit.to_le_bytes())
        .collect()
}

#[cfg(windows)]
fn bytes_to_path(bytes: Vec<u8>) -> io::Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    if bytes.len() % 2 != 0 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "encoded path payload has an odd length",
        ));
    }

    let wide: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    Ok(PathBuf::from(OsString::from_wide(&wide)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn recent_files_store_persists_entries() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("recent.db");

        {
            let mut store = RecentFilesStore::load(&store_path, 5).unwrap();
            assert_eq!(store.history().len(), 0);
            store.add(dir.path().join("alpha.txt")).unwrap();
            store.add(dir.path().join("beta.txt")).unwrap();
            store.set_capacity(1).unwrap();
        }

        let store = RecentFilesStore::load(&store_path, 3).unwrap();
        let collected: Vec<_> = store
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(collected, vec!["beta.txt"]);
        assert_eq!(store.history().capacity(), 1);
    }

    #[test]
    fn file_associations_store_round_trips() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("assoc.db");

        {
            let mut store = FileAssociationsStore::load(&store_path).unwrap();
            store.set("rs", "rustc").unwrap();
            store.set("txt", "less").unwrap();
        }

        let mut store = FileAssociationsStore::load(&store_path).unwrap();
        assert_eq!(store.associations().get("rs"), Some("rustc"));
        assert_eq!(store.associations().get("txt"), Some("less"));
        assert!(store.remove("rs").unwrap());
        assert!(!store.remove("rs").unwrap());
        store.clear().unwrap();

        let store = FileAssociationsStore::load(&store_path).unwrap();
        assert!(store.associations().is_empty());
    }
}
