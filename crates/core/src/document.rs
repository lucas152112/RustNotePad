use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
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
}

/// 文件載入或儲存時可能發生的錯誤。 / Errors that can occur while loading or saving a document.
#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("file encoding is not supported or data is invalid")]
    InvalidEncoding,
}

/// 代表以 Unicode 文字檔為後盾的文件記憶體模型。 / In-memory representation of a text document backed by a Unicode text file.
#[derive(Debug, Clone)]
pub struct Document {
    path: Option<PathBuf>,
    contents: String,
    line_ending: LineEnding,
    encoding: Encoding,
    has_bom: bool,
    is_dirty: bool,
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
        }
    }

    /// 從磁碟載入文件並將行尾內部正規化為 `\n`。 / Loads a document from disk, normalising newlines to `\n` internally.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let decoded = decode_bytes(bytes)?;
        let line_ending = detect_line_ending(&decoded.text);
        let contents = normalize_newlines(&decoded.text);

        Ok(Self {
            path: Some(path_ref.to_path_buf()),
            contents,
            line_ending,
            encoding: decoded.encoding,
            has_bom: decoded.has_bom,
            is_dirty: false,
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
        let encoded = self.serialise_contents();

        // 先寫入暫存檔再重新命名，避免出現部分寫入的情況。 / Use a temporary file plus rename to guard against partial writes.
        let tmp_path = path_ref.with_extension("tmp_rustnotepad");
        {
            let mut tmp_file = File::create(&tmp_path)?;
            tmp_file.write_all(&encoded)?;
            tmp_file.sync_all()?; // 確保資料在重新命名前已寫入磁碟。 / Ensure bytes hit the disk before rename.
        }
        fs::rename(&tmp_path, path_ref)?;

        self.path = Some(path_ref.to_path_buf());
        self.is_dirty = false;
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

    /// 取得文件所屬的檔案路徑（若存在）。 / Retrieves the associated path if the document is linked to one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn serialise_contents(&self) -> Vec<u8> {
        let text = self.contents.replace('\n', self.line_ending.as_str());
        match self.encoding {
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
        }
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

    let text = String::from_utf8(bytes).map_err(|_| DocumentError::InvalidEncoding)?;
    Ok(DecodedText {
        text,
        encoding: Encoding::Utf8,
        has_bom: false,
    })
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
    let mut buffer =
        Vec::with_capacity(text.len() * 2 + if include_bom { 2 } else { 0 });
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
    use std::fs;

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
        let payload = [
            0xFE, 0xFF, 0x00, b'h', 0x00, b'i', 0x00, b'!', 0x00, b'\n',
        ];
        write_bytes(&file_path, &payload);

        let doc = Document::open(&file_path).unwrap();
        assert_eq!(doc.contents(), "hi!\n");
        assert_eq!(doc.line_ending(), LineEnding::Lf);
        assert_eq!(doc.encoding(), Encoding::Utf16Be);
        assert!(doc.has_bom());
    }

    #[test]
    fn open_rejects_non_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("invalid.txt");
        // 無效的 UTF-8 序列，不符合支援的編碼。 / Invalid UTF-8 sequence that does not map to supported encodings.
        write_bytes(&file_path, &[0xC3, 0x28]);

        let err = Document::open(&file_path).unwrap_err();
        match err {
            DocumentError::InvalidEncoding => {}
            other => panic!("unexpected error: {:?}", other),
        }
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

    fn bytes_to_u16_be(bytes: &[u8]) -> Vec<u16> {
        bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect()
    }
}
