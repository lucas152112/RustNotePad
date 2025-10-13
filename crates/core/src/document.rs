use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Represents the current line ending style for a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    /// Returns the literal string representation used when serialising text.
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }
}

/// Errors that can occur while loading or saving a document.
#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("file is not valid UTF-8")]
    InvalidEncoding,
}

/// In-memory representation of a text document backed by a UTF-8 file.
#[derive(Debug, Clone)]
pub struct Document {
    path: Option<PathBuf>,
    contents: String,
    line_ending: LineEnding,
    has_bom: bool,
    is_dirty: bool,
}

impl Document {
    /// Creates an unsaved document with empty contents.
    pub fn new() -> Self {
        Self {
            path: None,
            contents: String::new(),
            line_ending: LineEnding::Lf,
            has_bom: false,
            is_dirty: false,
        }
    }

    /// Loads a document from disk, normalising newlines to `\n` internally.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let (has_bom, decoded) = decode_utf8(bytes)?;
        let line_ending = detect_line_ending(&decoded);
        let contents = normalize_newlines(&decoded);

        Ok(Self {
            path: Some(path_ref.to_path_buf()),
            contents,
            line_ending,
            has_bom,
            is_dirty: false,
        })
    }

    /// Saves the document to its current path. Fails if the document has no path yet.
    pub fn save(&mut self) -> Result<(), DocumentError> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "document has no associated path"))?
            .to_path_buf();
        self.save_as(path)
    }

    /// Saves the document to a new path, updating the associated metadata.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), DocumentError> {
        let path_ref = path.as_ref();
        let encoded = self.serialise_contents();

        // Use a temporary file + rename to guard against partial writes.
        let tmp_path = path_ref.with_extension("tmp_rustnotepad");
        {
            let mut tmp_file = File::create(&tmp_path)?;
            tmp_file.write_all(&encoded)?;
            tmp_file.sync_all()?; // make sure bytes hit the disk before rename
        }
        fs::rename(&tmp_path, path_ref)?;

        self.path = Some(path_ref.to_path_buf());
        self.is_dirty = false;
        Ok(())
    }

    /// Returns the current document contents, normalised to `\n` line endings.
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// Replaces the in-memory contents, marking the document as dirty.
    pub fn set_contents(&mut self, text: impl Into<String>) {
        let text = normalize_newlines(&text.into());
        self.contents = text;
        self.is_dirty = true;
    }

    /// Returns the current line ending preference.
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Updates the line ending preference.
    pub fn set_line_ending(&mut self, ending: LineEnding) {
        if self.line_ending != ending {
            self.line_ending = ending;
            self.is_dirty = true;
        }
    }

    /// Indicates whether the document includes a UTF-8 BOM when saved.
    pub fn has_bom(&self) -> bool {
        self.has_bom
    }

    /// Updates the BOM flag. Setting the value marks the document dirty if it changes.
    pub fn set_bom(&mut self, has_bom: bool) {
        if self.has_bom != has_bom {
            self.has_bom = has_bom;
            self.is_dirty = true;
        }
    }

    /// Returns whether the document has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Retrieves the associated path if the document is linked to one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn serialise_contents(&self) -> Vec<u8> {
        let text = self.contents.replace('\n', self.line_ending.as_str());
        if self.has_bom {
            // Prepend UTF-8 BOM bytes to the encoded payload.
            let mut prefixed = Vec::with_capacity(3 + text.len());
            prefixed.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
            prefixed.extend_from_slice(text.as_bytes());
            return prefixed;
        }
        text.into_bytes()
    }
}

fn decode_utf8(bytes: Vec<u8>) -> Result<(bool, String), DocumentError> {
    const UTF8_BOM: &[u8; 3] = b"\xEF\xBB\xBF";
    if bytes.starts_with(UTF8_BOM) {
        // Strip the BOM and decode the remaining payload.
        let string =
            String::from_utf8(bytes[3..].to_vec()).map_err(|_| DocumentError::InvalidEncoding)?;
        Ok((true, string))
    } else {
        let string = String::from_utf8(bytes).map_err(|_| DocumentError::InvalidEncoding)?;
        Ok((false, string))
    }
}

/// Scans the raw text for the first newline sentinel to infer the preferred line ending.
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
    // Convert CRLF and CR sequences to LF for internal storage simplicity.
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
        assert!(doc.has_bom());
    }

    #[test]
    fn open_rejects_non_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("invalid.txt");
        // Invalid UTF-8 sequence.
        write_bytes(&file_path, b"\xFF\xFE");

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
}
