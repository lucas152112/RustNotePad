//! Macro recording, storage, and playback facilities for RustNotePad.
//! （提供 RustNotePad 使用的巨集錄製、儲存與回放功能。）
//!
//! The module models macros as ordered events that reference command identifiers
//! or text insertions. Macros can be recorded at runtime, persisted to disk, and
//! replayed with repeat counts through a user-supplied executor implementation.
//! 本模組以事件序列描述巨集內容，事件可包含指令識別碼或文字插入。
//! 巨集支援在執行時錄製、保存至檔案，並透過外部提供的執行器在指定次數下回放。

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::path::Path;
use thiserror::Error;

/// Error type for macro operations.
/// （巨集操作時可能發生的錯誤型別。）
#[derive(Debug, Error)]
pub enum MacroError {
    #[error("recorder already active")]
    RecorderAlreadyActive,
    #[error("recorder is not running")]
    RecorderNotRunning,
    #[error("macro name cannot be empty")]
    EmptyMacroName,
    #[error("macro with the same name already exists: {0}")]
    DuplicateMacroName(String),
    #[error("macro not found: {0}")]
    MacroNotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error("{0}")]
    ExecutorError(String),
}

/// Represents a keyboard shortcut bound to a macro.
/// （表示與巨集綁定的鍵盤捷徑設定。）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroShortcut {
    pub primary: String,
    pub secondary: Option<String>,
}

impl MacroShortcut {
    /// Creates a new shortcut using the provided primary and optional secondary chords.
    /// （以主要按鍵組合與可選的第二組按鍵建立捷徑設定。）
    pub fn new(primary: impl Into<String>, secondary: Option<String>) -> Self {
        Self {
            primary: primary.into(),
            secondary,
        }
    }
}

/// Events captured while recording a macro.
/// （巨集錄製過程中擷取到的事件列表。）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroEvent {
    Command { id: String },
    InsertText { text: String },
}

impl MacroEvent {
    /// Returns a human-readable description of the event.
    /// （回傳此事件的可讀描述文字。）
    pub fn describe(&self) -> Cow<'_, str> {
        match self {
            MacroEvent::Command { id } => Cow::Owned(format!("command:{id}")),
            MacroEvent::InsertText { text } => {
                let snippet: String = text.chars().take(16).collect();
                if text.len() > snippet.len() {
                    Cow::Owned(format!("text:\"{snippet}…\""))
                } else {
                    Cow::Owned(format!("text:\"{snippet}\""))
                }
            }
        }
    }
}

/// Fully recorded macro with metadata and ordered events.
/// （完整的巨集定義，包含中繼資料與事件序列。）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedMacro {
    pub name: String,
    pub shortcut: Option<MacroShortcut>,
    pub events: Vec<MacroEvent>,
}

impl RecordedMacro {
    /// Creates a macro definition from a name and events.
    /// （以名稱與事件序列建立巨集定義。）
    pub fn new(
        name: impl Into<String>,
        events: Vec<MacroEvent>,
        shortcut: Option<MacroShortcut>,
    ) -> Result<Self, MacroError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MacroError::EmptyMacroName);
        }
        Ok(Self {
            name,
            shortcut,
            events,
        })
    }

    /// Returns true when the macro has no events.
    /// （判斷此巨集是否沒有任何事件。）
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Trait implemented by runtime integrations that can execute macro events.
/// （由執行階段整合層實作，用來執行巨集事件的介面。）
pub trait MacroExecutor {
    /// Executes the specified command identifier.
    /// （執行指定指令識別碼對應的行為。）
    fn execute_command(&mut self, command_id: &str) -> Result<(), String>;

    /// Inserts text at the current caret.
    /// （在目前游標位置插入文字。）
    fn insert_text(&mut self, text: &str) -> Result<(), String>;
}

/// Responsible for replaying recorded macros via a supplied executor.
/// （透過提供的執行器回放巨集的元件。）
pub struct MacroPlayer;

impl MacroPlayer {
    /// Replays a macro the requested number of times.
    /// （按照指定次數回放巨集。）
    pub fn play(
        macro_def: &RecordedMacro,
        repeat: NonZeroUsize,
        executor: &mut dyn MacroExecutor,
    ) -> Result<(), MacroError> {
        for _ in 0..repeat.get() {
            for event in &macro_def.events {
                match event {
                    MacroEvent::Command { id } => {
                        executor
                            .execute_command(id)
                            .map_err(|e| MacroError::ExecutorError(e))?;
                    }
                    MacroEvent::InsertText { text } => {
                        executor
                            .insert_text(text)
                            .map_err(|e| MacroError::ExecutorError(e))?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Tracks recording state and accumulates events.
/// （追蹤錄製狀態並累積事件的元件。）
#[derive(Default)]
pub struct MacroRecorder {
    state: RecorderState,
}

#[derive(Default)]
enum RecorderState {
    #[default]
    Idle,
    Recording {
        events: Vec<MacroEvent>,
    },
}

impl MacroRecorder {
    /// Creates a new recorder.
    /// （建立新的錄製器實體。）
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when currently recording.
    /// （回報目前是否正在錄製中。）
    pub fn is_recording(&self) -> bool {
        matches!(self.state, RecorderState::Recording { .. })
    }

    /// Starts recording, returning an error if already active.
    /// （開始錄製，若已在錄製則回傳錯誤。）
    pub fn start(&mut self) -> Result<(), MacroError> {
        if self.is_recording() {
            return Err(MacroError::RecorderAlreadyActive);
        }
        self.state = RecorderState::Recording { events: Vec::new() };
        Ok(())
    }

    /// Cancels recording and discards accumulated events.
    /// （取消錄製並丟棄已累積的事件。）
    pub fn cancel(&mut self) {
        self.state = RecorderState::Idle;
    }

    /// Pushes a command event into the recorder.
    /// （將指令事件加入錄製序列。）
    pub fn record_command(&mut self, id: impl Into<String>) -> Result<(), MacroError> {
        match &mut self.state {
            RecorderState::Recording { events } => {
                events.push(MacroEvent::Command { id: id.into() });
                Ok(())
            }
            RecorderState::Idle => Err(MacroError::RecorderNotRunning),
        }
    }

    /// Pushes a text insertion event into the recorder.
    /// （加入文字插入事件。）
    pub fn record_text(&mut self, text: impl Into<String>) -> Result<(), MacroError> {
        match &mut self.state {
            RecorderState::Recording { events } => {
                events.push(MacroEvent::InsertText { text: text.into() });
                Ok(())
            }
            RecorderState::Idle => Err(MacroError::RecorderNotRunning),
        }
    }

    /// Finishes recording and returns the resulting macro.
    /// （結束錄製並回傳產生的巨集。）
    pub fn finish(
        &mut self,
        name: impl Into<String>,
        shortcut: Option<MacroShortcut>,
    ) -> Result<RecordedMacro, MacroError> {
        let RecorderState::Recording { events } = std::mem::take(&mut self.state) else {
            return Err(MacroError::RecorderNotRunning);
        };
        let macro_def = RecordedMacro::new(name, events, shortcut)?;
        self.state = RecorderState::Idle;
        Ok(macro_def)
    }
}

/// Macro collection with persistence helpers.
/// （管理巨集集合並提供存取的工具。）
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroStore {
    macros: BTreeMap<String, RecordedMacro>,
}

impl MacroStore {
    /// Creates an empty store.
    /// （建立空的巨集儲存區。）
    pub fn new() -> Self {
        Self {
            macros: BTreeMap::new(),
        }
    }

    /// Inserts a macro, rejecting duplicate names.
    /// （新增巨集，若名稱重複則回傳錯誤。）
    pub fn insert(&mut self, macro_def: RecordedMacro) -> Result<(), MacroError> {
        let name = macro_def.name.clone();
        if self.macros.contains_key(&name) {
            return Err(MacroError::DuplicateMacroName(name));
        }
        self.macros.insert(name, macro_def);
        Ok(())
    }

    /// Removes a macro by name.
    /// （依名稱移除巨集。）
    pub fn remove(&mut self, name: &str) -> Result<RecordedMacro, MacroError> {
        self.macros
            .remove(name)
            .ok_or_else(|| MacroError::MacroNotFound(name.to_owned()))
    }

    /// Retrieves a macro by name.
    /// （依名稱取得巨集。）
    pub fn get(&self, name: &str) -> Option<&RecordedMacro> {
        self.macros.get(name)
    }

    /// Returns an iterator over macros sorted by name.
    /// （以名稱排序的迭代器。）
    pub fn iter(&self) -> impl Iterator<Item = (&String, &RecordedMacro)> {
        self.macros.iter()
    }

    /// Saves macros to the specified writer in JSON format.
    /// （將巨集以 JSON 格式寫入提供的 writer。）
    pub fn save<W: Write>(&self, mut writer: W) -> Result<(), MacroError> {
        let encoded = serde_json::to_vec_pretty(&self.macros)?;
        writer.write_all(&encoded)?;
        Ok(())
    }

    /// Loads macros from the specified reader in JSON format.
    /// （從 JSON 格式的 reader 載入巨集。）
    pub fn load<R: Read>(mut reader: R) -> Result<Self, MacroError> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        if buf.is_empty() {
            return Ok(Self::new());
        }
        let macros: BTreeMap<String, RecordedMacro> = serde_json::from_slice(&buf)?;
        Ok(Self { macros })
    }

    /// Loads macros from a path.
    /// （從檔案路徑載入巨集。）
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, MacroError> {
        let file = File::open(path)?;
        Self::load(file)
    }

    /// Saves macros to a path, overwriting existing contents.
    /// （儲存巨集至檔案，會覆寫既有內容。）
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), MacroError> {
        let mut file = File::create(path)?;
        self.save(&mut file)
    }
}

impl Display for MacroStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, (name, macro_def)) in self.macros.iter().enumerate() {
            if idx > 0 {
                writeln!(f)?;
            }
            writeln!(f, "{name}")?;
            for event in &macro_def.events {
                writeln!(f, "  - {}", event.describe())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingExecutor {
        commands: Vec<String>,
        insertions: Vec<String>,
        fail_on: Option<String>,
    }

    impl RecordingExecutor {
        fn new() -> Self {
            Self {
                commands: Vec::new(),
                insertions: Vec::new(),
                fail_on: None,
            }
        }

        fn with_failure(id: &str) -> Self {
            Self {
                commands: Vec::new(),
                insertions: Vec::new(),
                fail_on: Some(id.to_owned()),
            }
        }
    }

    impl MacroExecutor for RecordingExecutor {
        fn execute_command(&mut self, command_id: &str) -> Result<(), String> {
            if self.fail_on.as_deref() == Some(command_id) {
                return Err(format!("command failed: {command_id}"));
            }
            self.commands.push(command_id.to_owned());
            Ok(())
        }

        fn insert_text(&mut self, text: &str) -> Result<(), String> {
            self.insertions.push(text.to_owned());
            Ok(())
        }
    }

    #[test]
    fn record_and_playback_macro() {
        let mut recorder = MacroRecorder::new();
        recorder.start().unwrap();
        recorder.record_command("command.open").unwrap();
        recorder.record_text("hello").unwrap();
        recorder.record_command("command.save").unwrap();
        let macro_def = recorder
            .finish(
                "Sample Macro",
                Some(MacroShortcut::new("Ctrl+Shift+1", None)),
            )
            .unwrap();

        assert_eq!(
            macro_def.events.len(),
            3,
            "macro event count / 巨集事件數量"
        );

        let mut exec = RecordingExecutor::new();
        MacroPlayer::play(&macro_def, NonZeroUsize::new(2).unwrap(), &mut exec).unwrap();

        assert_eq!(
            exec.commands,
            vec![
                "command.open",
                "command.save",
                "command.open",
                "command.save"
            ],
            "command execution order / 指令執行順序不符"
        );
        assert_eq!(
            exec.insertions,
            vec!["hello", "hello"],
            "text insertions / 文字插入次序不符"
        );
    }

    #[test]
    fn recorder_state_transitions() {
        let mut recorder = MacroRecorder::new();
        assert!(!recorder.is_recording());
        recorder.start().unwrap();
        assert!(recorder.is_recording());
        assert!(matches!(
            recorder.start(),
            Err(MacroError::RecorderAlreadyActive)
        ));
        recorder.cancel();
        assert!(!recorder.is_recording());
        assert!(matches!(
            recorder.record_command("test"),
            Err(MacroError::RecorderNotRunning)
        ));
    }

    #[test]
    fn store_serialization_roundtrip() {
        let macro_a = RecordedMacro::new(
            "Insert Greeting",
            vec![
                MacroEvent::InsertText {
                    text: "Hello".into(),
                },
                MacroEvent::InsertText {
                    text: ", world!".into(),
                },
            ],
            None,
        )
        .unwrap();
        let macro_b = RecordedMacro::new(
            "Save File",
            vec![MacroEvent::Command {
                id: "command.save".into(),
            }],
            Some(MacroShortcut::new("Ctrl+S", None)),
        )
        .unwrap();

        let mut store = MacroStore::new();
        store.insert(macro_a.clone()).unwrap();
        store.insert(macro_b.clone()).unwrap();

        let mut buf = Vec::new();
        store.save(&mut buf).unwrap();

        let loaded = MacroStore::load(&buf[..]).unwrap();
        assert_eq!(
            loaded.get("Insert Greeting"),
            Some(&macro_a),
            "macro should persist / 巨集應成功保存"
        );
        assert_eq!(
            loaded.get("Save File"),
            Some(&macro_b),
            "macro should persist / 巨集應成功保存"
        );
    }

    #[test]
    fn duplicate_macro_rejected() {
        let mut store = MacroStore::new();
        let macro_def =
            RecordedMacro::new("Test", vec![], Some(MacroShortcut::new("Ctrl+Alt+T", None)))
                .unwrap();
        store.insert(macro_def.clone()).unwrap();
        let err = store.insert(macro_def).unwrap_err();
        match err {
            MacroError::DuplicateMacroName(name) => assert_eq!(name, "Test"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn playback_propagates_executor_errors() {
        let macro_def = RecordedMacro::new(
            "Broken Command",
            vec![MacroEvent::Command {
                id: "command.fail".into(),
            }],
            None,
        )
        .unwrap();

        let mut exec = RecordingExecutor::with_failure("command.fail");
        let err =
            MacroPlayer::play(&macro_def, NonZeroUsize::new(1).unwrap(), &mut exec).unwrap_err();
        match err {
            MacroError::ExecutorError(message) => {
                assert!(
                    message.contains("command.fail"),
                    "error should cite failing command / 錯誤訊息需指出失敗指令"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
