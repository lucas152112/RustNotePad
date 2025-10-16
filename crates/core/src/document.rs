use std::borrow::Cow;
use std::fs::{self, File, Metadata};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chardetng::EncodingDetector;
use encoding_rs::{Encoding as RsEncoding, BIG5, GBK, SHIFT_JIS, WINDOWS_1252};
use thiserror::Error;

/// 表示文件目前使用的行尾樣式。 / Represents the current line ending style for a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    /// 回傳序列化文字時使用的行尾字串。 / Returns the literal string representation used when serialising text.
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }
}

/// 列舉文件支援的文字編碼。 / Supported encodings for text documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Legacy(LegacyEncoding),
}

/// 反映磁碟上的檔案狀態，用以判定是否需要重新載入。 / Snapshot of the on-disk state for change detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskState {
    Unchanged,
    Modified,
    Removed,
}

/// 指定支援的傳統多位元編碼。 / Enumerates supported legacy multi-byte encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyEncoding {
    Windows1252,
    ShiftJis,
    Gbk,
    Big5,
}

impl LegacyEncoding {
    pub fn name(self) -> &'static str {
        match self {
            LegacyEncoding::Windows1252 => "windows-1252",
            LegacyEncoding::ShiftJis => "shift-jis",
            LegacyEncoding::Gbk => "gbk",
            LegacyEncoding::Big5 => "big5",
        }
    }

    fn to_rs(self) -> &'static RsEncoding {
        match self {
            LegacyEncoding::Windows1252 => WINDOWS_1252,
            LegacyEncoding::ShiftJis => SHIFT_JIS,
            LegacyEncoding::Gbk => GBK,
            LegacyEncoding::Big5 => BIG5,
        }
    }
}

impl Encoding {
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::Utf16Le => "utf-16le",
            Encoding::Utf16Be => "utf-16be",
            Encoding::Legacy(legacy) => legacy.name(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSignature {
    len: u64,
    modified_nanos: Option<u128>,
}

impl FileSignature {
    fn from_metadata(metadata: &Metadata) -> Self {
        let modified_nanos = metadata.modified().ok().and_then(system_time_to_nanos);
        Self {
            len: metadata.len(),
            modified_nanos,
        }
    }
}

/// 文件載入或儲存時可能發生的錯誤。 / Errors that can occur while loading or saving a document.
#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("file encoding is not supported or data is invalid")]
    InvalidEncoding,
    #[error("text cannot be represented in target encoding {0}")]
    Unrepresentable(&'static str),
}

/// 代表以 Unicode 或特定舊式文字編碼為後盾的文件記憶體模型。 / In-memory representation of a text document backed by a Unicode or selected legacy-encoded text file.
#[derive(Debug, Clone)]
pub struct Document {
    path: Option<PathBuf>,
    contents: String,
    line_ending: LineEnding,
    encoding: Encoding,
    has_bom: bool,
    is_dirty: bool,
    on_disk_signature: Option<FileSignature>,
}

impl Document {
    /// 建立一個空內容且尚未儲存的文件。 / Creates an unsaved document with empty contents.
    pub fn new() -> Self {
        Self {
            path: None,
            contents: String::new(),
            line_ending: LineEnding::Lf,
            encoding: Encoding::Utf8,
            has_bom: false,
            is_dirty: false,
            on_disk_signature: None,
        }
    }

    /// 從磁碟載入文件並將行尾內部正規化為 `\n`。 / Loads a document from disk, normalising newlines to `\n` internally.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)?;
        let metadata = file.metadata()?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let decoded = decode_bytes(bytes)?;
        let line_ending = detect_line_ending(&decoded.text);
        let contents = normalize_newlines(&decoded.text);
        let signature = FileSignature::from_metadata(&metadata);

        Ok(Self {
            path: Some(path_ref.to_path_buf()),
            contents,
            line_ending,
            encoding: decoded.encoding,
            has_bom: decoded.has_bom,
            is_dirty: false,
            on_disk_signature: Some(signature),
        })
    }

    /// 將文件儲存至現有路徑；若尚未指定路徑則失敗。 / Saves the document to its current path; fails if no path is set.
    pub fn save(&mut self) -> Result<(), DocumentError> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "document has no associated path"))?
            .to_path_buf();
        self.save_as(path)
    }

    /// 將文件另存為新路徑並更新相關中繼資料。 / Saves the document to a new path, updating the associated metadata.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), DocumentError> {
        let path_ref = path.as_ref();
        let encoded = self.serialise_contents()?;

        // 先寫入暫存檔再重新命名，避免出現部分寫入的情況。 / Use a temporary file plus rename to guard against partial writes.
        let tmp_path = path_ref.with_extension("tmp_rustnotepad");
        {
            let mut tmp_file = File::create(&tmp_path)?;
            tmp_file.write_all(&encoded)?;
            tmp_file.sync_all()?; // 確保資料在重新命名前已寫入磁碟。 / Ensure bytes hit the disk before rename.
        }
        fs::rename(&tmp_path, path_ref)?;

        let metadata = fs::metadata(path_ref)?;
        self.path = Some(path_ref.to_path_buf());
        self.is_dirty = false;
        self.on_disk_signature = Some(FileSignature::from_metadata(&metadata));
        Ok(())
    }

    /// 取得目前文件內容（行尾已正規化為 `\n`）。 / Returns the current document contents, normalised to `\n` line endings.
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// 以新文字取代記憶體內容並標記文件為已修改。 / Replaces the in-memory contents, marking the document as dirty.
    pub fn set_contents(&mut self, text: impl Into<String>) {
        let text = normalize_newlines(&text.into());
        self.contents = text;
        self.is_dirty = true;
    }

    /// 取得目前行尾設定。 / Returns the current line ending preference.
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// 更新行尾設定。 / Updates the line ending preference.
    pub fn set_line_ending(&mut self, ending: LineEnding) {
        if self.line_ending != ending {
            self.line_ending = ending;
            self.is_dirty = true;
        }
    }

    /// 取得目前文件編碼。 / Returns the current document encoding.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// 更新文件編碼設定。 / Updates the document encoding preference.
    pub fn set_encoding(&mut self, encoding: Encoding) {
        if self.encoding != encoding {
            self.encoding = encoding;
            if matches!(self.encoding, Encoding::Legacy(_)) {
                self.has_bom = false;
            }
            self.is_dirty = true;
        }
    }

    /// 指出儲存時是否包含 UTF-8 BOM。 / Indicates whether the document includes a UTF-8 BOM when saved.
    pub fn has_bom(&self) -> bool {
        self.has_bom
    }

    /// 更新 BOM 標記，改變時會標記文件為已修改。 / Updates the BOM flag and marks the document dirty if it changes.
    pub fn set_bom(&mut self, has_bom: bool) {
        if self.has_bom != has_bom {
            self.has_bom = has_bom;
            self.is_dirty = true;
        }
    }

    /// 判斷文件是否仍有未儲存變更。 / Returns whether the document has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// 將文件標記為已修改。 / Marks the document as having unsaved changes.
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// 取得文件所屬的檔案路徑（若存在）。 / Retrieves the associated path if the document is linked to one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 更新檔案路徑中繼資料（不影響 dirty 狀態）。 / Updates the associated file path metadata without affecting dirty state.
    pub fn set_path(&mut self, path: Option<PathBuf>) {
        self.path = path;
        self.on_disk_signature = None;
    }

    /// 重新載入磁碟內容覆蓋記憶體，並重設 dirty 狀態。 / Reloads the document from disk, replacing the in-memory buffer.
    pub fn reload(&mut self) -> Result<(), DocumentError> {
        let Some(path) = self.path.clone() else {
            return Err(DocumentError::Io(io::Error::new(
                ErrorKind::Other,
                "document has no associated path",
            )));
        };
        let fresh = Document::open(&path)?;
        *self = fresh;
        Ok(())
    }

    /// 檢查磁碟上的檔案是否與快照不同。 / Checks whether the on-disk file differs from the stored snapshot.
    pub fn check_disk_state(&self) -> Result<DiskState, DocumentError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(DiskState::Unchanged);
        };

        match fs::metadata(path) {
            Ok(metadata) => {
                let signature = FileSignature::from_metadata(&metadata);
                if self
                    .on_disk_signature
                    .map_or(true, |stored| stored != signature)
                {
                    Ok(DiskState::Modified)
                } else {
                    Ok(DiskState::Unchanged)
                }
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(DiskState::Removed),
            Err(err) => Err(DocumentError::Io(err)),
        }
    }

    pub(crate) fn serialise_contents(&self) -> Result<Vec<u8>, DocumentError> {
        let text = self.contents.replace('\n', self.line_ending.as_str());
        let bytes = match self.encoding {
            Encoding::Utf8 => {
                if self.has_bom {
                    // 在輸出資料前加上 UTF-8 BOM。 / Prepend UTF-8 BOM bytes to the encoded payload.
                    let mut prefixed = Vec::with_capacity(3 + text.len());
                    prefixed.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
                    prefixed.extend_from_slice(text.as_bytes());
                    prefixed
                } else {
                    text.into_bytes()
                }
            }
            Encoding::Utf16Le => encode_utf16(&text, self.has_bom, false),
            Encoding::Utf16Be => encode_utf16(&text, self.has_bom, true),
            Encoding::Legacy(legacy) => encode_legacy(&text, legacy)?,
        };
        Ok(bytes)
    }
}

struct DecodedText {
    text: String,
    encoding: Encoding,
    has_bom: bool,
}

fn decode_bytes(bytes: Vec<u8>) -> Result<DecodedText, DocumentError> {
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        let text =
            String::from_utf8(bytes[3..].to_vec()).map_err(|_| DocumentError::InvalidEncoding)?;
        return Ok(DecodedText {
            text,
            encoding: Encoding::Utf8,
            has_bom: true,
        });
    }

    if bytes.starts_with(b"\xFF\xFE") {
        let text = decode_utf16(&bytes[2..], false)?;
        return Ok(DecodedText {
            text,
            encoding: Encoding::Utf16Le,
            has_bom: true,
        });
    }

    if bytes.starts_with(b"\xFE\xFF") {
        let text = decode_utf16(&bytes[2..], true)?;
        return Ok(DecodedText {
            text,
            encoding: Encoding::Utf16Be,
            has_bom: true,
        });
    }

    if looks_like_utf16_le(&bytes) {
        let text = decode_utf16(&bytes, false)?;
        return Ok(DecodedText {
            text,
            encoding: Encoding::Utf16Le,
            has_bom: false,
        });
    }

    if looks_like_utf16_be(&bytes) {
        let text = decode_utf16(&bytes, true)?;
        return Ok(DecodedText {
            text,
            encoding: Encoding::Utf16Be,
            has_bom: false,
        });
    }

    if let Ok(text) = std::str::from_utf8(&bytes) {
        return Ok(DecodedText {
            text: text.to_owned(),
            encoding: Encoding::Utf8,
            has_bom: false,
        });
    }

    if let Some(legacy) = detect_legacy_encoding(&bytes) {
        let text = decode_legacy(&bytes, legacy)?;
        return Ok(DecodedText {
            text,
            encoding: Encoding::Legacy(legacy),
            has_bom: false,
        });
    }

    Err(DocumentError::InvalidEncoding)
}

fn decode_utf16(bytes: &[u8], big_endian: bool) -> Result<String, DocumentError> {
    if bytes.len() % 2 != 0 {
        return Err(DocumentError::InvalidEncoding);
    }

    let units_iter = bytes.chunks_exact(2).map(|chunk| {
        let pair = [chunk[0], chunk[1]];
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    });
    let units: Vec<u16> = units_iter.collect();
    String::from_utf16(&units).map_err(|_| DocumentError::InvalidEncoding)
}

fn encode_utf16(text: &str, include_bom: bool, big_endian: bool) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(text.len() * 2 + if include_bom { 2 } else { 0 });
    if include_bom {
        buffer.extend_from_slice(if big_endian { b"\xFE\xFF" } else { b"\xFF\xFE" });
    }

    for unit in text.encode_utf16() {
        let bytes = if big_endian {
            unit.to_be_bytes()
        } else {
            unit.to_le_bytes()
        };
        buffer.extend_from_slice(&bytes);
    }
    buffer
}

fn encode_legacy(text: &str, legacy: LegacyEncoding) -> Result<Vec<u8>, DocumentError> {
    let encoder = legacy.to_rs();
    let (cow, _, had_errors) = encoder.encode(text);
    if had_errors {
        return Err(DocumentError::Unrepresentable(legacy.name()));
    }
    Ok(match cow {
        Cow::Borrowed(slice) => slice.to_vec(),
        Cow::Owned(vec) => vec,
    })
}

fn decode_legacy(bytes: &[u8], legacy: LegacyEncoding) -> Result<String, DocumentError> {
    let decoder = legacy.to_rs();
    let (cow, had_errors) = decoder.decode_without_bom_handling(bytes);
    if had_errors {
        return Err(DocumentError::InvalidEncoding);
    }
    Ok(match cow {
        Cow::Borrowed(slice) => slice.to_owned(),
        Cow::Owned(string) => string,
    })
}

fn detect_legacy_encoding(bytes: &[u8]) -> Option<LegacyEncoding> {
    if bytes.is_empty() {
        return None;
    }
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let guess = detector.guess(None, true);
    map_rs_encoding(guess)
}

fn map_rs_encoding(encoding: &'static RsEncoding) -> Option<LegacyEncoding> {
    if encoding == WINDOWS_1252 {
        Some(LegacyEncoding::Windows1252)
    } else if encoding == SHIFT_JIS {
        Some(LegacyEncoding::ShiftJis)
    } else if encoding == GBK {
        Some(LegacyEncoding::Gbk)
    } else if encoding == BIG5 {
        Some(LegacyEncoding::Big5)
    } else {
        None
    }
}

fn looks_like_utf16_le(bytes: &[u8]) -> bool {
    looks_like_utf16(bytes, false)
}

fn looks_like_utf16_be(bytes: &[u8]) -> bool {
    looks_like_utf16(bytes, true)
}

fn looks_like_utf16(bytes: &[u8], big_endian: bool) -> bool {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return false;
    }

    let sample_len = bytes.len().min(64);
    let mut zero_count = 0;
    let mut total = 0;

    for chunk in bytes[..sample_len].chunks_exact(2) {
        let zero_byte = if big_endian { chunk[0] } else { chunk[1] };
        if zero_byte == 0 {
            zero_count += 1;
        }
        total += 1;
    }

    total > 0 && zero_count * 2 >= total
}

fn system_time_to_nanos(time: SystemTime) -> Option<u128> {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_nanos()),
        Err(_) => None,
    }
}

/// 掃描原始文字找到第一個換行記號以推斷行尾偏好。 / Scans the raw text for the first newline sentinel to infer the preferred line ending.
fn detect_line_ending(text: &str) -> LineEnding {
    let bytes = text.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\r' => {
                if idx + 1 < bytes.len() && bytes[idx + 1] == b'\n' {
                    return LineEnding::CrLf;
                }
                return LineEnding::Cr;
            }
            b'\n' => return LineEnding::Lf,
            _ => {
                idx += 1;
                continue;
            }
        }
    }
    LineEnding::Lf
}

fn normalize_newlines(input: &str) -> String {
    // 將 CRLF 與 CR 轉成 LF，簡化記憶體儲存。 / Convert CRLF and CR sequences to LF for internal storage simplicity.
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                result.push('\n');
            }
            _ => result.push(ch),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::{GBK, SHIFT_JIS};
    use std::fs;
    use std::thread;
    use std::time::Duration;

    fn write_bytes(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).expect("failed to seed test file");
    }

    #[test]
    fn open_detects_line_endings_and_normalises_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("sample.txt");
        write_bytes(&file_path, b"line1\r\nline2\r\n");

        let doc = Document::open(&file_path).unwrap();
        assert_eq!(doc.contents(), "line1\nline2\n");
        assert_eq!(doc.line_ending(), LineEnding::CrLf);
        assert_eq!(doc.encoding(), Encoding::Utf8);
        assert!(!doc.has_bom());
        assert!(!doc.is_dirty());
    }

    #[test]
    fn open_handles_utf8_bom() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("bom.txt");
        write_bytes(&file_path, b"\xEF\xBB\xBFhello\n");

        let doc = Document::open(&file_path).unwrap();
        assert_eq!(doc.contents(), "hello\n");
        assert_eq!(doc.line_ending(), LineEnding::Lf);
        assert_eq!(doc.encoding(), Encoding::Utf8);
        assert!(doc.has_bom());
    }

    #[test]
    fn open_handles_utf16_le_bom() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("utf16-le.txt");
        // 包含 BOM 與字串 "hi\r\n!"。 / BOM plus the literal "hi\r\n!".
        let payload: &[u8] = b"\xFF\xFEh\x00i\x00\r\x00\n\x00!\x00";
        write_bytes(&file_path, payload);

        let doc = Document::open(&file_path).unwrap();
        assert_eq!(doc.contents(), "hi\n!");
        assert_eq!(doc.line_ending(), LineEnding::CrLf);
        assert_eq!(doc.encoding(), Encoding::Utf16Le);
        assert!(doc.has_bom());
    }

    #[test]
    fn open_handles_utf16_be_bom() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("utf16-be.txt");
        let payload = [0xFE, 0xFF, 0x00, b'h', 0x00, b'i', 0x00, b'!', 0x00, b'\n'];
        write_bytes(&file_path, &payload);

        let doc = Document::open(&file_path).unwrap();
        assert_eq!(doc.contents(), "hi!\n");
        assert_eq!(doc.line_ending(), LineEnding::Lf);
        assert_eq!(doc.encoding(), Encoding::Utf16Be);
        assert!(doc.has_bom());
    }

    #[test]
    fn open_detects_gbk_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("gbk.txt");
        let (encoded, _, _) = GBK.encode("中文測試");
        write_bytes(&file_path, encoded.as_ref());

        let doc = Document::open(&file_path).unwrap();
        assert_eq!(doc.contents(), "中文測試");
        assert!(matches!(
            doc.encoding(),
            Encoding::Legacy(LegacyEncoding::Gbk)
        ));
        assert!(!doc.has_bom());
    }

    #[test]
    fn open_detects_shift_jis_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("shiftjis.txt");
        let (encoded, _, _) = SHIFT_JIS.encode("テスト");
        write_bytes(&file_path, encoded.as_ref());

        let doc = Document::open(&file_path).unwrap();
        assert_eq!(doc.contents(), "テスト");
        assert!(matches!(
            doc.encoding(),
            Encoding::Legacy(LegacyEncoding::ShiftJis)
        ));
    }

    #[test]
    fn open_rejects_invalid_shift_jis_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("invalid-sjis.txt");
        // SHIFT_JIS 中 0x82 需搭配第二個位元組，搭配 0xFF 會造成解碼錯誤。 / SHIFT_JIS lead byte 0x82 paired with 0xFF produces an invalid sequence.
        write_bytes(&file_path, &[0x82, 0xFF]);

        let err = Document::open(&file_path).unwrap_err();
        assert!(matches!(err, DocumentError::InvalidEncoding));
    }

    #[test]
    fn save_preserves_line_endings_and_bom() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("output.txt");

        let mut doc = Document::new();
        doc.set_contents("a\nb\n");
        doc.set_line_ending(LineEnding::CrLf);
        doc.set_bom(true);
        doc.save_as(&file_path).unwrap();

        let bytes = fs::read(&file_path).unwrap();
        assert_eq!(&bytes[..3], b"\xEF\xBB\xBF");
        assert_eq!(&bytes[3..], b"a\r\nb\r\n");
        assert!(!doc.is_dirty());
    }

    #[test]
    fn save_serialises_utf16_le() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("utf16-le-save.txt");

        let mut doc = Document::new();
        doc.set_contents("Rust\n");
        doc.set_line_ending(LineEnding::CrLf);
        doc.set_encoding(Encoding::Utf16Le);
        doc.set_bom(true);
        doc.save_as(&file_path).unwrap();

        let bytes = fs::read(&file_path).unwrap();
        assert_eq!(&bytes[..2], b"\xFF\xFE");
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        let text = String::from_utf16(&units).unwrap();
        assert_eq!(text, "Rust\r\n");
        assert!(!doc.is_dirty());
    }

    #[test]
    fn save_serialises_utf16_be_without_bom() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("utf16-be-save.txt");

        let mut doc = Document::new();
        doc.set_contents("AB");
        doc.set_encoding(Encoding::Utf16Be);
        doc.set_bom(false);
        doc.save_as(&file_path).unwrap();

        let raw = fs::read(&file_path).unwrap();
        let units: Vec<u16> = bytes_to_u16_be(&raw);
        let text = String::from_utf16(&units).unwrap();
        assert_eq!(text, "AB");
    }

    #[test]
    fn save_serialises_gbk() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("gbk-save.txt");

        let mut doc = Document::new();
        doc.set_contents("中文");
        doc.set_encoding(Encoding::Legacy(LegacyEncoding::Gbk));
        doc.save_as(&file_path).unwrap();

        let bytes = fs::read(&file_path).unwrap();
        let (decoded, _, _) = GBK.decode(&bytes);
        assert_eq!(decoded.as_ref(), "中文");
    }

    #[test]
    fn save_rejects_unrepresentable_characters() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("latin1.txt");

        let mut doc = Document::new();
        doc.set_contents("漢");
        doc.set_encoding(Encoding::Legacy(LegacyEncoding::Windows1252));
        let err = doc.save_as(&file_path).unwrap_err();
        assert!(matches!(
            err,
            DocumentError::Unrepresentable("windows-1252")
        ));
    }

    #[test]
    fn save_overwrites_existing_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("overwrite.txt");
        write_bytes(&file_path, b"old");

        let mut doc = Document::open(&file_path).unwrap();
        doc.set_contents("new\ncontent\n");
        doc.set_line_ending(LineEnding::Lf);
        doc.set_bom(false);
        doc.save().unwrap();

        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents, "new\ncontent\n");
        assert!(!doc.is_dirty());
    }

    #[test]
    fn check_disk_state_reports_modification_and_reload_resets_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("monitor.txt");
        write_bytes(&file_path, b"alpha");

        let mut doc = Document::open(&file_path).unwrap();
        thread::sleep(Duration::from_millis(10));
        write_bytes(&file_path, b"alpha-beta");

        assert_eq!(doc.check_disk_state().unwrap(), DiskState::Modified);
        doc.reload().unwrap();
        assert_eq!(doc.contents(), "alpha-beta");
        assert_eq!(doc.check_disk_state().unwrap(), DiskState::Unchanged);
    }

    #[test]
    fn check_disk_state_reports_removal() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("removed.txt");
        write_bytes(&file_path, b"temporary");

        let doc = Document::open(&file_path).unwrap();
        fs::remove_file(&file_path).unwrap();

        assert_eq!(doc.check_disk_state().unwrap(), DiskState::Removed);
    }

    fn bytes_to_u16_be(bytes: &[u8]) -> Vec<u16> {
        bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect()
    }
}
