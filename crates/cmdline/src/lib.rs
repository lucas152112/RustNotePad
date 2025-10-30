//! Command-line parser for RustNotePad launch scenarios.
//! RustNotePad 啟動參數解析器。

use std::ffi::OsString;
use std::iter::Peekable;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Parsed launch configuration derived from CLI arguments.
/// 從命令列參數解析出的啟動組態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchConfig {
    pub program: Option<PathBuf>,
    pub multi_instance: bool,
    pub skip_session: bool,
    pub files: Vec<FileTarget>,
    pub session_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
    pub theme: Option<ThemeSpec>,
    pub workspace_root: Option<PathBuf>,
    pub raw_unknown: Vec<String>,
    pub suppress_plugins: bool,
}

impl LaunchConfig {
    pub fn empty() -> Self {
        Self {
            program: None,
            multi_instance: false,
            skip_session: false,
            files: Vec::new(),
            session_path: None,
            project_path: None,
            theme: None,
            workspace_root: None,
            raw_unknown: Vec::new(),
            suppress_plugins: false,
        }
    }
}

/// Describes a file that should be opened on launch.
/// 描述啟動時需開啟的檔案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTarget {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub read_only: bool,
    pub language: Option<String>,
}

impl FileTarget {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            line: None,
            column: None,
            read_only: false,
            language: None,
        }
    }
}

/// Theme selection derived from CLI input.
/// 從命令列取得的主題設定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSpec {
    Name(String),
    Path(PathBuf),
}

/// Errors emitted while parsing CLI arguments.
/// 解析命令列參數時可能回傳的錯誤。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("unknown flag: {0}")]
    UnknownFlag(String),
    #[error("missing value for option '{0}'")]
    MissingValue(String),
    #[error("invalid numeric value '{value}' for option '{option}'")]
    InvalidNumber { option: String, value: String },
    #[error("option '{0}' requires a file argument")]
    MissingFileForOption(String),
    #[error("language value for '-l' cannot be empty")]
    EmptyLanguage,
}

/// Parses command-line arguments into a [`LaunchConfig`].
/// 將命令列參數解析為 [`LaunchConfig`]。
pub fn parse<I, S>(args: I) -> Result<LaunchConfig, ParseError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let converted: Vec<String> = args
        .into_iter()
        .map(|item| item.into().to_string_lossy().to_string())
        .collect();
    parse_strings(converted.into_iter())
}

fn parse_strings<I>(mut args: I) -> Result<LaunchConfig, ParseError>
where
    I: Iterator<Item = String>,
{
    let mut config = LaunchConfig::empty();
    if let Some(program) = args.next() {
        config.program = Some(PathBuf::from(program));
    }
    let mut iter = args.peekable();
    let mut pending = PendingState::default();

    while let Some(arg) = iter.next() {
        if arg == "--" {
            drain_files(&mut config, &mut pending, iter);
            break;
        }
        if let Some(long) = arg.strip_prefix("--") {
            parse_long_option(long, &mut iter, &mut config, &mut pending)?;
            continue;
        }
        if arg.starts_with('-') && arg.len() > 1 {
            parse_legacy_option(arg, &mut iter, &mut config, &mut pending)?;
            continue;
        }
        append_file(arg, &mut config, &mut pending);
    }

    if pending.has_directives() {
        return Err(ParseError::MissingFileForOption(
            pending.describe_pending_option(),
        ));
    }

    Ok(config)
}

fn parse_long_option<I>(
    option: &str,
    iter: &mut Peekable<I>,
    config: &mut LaunchConfig,
    _pending: &mut PendingState,
) -> Result<(), ParseError>
where
    I: Iterator<Item = String>,
{
    let (name, value_inline) = split_name_value(option);
    match name {
        "session" => {
            let value = require_value("session", value_inline, iter)?;
            config.session_path = Some(PathBuf::from(value));
        }
        "project" => {
            let value = require_value("project", value_inline, iter)?;
            config.project_path = Some(PathBuf::from(value));
        }
        "theme" => {
            let value = require_value("theme", value_inline, iter)?;
            if looks_like_path(&value) {
                config.theme = Some(ThemeSpec::Path(PathBuf::from(value)));
            } else {
                config.theme = Some(ThemeSpec::Name(value));
            }
        }
        "workspace" => {
            let value = require_value("workspace", value_inline, iter)?;
            config.workspace_root = Some(PathBuf::from(value));
        }
        other => {
            config
                .raw_unknown
                .push(format!("--{other}{}", value_inline.unwrap_or_default()));
        }
    }
    Ok(())
}

fn parse_legacy_option<I>(
    arg: String,
    iter: &mut Peekable<I>,
    config: &mut LaunchConfig,
    pending: &mut PendingState,
) -> Result<(), ParseError>
where
    I: Iterator<Item = String>,
{
    let stripped = arg.trim_start_matches('-');
    let lowered = stripped.to_ascii_lowercase();
    match lowered.as_str() {
        "multiinst" => {
            config.multi_instance = true;
        }
        "nosession" => {
            config.skip_session = true;
        }
        "noplugin" => {
            config.suppress_plugins = true;
        }
        "ro" => {
            pending.read_only = true;
        }
        _ => {
            if let Some(value) = lowered.strip_prefix('n') {
                let number = parse_numeric_option("n", value, iter)?;
                pending.line = Some(number);
                return Ok(());
            }
            if let Some(value) = lowered.strip_prefix('c') {
                let number = parse_numeric_option("c", value, iter)?;
                pending.column = Some(number);
                return Ok(());
            }
            if let Some(value) = stripped.strip_prefix('l') {
                let lang = parse_string_option("l", value, iter)?;
                if lang.is_empty() {
                    return Err(ParseError::EmptyLanguage);
                }
                pending.language = Some(lang);
                return Ok(());
            }
            return Err(ParseError::UnknownFlag(arg));
        }
    }
    Ok(())
}

fn parse_numeric_option<I>(
    option: &str,
    inline: &str,
    iter: &mut Peekable<I>,
) -> Result<u32, ParseError>
where
    I: Iterator<Item = String>,
{
    let value = fetch_option_value(option, inline, iter)?;
    value.parse::<u32>().map_err(|_| ParseError::InvalidNumber {
        option: option.to_string(),
        value,
    })
}

fn parse_string_option<I>(
    option: &str,
    inline: &str,
    iter: &mut Peekable<I>,
) -> Result<String, ParseError>
where
    I: Iterator<Item = String>,
{
    fetch_option_value(option, inline, iter)
}

fn fetch_option_value<I>(
    option: &str,
    inline: &str,
    iter: &mut Peekable<I>,
) -> Result<String, ParseError>
where
    I: Iterator<Item = String>,
{
    if !inline.is_empty() {
        if let Some(stripped) = inline.strip_prefix('=') {
            if stripped.is_empty() {
                return iter
                    .next()
                    .ok_or_else(|| ParseError::MissingValue(option.to_string()));
            }
            return Ok(stripped.to_string());
        }
        return Ok(inline.to_string());
    }
    iter.next()
        .ok_or_else(|| ParseError::MissingValue(option.to_string()))
}

fn split_name_value(option: &str) -> (&str, Option<String>) {
    if let Some((name, value)) = option.split_once('=') {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            (name, Some(String::new()))
        } else {
            (name, Some(trimmed.to_string()))
        }
    } else {
        (option, None)
    }
}

fn require_value<I>(
    option: &str,
    inline: Option<String>,
    iter: &mut Peekable<I>,
) -> Result<String, ParseError>
where
    I: Iterator<Item = String>,
{
    if let Some(value) = inline {
        if value.is_empty() {
            return iter
                .next()
                .ok_or_else(|| ParseError::MissingValue(option.to_string()));
        }
        return Ok(value);
    }
    iter.next()
        .ok_or_else(|| ParseError::MissingValue(option.to_string()))
}

fn append_file<S>(arg: S, config: &mut LaunchConfig, pending: &mut PendingState)
where
    S: Into<String>,
{
    let path = PathBuf::from(arg.into());
    let mut target = FileTarget::new(path);
    target.line = pending.line.take();
    target.column = pending.column.take();
    target.read_only = pending.read_only;
    target.language = pending.language.take();
    config.files.push(target);
    pending.reset_flags();
}

fn drain_files<I>(config: &mut LaunchConfig, pending: &mut PendingState, iter: Peekable<I>)
where
    I: Iterator<Item = String>,
{
    for arg in iter {
        append_file(arg, config, pending);
    }
}

fn looks_like_path(value: &str) -> bool {
    value.contains(std::path::MAIN_SEPARATOR)
        || value.contains('/')
        || value.contains('\\')
        || value.ends_with(".json")
        || Path::new(value).extension().is_some()
}

#[derive(Debug, Default)]
struct PendingState {
    line: Option<u32>,
    column: Option<u32>,
    language: Option<String>,
    read_only: bool,
}

impl PendingState {
    fn reset_flags(&mut self) {
        self.read_only = false;
    }

    fn has_directives(&self) -> bool {
        self.line.is_some() || self.column.is_some() || self.language.is_some() || self.read_only
    }

    fn describe_pending_option(&self) -> String {
        if self.line.is_some() {
            "-n".to_string()
        } else if self.column.is_some() {
            "-c".to_string()
        } else if self.language.is_some() {
            "-l".to_string()
        } else {
            "-ro".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_file() {
        let args = [
            "rustnotepad".to_string(),
            "README.md".to_string(),
            "src/lib.rs".to_string(),
        ];
        let config = parse_strings(args.into_iter()).unwrap();
        assert_eq!(config.files.len(), 2);
        assert_eq!(config.files[0].path, PathBuf::from("README.md"));
        assert_eq!(config.files[1].path, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn parses_line_and_column() {
        let args = [
            "rustnotepad".into(),
            "-n42".into(),
            "-c7".into(),
            "main.rs".into(),
        ];
        let config = parse_strings(args.into_iter()).unwrap();
        assert_eq!(config.files.len(), 1);
        let target = &config.files[0];
        assert_eq!(target.line, Some(42));
        assert_eq!(target.column, Some(7));
    }

    #[test]
    fn parses_language_and_read_only() {
        let args = [
            "rustnotepad".into(),
            "-lRust".into(),
            "-ro".into(),
            "code.rs".into(),
        ];
        let config = parse_strings(args.into_iter()).unwrap();
        let target = &config.files[0];
        assert_eq!(target.language.as_deref(), Some("Rust"));
        assert!(target.read_only);
    }

    #[test]
    fn parses_long_options() {
        let args = [
            "rustnotepad".into(),
            "--session=my.rnsession".into(),
            "--project".into(),
            "tree.json".into(),
            "--theme".into(),
            "Solarized Dark".into(),
            "--workspace".into(),
            "/tmp/ws".into(),
        ];
        let config = parse_strings(args.into_iter()).unwrap();
        assert_eq!(config.session_path, Some(PathBuf::from("my.rnsession")));
        assert_eq!(config.project_path, Some(PathBuf::from("tree.json")));
        assert_eq!(config.theme, Some(ThemeSpec::Name("Solarized Dark".into())));
        assert_eq!(config.workspace_root, Some(PathBuf::from("/tmp/ws")));
    }

    #[test]
    fn errors_on_missing_value() {
        let args = ["rustnotepad".into(), "--session".into()];
        let err = parse_strings(args.into_iter()).unwrap_err();
        assert!(matches!(err, ParseError::MissingValue(option) if option == "session"));
    }

    #[test]
    fn errors_on_trailing_directive() {
        let args = ["rustnotepad".into(), "-n10".into()];
        let err = parse_strings(args.into_iter()).unwrap_err();
        assert!(matches!(err, ParseError::MissingFileForOption(option) if option == "-n"));
    }

    #[test]
    fn unknown_flags_are_reported() {
        let args = ["rustnotepad".into(), "-unknown".into()];
        let err = parse_strings(args.into_iter()).unwrap_err();
        assert!(matches!(err, ParseError::UnknownFlag(flag) if flag == "-unknown"));
    }
}
