use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use thiserror::Error;

use crate::{Document, DocumentError, Encoding, LegacyEncoding, LineEnding};

/// 自動儲存與還原流程的錯誤型別。 / Error type for autosave and recovery routines.
#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("document error: {0}")]
    Document(#[from] DocumentError),
    #[error("invalid recovery metadata: {0}")]
    InvalidMetadata(String),
}

/// 管理暫存的恢復檔。 / Manages on-disk recovery snapshots.
#[derive(Debug, Clone)]
pub struct RecoveryManager {
    root: PathBuf,
}

/// 描述一個可供還原的快照。 / Describes a recoverable snapshot entry.
#[derive(Debug, Clone)]
pub struct RecoveryEntry {
    pub original_path: Option<PathBuf>,
    pub snapshot_path: PathBuf,
    pub encoding: Encoding,
    pub line_ending: LineEnding,
    pub has_bom: bool,
    pub timestamp: SystemTime,
}

impl RecoveryManager {
    /// 建立新的恢復管理員並指定儲存資料夾。 / Creates a new manager rooted at the provided directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// 將當前文件建立恢復快照。 / Persists a recovery snapshot for the given document.
    pub fn snapshot(&self, doc: &Document) -> Result<RecoveryEntry, RecoveryError> {
        self.ensure_root()?;

        let timestamp = SystemTime::now();
        let timestamp_ms = duration_since_epoch_ms(timestamp);
        let identifier = snapshot_identifier(doc, timestamp_ms);

        let data_path = self.root.join(format!("{}.autosave", identifier));
        let meta_path = self.root.join(format!("{}.meta", identifier));

        let payload = doc.serialise_contents()?;
        write_atomic(&data_path, &payload)?;

        let metadata = compose_metadata(doc, timestamp_ms);
        write_atomic(&meta_path, metadata.as_bytes())?;

        Ok(RecoveryEntry {
            original_path: doc.path().map(|p| p.to_path_buf()),
            snapshot_path: data_path,
            encoding: doc.encoding(),
            line_ending: doc.line_ending(),
            has_bom: doc.has_bom(),
            timestamp,
        })
    }

    /// 列出目前所有快照，依時間新到舊排序。 / Lists available snapshots sorted by newest first.
    pub fn list(&self) -> Result<Vec<RecoveryEntry>, RecoveryError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("meta") {
                continue;
            }

            let data_path = path.with_extension("autosave");
            if !data_path.is_file() {
                continue;
            }

            let contents = fs::read_to_string(&path)?;
            let metadata = parse_metadata(&contents)?;
            let timestamp = UNIX_EPOCH
                .checked_add(Duration::from_millis(metadata.timestamp_ms))
                .unwrap_or(UNIX_EPOCH);

            entries.push(RecoveryEntry {
                original_path: metadata.original_path,
                snapshot_path: data_path,
                encoding: metadata.encoding,
                line_ending: metadata.line_ending,
                has_bom: metadata.has_bom,
                timestamp,
            });
        }

        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(entries)
    }

    /// 移除指定快照。 / Removes the provided snapshot entry.
    pub fn remove(&self, entry: &RecoveryEntry) -> Result<(), RecoveryError> {
        if entry.snapshot_path.exists() {
            let _ = fs::remove_file(&entry.snapshot_path);
        }
        let meta_path = entry.snapshot_path.with_extension("meta");
        if meta_path.exists() {
            let _ = fs::remove_file(meta_path);
        }
        Ok(())
    }

    /// 載入快照為文件實例。 / Loads a snapshot into a `Document` instance.
    pub fn load(&self, entry: &RecoveryEntry) -> Result<Document, RecoveryError> {
        let mut doc = Document::open(&entry.snapshot_path)?;
        doc.set_path(entry.original_path.clone());
        doc.set_encoding(entry.encoding);
        doc.set_line_ending(entry.line_ending);
        doc.set_bom(entry.has_bom);
        doc.mark_dirty();
        Ok(doc)
    }

    fn ensure_root(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)
    }
}

struct ParsedMetadata {
    original_path: Option<PathBuf>,
    encoding: Encoding,
    line_ending: LineEnding,
    has_bom: bool,
    timestamp_ms: u64,
}

fn compose_metadata(doc: &Document, timestamp_ms: u64) -> String {
    let original_path = doc
        .path()
        .map(|p| BASE64.encode(p.to_string_lossy().as_bytes()))
        .unwrap_or_default();
    format!(
        "original_path={}\nencoding={}\nline_ending={}\nhas_bom={}\ntimestamp={}\n",
        original_path,
        doc.encoding().name(),
        line_ending_token(doc.line_ending()),
        doc.has_bom(),
        timestamp_ms
    )
}

fn parse_metadata(contents: &str) -> Result<ParsedMetadata, RecoveryError> {
    let mut original_path = None;
    let mut encoding = None;
    let mut line_ending = None;
    let mut has_bom = None;
    let mut timestamp_ms = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (key, value) = trimmed
            .split_once('=')
            .ok_or_else(|| RecoveryError::InvalidMetadata(format!("malformed line: {trimmed}")))?;
        match key {
            "original_path" => {
                if !value.is_empty() {
                    let decoded = BASE64.decode(value.as_bytes()).map_err(|_| {
                        RecoveryError::InvalidMetadata("failed to decode original_path".into())
                    })?;
                    let string = String::from_utf8(decoded).map_err(|_| {
                        RecoveryError::InvalidMetadata("original_path is not valid UTF-8".into())
                    })?;
                    original_path = Some(PathBuf::from(string));
                }
            }
            "encoding" => {
                encoding = Some(parse_encoding(value).ok_or_else(|| {
                    RecoveryError::InvalidMetadata(format!("unknown encoding: {value}"))
                })?);
            }
            "line_ending" => {
                line_ending = Some(parse_line_ending(value).ok_or_else(|| {
                    RecoveryError::InvalidMetadata(format!("unknown line ending: {value}"))
                })?);
            }
            "has_bom" => {
                has_bom = Some(value.parse::<bool>().map_err(|_| {
                    RecoveryError::InvalidMetadata("has_bom must be true/false".into())
                })?);
            }
            "timestamp" => {
                timestamp_ms =
                    Some(value.parse::<u64>().map_err(|_| {
                        RecoveryError::InvalidMetadata("timestamp must be u64".into())
                    })?);
            }
            other => {
                return Err(RecoveryError::InvalidMetadata(format!(
                    "unexpected metadata key: {other}"
                )));
            }
        }
    }

    Ok(ParsedMetadata {
        original_path,
        encoding: encoding
            .ok_or_else(|| RecoveryError::InvalidMetadata("missing encoding field".into()))?,
        line_ending: line_ending
            .ok_or_else(|| RecoveryError::InvalidMetadata("missing line_ending field".into()))?,
        has_bom: has_bom
            .ok_or_else(|| RecoveryError::InvalidMetadata("missing has_bom field".into()))?,
        timestamp_ms: timestamp_ms
            .ok_or_else(|| RecoveryError::InvalidMetadata("missing timestamp field".into()))?,
    })
}

fn parse_encoding(value: &str) -> Option<Encoding> {
    match value {
        "utf-8" => Some(Encoding::Utf8),
        "utf-16le" => Some(Encoding::Utf16Le),
        "utf-16be" => Some(Encoding::Utf16Be),
        "windows-1252" => Some(Encoding::Legacy(LegacyEncoding::Windows1252)),
        "shift-jis" => Some(Encoding::Legacy(LegacyEncoding::ShiftJis)),
        "gbk" => Some(Encoding::Legacy(LegacyEncoding::Gbk)),
        "big5" => Some(Encoding::Legacy(LegacyEncoding::Big5)),
        _ => None,
    }
}

fn parse_line_ending(value: &str) -> Option<LineEnding> {
    match value {
        "lf" => Some(LineEnding::Lf),
        "crlf" => Some(LineEnding::CrLf),
        "cr" => Some(LineEnding::Cr),
        _ => None,
    }
}

fn line_ending_token(ending: LineEnding) -> &'static str {
    match ending {
        LineEnding::Lf => "lf",
        LineEnding::CrLf => "crlf",
        LineEnding::Cr => "cr",
    }
}

fn snapshot_identifier(doc: &Document, timestamp_ms: u64) -> String {
    if let Some(path) = doc.path() {
        let mut hasher = DefaultHasher::new();
        path.to_string_lossy().hash(&mut hasher);
        format!("saved-{:016x}-{}", hasher.finish(), timestamp_ms)
    } else {
        format!("untitled-{}", timestamp_ms)
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)
}

fn duration_since_epoch_ms(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;
    use tempfile::tempdir;

    #[test]
    fn snapshot_and_restore_round_trip() {
        let dir = tempdir().unwrap();
        let manager = RecoveryManager::new(dir.path().join("recovery"));

        let mut doc = Document::new();
        doc.set_contents("line1\nline2\n");
        doc.set_line_ending(LineEnding::CrLf);
        doc.set_bom(true);
        let original_path = dir.path().join("note.txt");
        doc.set_path(Some(original_path.clone()));

        let entry = manager.snapshot(&doc).unwrap();
        assert_eq!(entry.original_path.as_ref(), Some(&original_path));
        assert_eq!(entry.line_ending, LineEnding::CrLf);
        assert!(entry.has_bom);

        let listed = manager.list().unwrap();
        assert_eq!(listed.len(), 1);

        let restored = manager.load(&listed[0]).unwrap();
        assert_eq!(restored.contents(), "line1\nline2\n");
        assert_eq!(restored.line_ending(), LineEnding::CrLf);
        assert!(restored.has_bom());
        assert!(restored.is_dirty());
        assert_eq!(
            restored.path().map(|p| p.to_path_buf()),
            Some(original_path.clone())
        );
    }

    #[test]
    fn snapshot_tracks_untitled_document() {
        let dir = tempdir().unwrap();
        let manager = RecoveryManager::new(dir.path().join("recovery"));

        let mut doc = Document::new();
        doc.set_contents("scratch");

        let entry = manager.snapshot(&doc).unwrap();
        assert!(entry.original_path.is_none());

        let listed = manager.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].original_path.is_none());

        manager.remove(&listed[0]).unwrap();
        assert!(manager.list().unwrap().is_empty());
    }
}
