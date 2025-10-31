//! Lightweight helpers for discovering Notepad++-style DLL plugins.
//! 掃描符合 Notepad++ 外掛格式之 DLL 的輕量工具。
//!
//! Actual ABI bridging is implemented elsewhere; this crate focuses on
//! filesystem discovery and metadata collection that is independent from the
//! Windows loader. This keeps the GUI code portable while still reporting which
//! plugins are present on disk.
//! 實際的 ABI 橋接另由其他元件負責，本 crate 僅處理與 Windows 載入器無關的檔案掃描與後設資料，
//! 讓 GUI 在各平台仍能顯示磁碟上的外掛狀態。

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(target_os = "windows")]
use libloading::Library;

/// File extension used by Windows plugins.
/// Windows 外掛採用的檔案副檔名。
pub const PLUGIN_EXTENSION: &str = "dll";

/// Default location for DLL plugins relative to the workspace root.
/// 工作區根目錄下 DLL 外掛的預設路徑。
pub const DEFAULT_RELATIVE_ROOT: &str = "plugins/win32";

/// High level classification of discovered plugin artifacts.
/// 外掛資源的高層分類。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginDescriptor {
    /// A valid DLL file that could be loaded by the ABI bridge.
    /// 可供 ABI 橋接載入的 DLL。
    Binary { path: PathBuf, file_size: u64 },
    /// A dangling metadata folder without an actual DLL present.
    /// 僅含後設資料但缺少 DLL 的資料夾。
    MetadataOnly { path: PathBuf },
}

impl PluginDescriptor {
    /// Returns the on-disk path backing this descriptor.
    /// 回傳描述項目對應的檔案路徑。
    pub fn path(&self) -> &Path {
        match self {
            PluginDescriptor::Binary { path, .. } => path,
            PluginDescriptor::MetadataOnly { path } => path,
        }
    }
}

impl fmt::Display for PluginDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginDescriptor::Binary { path, .. } => write!(f, "{}", path.display()),
            PluginDescriptor::MetadataOnly { path } => {
                write!(f, "{} (metadata only)", path.display())
            }
        }
    }
}

/// Failure emitted when scanning the plugin directory.
/// 掃描外掛資料夾時可能出現的錯誤。
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("failed to read directory {0}")]
    /// 無法讀取外掛目錄。
    DirectoryRead(PathBuf, #[source] std::io::Error),
}

/// Scans a directory for potential Notepad++ plugins.
/// 掃描目錄以尋找 Notepad++ 類型外掛。
pub fn discover(root_dir: &Path) -> Result<Vec<PluginDescriptor>, DiscoveryError> {
    if !root_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(root_dir)
        .map_err(|err| DiscoveryError::DirectoryRead(root_dir.to_path_buf(), err))?;
    let mut descriptors = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Allow plugins to live inside a directory that matches the DLL name.
            // 允許外掛以與 DLL 名稱相同的資料夾做為容器。
            let dll_name = format!(
                "{}.{}",
                path.file_name().and_then(OsStr::to_str).unwrap_or_default(),
                PLUGIN_EXTENSION
            );
            let candidate = path.join(&dll_name);
            if candidate.exists() {
                descriptors.push(describe_binary(candidate));
            } else {
                descriptors.push(PluginDescriptor::MetadataOnly { path });
            }
            continue;
        }

        if is_dll(&path) {
            descriptors.push(describe_binary(path));
        }
    }

    Ok(descriptors)
}

/// Errors that can occur while loading a Windows plugin.
/// 載入 Windows 外掛時可能發生的錯誤。
#[derive(Debug, Error)]
pub enum LoadError {
    #[cfg(not(target_os = "windows"))]
    #[error("Windows plugin loading is not supported on this platform")]
    /// 目前平台不支援載入 Windows 外掛。
    UnsupportedPlatform,

    #[cfg(target_os = "windows")]
    #[error("failed to load DLL {path}: {source}")]
    /// 載入 DLL 檔案失敗。
    DllLoad {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },

    #[cfg(target_os = "windows")]
    #[error("export '{symbol}' missing in plugin {path}")]
    /// 外掛缺少必要的匯出項目。
    MissingExport { path: PathBuf, symbol: String },
}

/// Handle representing a loaded Windows plugin.
/// 已載入的 Windows 外掛控制代碼。
#[cfg_attr(target_os = "windows", derive(Debug))]
#[cfg_attr(not(target_os = "windows"), derive(Debug))]
#[cfg(target_os = "windows")]
pub struct LoadedPlugin {
    library: Library,
    path: PathBuf,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub struct LoadedPlugin;

impl LoadedPlugin {
    /// Loads a plugin DLL into memory when supported.
    /// 在支援的平台上載入 DLL 外掛。
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        Self::load_impl(path)
    }

    #[cfg(target_os = "windows")]
    fn load_impl(path: &Path) -> Result<Self, LoadError> {
        unsafe {
            Library::new(path)
                .map(|library| LoadedPlugin {
                    library,
                    path: path.to_path_buf(),
                })
                .map_err(|source| LoadError::DllLoad {
                    path: path.to_path_buf(),
                    source,
                })
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn load_impl(_path: &Path) -> Result<Self, LoadError> {
        Err(LoadError::UnsupportedPlatform)
    }

    /// Gets a raw symbol from the DLL when available.
    /// 取得 DLL 匯出的原始符號。
    #[cfg(target_os = "windows")]
    pub unsafe fn symbol<T>(&self, name: &[u8]) -> Result<libloading::Symbol<T>, LoadError> {
        self.library
            .get(name)
            .map_err(|_| LoadError::MissingExport {
                path: self.path.clone(),
                symbol: String::from_utf8_lossy(name).to_string(),
            })
    }
}

fn is_dll(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|ext| ext.eq_ignore_ascii_case(PLUGIN_EXTENSION))
        .unwrap_or(false)
}

fn describe_binary(path: PathBuf) -> PluginDescriptor {
    let file_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    PluginDescriptor::Binary { path, file_size }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn discovers_root_level_dll() {
        let temp = tempdir().unwrap();
        let dll_path = temp.path().join("SamplePlugin.dll");
        fs::File::create(&dll_path).unwrap();

        let descriptors = discover(temp.path()).unwrap();
        assert_eq!(descriptors.len(), 1);
        match &descriptors[0] {
            PluginDescriptor::Binary { path, .. } => assert_eq!(path, &dll_path),
            other => panic!("unexpected descriptor: {other:?}"),
        }
    }

    #[test]
    fn discovers_directory_wrapped_plugin() {
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join("SamplePlugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        let dll_path = plugin_dir.join("SamplePlugin.dll");
        let mut file = fs::File::create(&dll_path).unwrap();
        writeln!(file, "dll stub").unwrap();

        let descriptors = discover(temp.path()).unwrap();
        assert_eq!(descriptors.len(), 1);
        match &descriptors[0] {
            PluginDescriptor::Binary { path, .. } => assert_eq!(path, &dll_path),
            other => panic!("unexpected descriptor: {other:?}"),
        }
    }

    #[test]
    fn reports_metadata_only_directories() {
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join("MetadataOnly");
        fs::create_dir_all(&plugin_dir).unwrap();

        let descriptors = discover(temp.path()).unwrap();
        assert_eq!(descriptors.len(), 1);
        match &descriptors[0] {
            PluginDescriptor::MetadataOnly { path } => assert_eq!(path, &plugin_dir),
            other => panic!("unexpected descriptor: {other:?}"),
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn loading_plugins_is_unsupported_on_non_windows() {
        let err = LoadedPlugin::load(Path::new("SamplePlugin.dll")).unwrap_err();
        assert!(matches!(err, LoadError::UnsupportedPlatform));
    }
}
