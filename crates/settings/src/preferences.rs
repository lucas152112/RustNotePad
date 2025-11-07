use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

const PREFERENCES_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum PreferencesError {
    #[error("failed to read preferences {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse preferences {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize preferences {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write preferences {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to prepare directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub editor: EditorPreferences,
    #[serde(default)]
    pub ui: UiPreferences,
}

fn default_version() -> u32 {
    PREFERENCES_VERSION
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: PREFERENCES_VERSION,
            editor: EditorPreferences::default(),
            ui: UiPreferences::default(),
        }
    }
}

impl Preferences {
    pub fn sanitize(&mut self) {
        if self.version == 0 {
            self.version = PREFERENCES_VERSION;
        }
        self.editor.sanitize();
        self.ui.sanitize();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorPreferences {
    #[serde(default = "default_true")]
    pub autosave_enabled: bool,
    #[serde(default = "default_autosave_interval")]
    pub autosave_interval_minutes: u32,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub highlight_active_line: bool,
}

fn default_true() -> bool {
    true
}

fn default_autosave_interval() -> u32 {
    5
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            autosave_enabled: true,
            autosave_interval_minutes: default_autosave_interval(),
            show_line_numbers: true,
            highlight_active_line: true,
        }
    }
}

impl EditorPreferences {
    fn sanitize(&mut self) {
        if self.autosave_interval_minutes == 0 {
            self.autosave_interval_minutes = default_autosave_interval();
        }
        self.autosave_interval_minutes = self.autosave_interval_minutes.clamp(1, 240);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreferences {
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_locale() -> String {
    "en-US".to_string()
}

fn default_theme() -> String {
    "Midnight Indigo".to_string()
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            locale: default_locale(),
            theme: default_theme(),
        }
    }
}

impl UiPreferences {
    fn sanitize(&mut self) {
        if self.locale.trim().is_empty() {
            self.locale = default_locale();
        }
        if self.theme.trim().is_empty() {
            self.theme = default_theme();
        }
    }
}

#[derive(Debug)]
pub struct PreferencesStore {
    path: PathBuf,
    data: Preferences,
}

impl PreferencesStore {
    pub fn new(path: impl Into<PathBuf>, preferences: Preferences) -> Self {
        Self {
            path: path.into(),
            data: preferences,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, PreferencesError> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            let mut data = Preferences::default();
            data.sanitize();
            return Ok(Self { path, data });
        }

        let contents = fs::read_to_string(&path).map_err(|source| PreferencesError::Read {
            path: path.clone(),
            source,
        })?;
        let mut data: Preferences =
            serde_json::from_str(&contents).map_err(|source| PreferencesError::Parse {
                path: path.clone(),
                source,
            })?;
        data.sanitize();
        Ok(Self { path, data })
    }

    pub fn preferences(&self) -> &Preferences {
        &self.data
    }

    pub fn preferences_mut(&mut self) -> &mut Preferences {
        &mut self.data
    }

    pub fn update<F>(&mut self, mut op: F) -> Result<(), PreferencesError>
    where
        F: FnMut(&mut Preferences),
    {
        op(&mut self.data);
        self.data.sanitize();
        self.save()
    }

    pub fn overwrite(&mut self, preferences: Preferences) -> Result<(), PreferencesError> {
        self.data = preferences;
        self.data.sanitize();
        self.save()
    }

    pub fn save(&self) -> Result<(), PreferencesError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| PreferencesError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let payload = serde_json::to_string_pretty(&self.data).map_err(|source| {
            PreferencesError::Serialize {
                path: self.path.clone(),
                source,
            }
        })?;

        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, payload.as_bytes()).map_err(|source| PreferencesError::Write {
            path: tmp_path.clone(),
            source,
        })?;
        fs::rename(&tmp_path, &self.path).map_err(|source| PreferencesError::Write {
            path: self.path.clone(),
            source,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn export_to(&self, path: impl AsRef<Path>) -> Result<(), PreferencesError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| PreferencesError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let payload = serde_json::to_string_pretty(&self.data).map_err(|source| {
            PreferencesError::Serialize {
                path: path.clone(),
                source,
            }
        })?;
        fs::write(&path, payload.as_bytes())
            .map_err(|source| PreferencesError::Write { path, source })
    }

    pub fn import_from(&mut self, source: impl AsRef<Path>) -> Result<(), PreferencesError> {
        let source = source.as_ref().to_path_buf();
        let contents = fs::read_to_string(&source).map_err(|err| PreferencesError::Read {
            path: source.clone(),
            source: err,
        })?;
        let mut data: Preferences =
            serde_json::from_str(&contents).map_err(|err| PreferencesError::Parse {
                path: source.clone(),
                source: err,
            })?;
        data.sanitize();
        self.backup_existing()?;
        self.data = data;
        self.save()
    }

    fn backup_existing(&self) -> Result<(), PreferencesError> {
        if self.path.exists() {
            let backup = self.path.with_extension("bak");
            if let Some(parent) = backup.parent() {
                fs::create_dir_all(parent).map_err(|source| PreferencesError::CreateDir {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(&self.path, &backup).map_err(|source| PreferencesError::Write {
                path: backup,
                source,
            })?;
        }
        Ok(())
    }
}
