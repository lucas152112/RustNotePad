//! Discovery and ABI bridge helpers for classic Notepad++ DLL plugins.
//! 掃描並橋接 Notepad++ DLL 外掛的工具集合。

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(target_os = "windows")]
use {libloading::Library, std::ffi::c_void, std::mem, std::os::raw::c_int, std::slice};

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

    #[cfg(target_os = "windows")]
    #[error("plugin {path} must declare Unicode support via isUnicode()")]
    /// 外掛未宣告 Unicode 支援。
    UnicodeRequired { path: PathBuf },

    #[cfg(target_os = "windows")]
    #[error("plugin {path} returned invalid UTF-16 for {field}: {reason}")]
    /// 外掛回傳的 UTF-16 字串無效。
    InvalidUtf16 {
        path: PathBuf,
        field: &'static str,
        reason: String,
    },

    #[cfg(target_os = "windows")]
    #[error("plugin {path} did not expose any command items")]
    /// 外掛未提供任何命令項目。
    MissingCommandTable { path: PathBuf },

    #[cfg(target_os = "windows")]
    #[error("plugin {path} returned an invalid command function pointer for '{command}'")]
    /// 外掛命令的函式指標無效。
    NullCommandFunction { path: PathBuf, command: String },

    #[cfg(target_os = "windows")]
    #[error("plugin {path} did not expose a name string")]
    /// 無法取得外掛名稱。
    MissingPluginName { path: PathBuf },
}

/// Window and Scintilla handles exposed to plugins via NppData.
/// 傳遞給外掛的 NppData 窗口控制代碼。
#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NppData {
    pub npp_handle: isize,
    pub scintilla_main_handle: isize,
    pub scintilla_second_handle: isize,
}

/// Safe abstraction over a Notepad++ FuncItem entry.
/// 對 Notepad++ FuncItem 的安全抽象。
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct PluginCommand {
    name: String,
    command_id: i32,
    initially_checked: bool,
    shortcut: Option<Shortcut>,
    callback: PluginCallback,
}

/// Shortcut information attached to a FuncItem.
/// 外掛命令的快捷鍵資訊。
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: u8,
}

/// Wrapper for Windows messages forwarded to plugins.
/// 轉送給外掛的 Windows 訊息封裝。
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowsMessage {
    pub id: u32,
    pub w_param: usize,
    pub l_param: isize,
}

#[cfg(target_os = "windows")]
impl WindowsMessage {
    pub const fn new(id: u32, w_param: usize, l_param: isize) -> Self {
        Self {
            id,
            w_param,
            l_param,
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowsMessage;

/// Common Windows message constants used when talking to plugins.
/// 對外掛傳遞時常用的 Windows 訊息常數。
#[cfg(target_os = "windows")]
pub mod winconst {
    pub const WM_COMMAND: u32 = 0x0111;
    pub const WM_INITDIALOG: u32 = 0x0110;
    pub const WM_NOTIFY: u32 = 0x004E;
    pub const WM_SIZE: u32 = 0x0005;
    pub const NPPM_INTERNAL_REFRESHDARKMODE: u32 = 0x0010_0604;
}

/// Handle representing a loaded Windows plugin.
/// 已載入的 Windows 外掛控制代碼。
#[cfg(target_os = "windows")]
pub struct LoadedPlugin {
    library: Library,
    path: PathBuf,
    exports: PluginExports,
    name: String,
    commands: Vec<PluginCommand>,
    unicode: bool,
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
        let library = unsafe {
            Library::new(path).map_err(|source| LoadError::DllLoad {
                path: path.to_path_buf(),
                source,
            })?
        };
        let exports = PluginExports::load(&library, path)?;
        let unicode = unsafe { (exports.is_unicode)() != 0 };
        if !unicode {
            return Err(LoadError::UnicodeRequired {
                path: path.to_path_buf(),
            });
        }

        let name = unsafe { exports.plugin_name(path)? };
        let commands = unsafe { exports.func_items(path)? };

        Ok(Self {
            library,
            path: path.to_path_buf(),
            exports,
            name,
            commands,
            unicode,
        })
    }

    #[cfg(not(target_os = "windows"))]
    fn load_impl(_path: &Path) -> Result<Self, LoadError> {
        Err(LoadError::UnsupportedPlatform)
    }

    #[cfg(target_os = "windows")]
    /// Returns the friendly plugin name exposed via `getName`.
    /// 取得外掛名稱（`getName` 匯出）。
    pub fn name(&self) -> &str {
        &self.name
    }

    #[cfg(target_os = "windows")]
    /// Provides the list of command descriptors exposed by the plugin.
    /// 回傳外掛宣告的命令描述。
    pub fn commands(&self) -> &[PluginCommand] {
        &self.commands
    }

    #[cfg(target_os = "windows")]
    /// Returns true when the plugin declared Unicode support.
    /// 外掛是否宣告 Unicode 支援。
    pub fn is_unicode(&self) -> bool {
        self.unicode
    }

    #[cfg(target_os = "windows")]
    /// Invokes the underlying `setInfo` export with the provided Notepad++ handles.
    /// 使用給定的 Notepad++ 控制代碼呼叫 `setInfo`。
    pub unsafe fn set_info(&self, data: NppData) {
        (self.exports.set_info)(data);
    }

    #[cfg(target_os = "windows")]
    /// Forwards a Windows message through the plugin's `messageProc`.
    /// 透過外掛的 `messageProc` 轉送 Windows 訊息。
    pub unsafe fn message_proc(&self, message: u32, w_param: usize, l_param: isize) -> isize {
        match self.exports.message_proc {
            Some(proc) => proc(message, w_param, l_param),
            None => 0,
        }
    }

    #[cfg(target_os = "windows")]
    /// Convenience helper for forwarding a structured message.
    /// 以結構化方式轉送訊息。
    pub unsafe fn dispatch_message(&self, message: WindowsMessage) -> isize {
        self.message_proc(message.id, message.w_param, message.l_param)
    }

    #[cfg(target_os = "windows")]
    /// Notifies the plugin about Scintilla/Notepad++ events.
    /// 通知外掛 Scintilla / Notepad++ 事件。
    pub unsafe fn be_notified(&self, notification: *mut c_void) {
        if let Some(callback) = self.exports.be_notified {
            callback(notification);
        }
    }

    #[cfg(target_os = "windows")]
    /// Returns the filesystem path backing this plugin.
    /// 回傳外掛對應的檔案路徑。
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(target_os = "windows")]
type SetInfoFn = unsafe extern "system" fn(NppData);
#[cfg(target_os = "windows")]
type GetNameFn = unsafe extern "system" fn() -> *const u16;
#[cfg(target_os = "windows")]
type GetFuncsArrayFn = unsafe extern "system" fn(*mut c_int) -> *mut FuncItem;
#[cfg(target_os = "windows")]
type BeNotifiedFn = unsafe extern "system" fn(*mut c_void);
#[cfg(target_os = "windows")]
type MessageProcFn = unsafe extern "system" fn(u32, usize, isize) -> isize;
#[cfg(target_os = "windows")]
type IsUnicodeFn = unsafe extern "system" fn() -> i32;

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct PluginExports {
    set_info: SetInfoFn,
    get_name: GetNameFn,
    get_funcs_array: GetFuncsArrayFn,
    be_notified: Option<BeNotifiedFn>,
    message_proc: Option<MessageProcFn>,
    is_unicode: IsUnicodeFn,
}

#[cfg(target_os = "windows")]
impl PluginExports {
    unsafe fn load(library: &Library, path: &Path) -> Result<Self, LoadError> {
        Ok(Self {
            set_info: Self::required(library, path, b"setInfo\0")?,
            get_name: Self::required(library, path, b"getName\0")?,
            get_funcs_array: Self::required(library, path, b"getFuncsArray\0")?,
            be_notified: Self::optional(library, b"beNotified\0")?,
            message_proc: Self::optional(library, b"messageProc\0")?,
            is_unicode: Self::required(library, path, b"isUnicode\0")?,
        })
    }

    unsafe fn required<T>(library: &Library, path: &Path, name: &[u8]) -> Result<T, LoadError> {
        let symbol = library
            .get::<T>(name)
            .map_err(|_| LoadError::MissingExport {
                path: path.to_path_buf(),
                symbol: String::from_utf8_lossy(name).to_string(),
            })?;
        Ok(mem::transmute::<_, T>(symbol.into_raw()))
    }

    unsafe fn optional<T>(library: &Library, name: &[u8]) -> Result<Option<T>, LoadError> {
        match library.get::<T>(name) {
            Ok(symbol) => Ok(Some(mem::transmute::<_, T>(symbol.into_raw()))),
            Err(_) => Ok(None),
        }
    }

    unsafe fn plugin_name(&self, path: &Path) -> Result<String, LoadError> {
        let ptr = (self.get_name)();
        if ptr.is_null() {
            return Err(LoadError::MissingPluginName {
                path: path.to_path_buf(),
            });
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = slice::from_raw_parts(ptr, len);
        decode_utf16(path, "plugin name", slice)
    }

    unsafe fn func_items(&self, path: &Path) -> Result<Vec<PluginCommand>, LoadError> {
        let mut count: c_int = 0;
        let ptr = (self.get_funcs_array)(&mut count);
        if ptr.is_null() || count <= 0 {
            return Err(LoadError::MissingCommandTable {
                path: path.to_path_buf(),
            });
        }
        let items = slice::from_raw_parts(ptr, count as usize);
        let mut commands = Vec::with_capacity(items.len());
        for item in items {
            commands.push(func_item_to_command(path, item)?);
        }
        Ok(commands)
    }
}

#[cfg(target_os = "windows")]
fn func_item_to_command(path: &Path, item: &FuncItem) -> Result<PluginCommand, LoadError> {
    let name = decode_utf16(path, "command name", &item.item_name)?;
    let callback = PluginCallback::new(path, &name, item.p_func)?;
    let shortcut = unsafe { item.shortcut.as_ref().map(Shortcut::from_raw) };
    Ok(PluginCommand {
        name,
        command_id: item.cmd_id,
        initially_checked: item.init_to_check != 0,
        shortcut,
        callback,
    })
}

#[cfg(target_os = "windows")]
impl PluginCommand {
    /// Returns the human readable name advertised by the plugin.
    /// 取得外掛顯示名稱。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Internal command identifier used by the plugin.
    /// 外掛使用的命令 ID。
    pub fn command_id(&self) -> i32 {
        self.command_id
    }

    /// Whether the command should be checked on first show.
    /// 命令是否預設為勾選狀態。
    pub fn initially_checked(&self) -> bool {
        self.initially_checked
    }

    /// Shortcut defined by the plugin, if any.
    /// 外掛定義的快捷鍵（若有）。
    pub fn shortcut(&self) -> Option<&Shortcut> {
        self.shortcut.as_ref()
    }

    /// Invokes the command callback inside the plugin DLL.
    /// 呼叫外掛命令對應的函式。
    pub unsafe fn invoke(&self) {
        (self.callback.ptr)();
    }
}

#[cfg(target_os = "windows")]
impl Shortcut {
    unsafe fn from_raw(raw: &ShortcutKey) -> Self {
        Self {
            ctrl: raw.is_ctrl != 0,
            alt: raw.is_alt != 0,
            shift: raw.is_shift != 0,
            key: raw.key,
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct PluginCallback {
    ptr: unsafe extern "system" fn(),
}

#[cfg(target_os = "windows")]
impl PluginCallback {
    fn new(
        path: &Path,
        command: &str,
        ptr: unsafe extern "system" fn(),
    ) -> Result<Self, LoadError> {
        let address = ptr as *const ();
        if address.is_null() {
            return Err(LoadError::NullCommandFunction {
                path: path.to_path_buf(),
                command: command.to_string(),
            });
        }
        Ok(Self { ptr })
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

#[cfg(target_os = "windows")]
fn decode_utf16(path: &Path, field: &'static str, buf: &[u16]) -> Result<String, LoadError> {
    let terminator = buf.iter().position(|&ch| ch == 0).unwrap_or(buf.len());
    String::from_utf16(&buf[..terminator]).map_err(|err| LoadError::InvalidUtf16 {
        path: path.to_path_buf(),
        field,
        reason: err.to_string(),
    })
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct FuncItem {
    item_name: [u16; 64],
    p_func: unsafe extern "system" fn(),
    cmd_id: c_int,
    init_to_check: c_int,
    shortcut: *mut ShortcutKey,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
struct ShortcutKey {
    is_ctrl: u8,
    is_alt: u8,
    is_shift: u8,
    key: u8,
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

    #[cfg(target_os = "windows")]
    #[test]
    fn shortcut_conversion_matches_flags() {
        let raw = ShortcutKey {
            is_ctrl: 1,
            is_alt: 0,
            is_shift: 1,
            key: 0x41,
        };
        let shortcut = unsafe { Shortcut::from_raw(&raw) };
        assert!(shortcut.ctrl);
        assert!(!shortcut.alt);
        assert!(shortcut.shift);
        assert_eq!(shortcut.key, 0x41);
    }
}
