//! WASM plugin discovery and manifest validation for RustNotePad.
//! RustNotePad 的 WASM 外掛掃描與清單驗證。

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default manifest file name located inside each plugin directory.
/// 每個外掛資料夾中的預設清單檔名。
pub const MANIFEST_FILE: &str = "plugin.json";

/// Default sub directory under the workspace that holds WASM plugins.
/// 工作區中存放 WASM 外掛的預設子目錄。
pub const DEFAULT_RELATIVE_ROOT: &str = "plugins/wasm";

/// Capabilities a plugin can request from the host.
/// 外掛可向主程式請求的能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Read buffers and metadata from open documents.
    /// 讀取開啟中文件與其後設資料。
    BufferRead,
    /// Modify buffers and save contents back to disk.
    /// 修改文件內容並寫回磁碟。
    BufferWrite,
    /// Register commands surfaced in the command palette or menus.
    /// 在指令集合或選單中註冊命令。
    RegisterCommand,
    /// Create custom UI panels or tool windows.
    /// 建立自訂 UI 面板或工具視窗。
    UiPanels,
    /// Subscribe to editor events (document opened, saved, etc.).
    /// 訂閱編輯器事件（開啟、儲存等）。
    EventSubscriptions,
    /// Read files from the filesystem (outside of explicit buffers).
    /// 讀取檔案系統中的其他檔案。
    FileSystemRead,
    /// Write files to the filesystem.
    /// 寫入檔案系統。
    FileSystemWrite,
    /// Perform outbound network requests.
    /// 發出對外網路請求。
    NetworkAccess,
}

impl Capability {
    /// Capabilities considered safe by default.
    /// 預設視為安全的能力清單。
    pub fn default_safe() -> &'static [Capability] {
        use Capability::*;
        &[BufferRead, RegisterCommand, UiPanels, EventSubscriptions]
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Capability::BufferRead => "buffer-read",
                Capability::BufferWrite => "buffer-write",
                Capability::RegisterCommand => "register-command",
                Capability::UiPanels => "ui-panels",
                Capability::EventSubscriptions => "event-subscriptions",
                Capability::FileSystemRead => "fs-read",
                Capability::FileSystemWrite => "fs-write",
                Capability::NetworkAccess => "network",
            }
        )
    }
}

/// Manifest metadata declared by a plugin author.
/// 外掛作者宣告的清單後設資料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Stable identifier, e.g. `dev.rustnotepad.hello`.
    /// 穩定識別碼，如 `dev.rustnotepad.hello`。
    pub id: String,
    /// Friendly name for the plugin.
    /// 外掛顯示名稱。
    pub name: String,
    /// Human-facing description.
    /// 提供給使用者的描述。
    #[serde(default)]
    pub description: Option<String>,
    /// Semantic version string.
    /// 語意化版本字串。
    pub version: String,
    /// Relative path to the WASM module within the plugin directory.
    /// 指向外掛資料夾內 WASM 模組的相對路徑。
    pub entry: String,
    /// Capabilities requested by the plugin.
    /// 外掛要求的能力。
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Optional minimum host version required.
    /// 選填的主程式最低版本需求。
    #[serde(default)]
    pub minimum_host_version: Option<String>,
    /// Declared commands exposed by the plugin.
    /// 外掛提供的命令清單。
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
}

/// Describes a command registered by a plugin.
/// 外掛註冊的命令描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCommand {
    /// Stable identifier used for invocation.
    /// 供呼叫使用的穩定識別碼。
    pub id: String,
    /// Human readable display name.
    /// 顯示於介面的名稱。
    pub name: String,
    /// Optional description explaining behaviour.
    /// 說明命令用途的描述（可選）。
    #[serde(default)]
    pub description: Option<String>,
}

impl PluginCommand {
    /// Validates command metadata.
    /// 驗證命令後設資料。
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("command id cannot be empty".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("command name cannot be empty".to_string());
        }
        Ok(())
    }
}

impl PluginManifest {
    /// Validates manifest invariants, returning an error string on failure.
    /// 驗證清單內容的合法性，失敗時回傳錯誤字串。
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("manifest id cannot be empty".to_string());
        }
        if !self
            .id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || matches!(ch, '.' | '-' | '_' | '0'..='9'))
        {
            return Err("manifest id must use lowercase ASCII plus '.', '-', '_'".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("manifest name cannot be empty".to_string());
        }
        if self.version.trim().is_empty() {
            return Err("manifest version cannot be empty".to_string());
        }
        if self.entry.trim().is_empty() {
            return Err("manifest entry cannot be empty".to_string());
        }
        if self.entry.contains("..") {
            return Err("manifest entry cannot contain parent components ('..')".to_string());
        }
        if Path::new(&self.entry).is_absolute() {
            return Err("manifest entry must be a relative path".to_string());
        }
        for command in &self.commands {
            command
                .validate()
                .map_err(|reason| format!("invalid command metadata: {reason}"))?;
        }
        Ok(())
    }

    fn module_path(&self, plugin_root: &Path) -> PathBuf {
        plugin_root.join(&self.entry)
    }
}

/// Result of loading a plugin manifest and resolving its module path.
/// 成功載入外掛清單後的封包資訊。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmPluginPackage {
    pub manifest: PluginManifest,
    pub root_dir: PathBuf,
    pub module_path: PathBuf,
}

impl WasmPluginPackage {
    /// Creates a new package from a manifest and root directory.
    /// 依據清單與根目錄建立新的套件。
    pub fn new(manifest: PluginManifest, root_dir: PathBuf) -> Result<Self, WasmPluginError> {
        let module_path = manifest.module_path(&root_dir);
        if !module_path.exists() {
            return Err(WasmPluginError::ModuleMissing(module_path));
        }
        Ok(Self {
            manifest,
            root_dir,
            module_path,
        })
    }
}

/// Policy controlling which capabilities are permitted.
/// 控制允許哪些能力的策略。
#[derive(Debug, Clone)]
pub struct CapabilityPolicy {
    allowed: BTreeSet<Capability>,
}

impl CapabilityPolicy {
    /// Construct a policy that allows only the provided capabilities.
    /// 建立僅允許指定能力的策略。
    pub fn allow_only<I: IntoIterator<Item = Capability>>(caps: I) -> Self {
        Self {
            allowed: caps.into_iter().collect(),
        }
    }

    /// Minimum privilege policy used by default host configuration.
    /// 主程式預設採用的最低權限策略。
    pub fn locked_down() -> Self {
        Self::allow_only(Capability::default_safe().iter().copied())
    }

    /// Returns true when a capability is allowed.
    /// 回報某能力是否被允許。
    pub fn allows(&self, capability: Capability) -> bool {
        self.allowed.contains(&capability)
    }

    /// Validates requested capabilities against the policy.
    /// 依策略檢查外掛要求的能力。
    pub fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), WasmPluginError> {
        for capability in &manifest.capabilities {
            if !self.allows(*capability) {
                return Err(WasmPluginError::CapabilityDenied(*capability));
            }
        }
        Ok(())
    }
}

impl Default for CapabilityPolicy {
    fn default() -> Self {
        Self::locked_down()
    }
}

/// Errors emitted while loading plugin manifests/packages.
/// 掃描或載入外掛清單時可能出現的錯誤。
#[derive(Debug, Error)]
pub enum WasmPluginError {
    /// 無法讀取外掛清單檔案。
    #[error("failed to read manifest {0}")]
    ManifestRead(PathBuf, #[source] std::io::Error),
    /// 無法解析外掛清單內容。
    #[error("failed to parse manifest {0}")]
    ManifestParse(PathBuf, #[source] serde_json::Error),
    /// 外掛清單內容不合法。
    #[error("invalid manifest {0}: {1}")]
    ManifestInvalid(PathBuf, String),
    /// 外掛要求未被允許的能力。
    #[error("requested capability '{0}' is not allowed")]
    CapabilityDenied(Capability),
    /// 清單指向的 WASM 模組不存在。
    #[error("referenced wasm module missing: {0}")]
    ModuleMissing(PathBuf),
}

/// Non-fatal failure encountered during discovery.
/// 掃描過程中遭遇的非致命失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginLoadFailure {
    pub path: PathBuf,
    pub message: String,
}

/// Successful discovery result containing loaded plugins and warnings.
/// 掃描完成後的外掛清單與警示。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventory {
    pub plugins: Vec<WasmPluginPackage>,
    pub failures: Vec<PluginLoadFailure>,
}

impl Inventory {
    /// Returns true when no plugins were loaded and no failures occurred.
    /// 若無載入外掛亦無失敗，則回傳 `true`。
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty() && self.failures.is_empty()
    }
}

/// Discovers plugins from the given directory using the supplied policy.
/// 以指定策略掃描某個目錄下的外掛。
pub fn discover(root_dir: &Path, policy: &CapabilityPolicy) -> Result<Inventory, std::io::Error> {
    let mut inventory = Inventory::default();
    if !root_dir.exists() {
        return Ok(inventory);
    }
    let read_dir = match fs::read_dir(root_dir) {
        Ok(iter) => iter,
        Err(err) => return Err(err),
    };
    for entry in read_dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                inventory.failures.push(PluginLoadFailure {
                    path: root_dir.to_path_buf(),
                    message: format!("failed to read entry: {err}"),
                });
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match load_plugin_dir(&path, policy) {
            Ok(package) => inventory.plugins.push(package),
            Err(err) => inventory.failures.push(PluginLoadFailure {
                path,
                message: err.to_string(),
            }),
        }
    }
    Ok(inventory)
}

fn load_plugin_dir(
    dir: &Path,
    policy: &CapabilityPolicy,
) -> Result<WasmPluginPackage, WasmPluginError> {
    let manifest_path = dir.join(MANIFEST_FILE);
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|err| WasmPluginError::ManifestRead(manifest_path.clone(), err))?;
    let manifest: PluginManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| WasmPluginError::ManifestParse(manifest_path.clone(), err))?;
    manifest
        .validate()
        .map_err(|reason| WasmPluginError::ManifestInvalid(manifest_path.clone(), reason))?;
    policy.validate_manifest(&manifest)?;
    WasmPluginPackage::new(manifest, dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_manifest(dir: &Path, manifest: &PluginManifest) {
        let manifest_path = dir.join(MANIFEST_FILE);
        let json = serde_json::to_string_pretty(manifest).expect("serialize manifest");
        fs::write(manifest_path, json).expect("write manifest");
        let module_path = dir.join(&manifest.entry);
        if let Some(parent) = module_path.parent() {
            fs::create_dir_all(parent).expect("create module dir");
        }
        fs::write(module_path, b"\0asm").expect("write wasm stub");
    }

    fn base_manifest() -> PluginManifest {
        PluginManifest {
            id: "dev.rustnotepad.hello".into(),
            name: "Hello Plugin".into(),
            description: Some("Sample plugin".into()),
            version: "0.1.0".into(),
            entry: "hello.wasm".into(),
            capabilities: vec![Capability::BufferRead, Capability::RegisterCommand],
            minimum_host_version: Some("0.1.0".into()),
            commands: vec![PluginCommand {
                id: "hello.run".into(),
                name: "Run Hello".into(),
                description: Some("Prints a hello message".into()),
            }],
        }
    }

    #[test]
    fn manifest_validation_rejects_invalid_entries() {
        let mut manifest = base_manifest();
        manifest.entry = "../escape.wasm".into();
        assert!(manifest.validate().is_err());

        let mut manifest = base_manifest();
        manifest.id = "Invalid Id".into();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn discovery_returns_loaded_plugins() {
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join("plugins");
        fs::create_dir_all(&plugin_dir).unwrap();

        let plugin_a = plugin_dir.join("a");
        fs::create_dir(&plugin_a).unwrap();
        let manifest_a = base_manifest();
        write_manifest(&plugin_a, &manifest_a);

        let plugin_b = plugin_dir.join("b");
        fs::create_dir(&plugin_b).unwrap();
        let mut manifest_b = base_manifest();
        manifest_b.id = "dev.rustnotepad.goodbye".into();
        manifest_b.entry = "pkg/main.wasm".into();
        write_manifest(&plugin_b, &manifest_b);

        let policy = CapabilityPolicy::allow_only(Capability::default_safe().iter().copied());
        let inventory = discover(&plugin_dir, &policy).unwrap();
        assert_eq!(inventory.plugins.len(), 2);
        assert!(inventory.failures.is_empty());
        let ids: Vec<_> = inventory
            .plugins
            .iter()
            .map(|pkg| pkg.manifest.id.as_str())
            .collect();
        assert!(ids.contains(&"dev.rustnotepad.hello"));
        assert!(ids.contains(&"dev.rustnotepad.goodbye"));
    }

    #[test]
    fn capability_policy_blocks_forbidden_entries() {
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join("plugins");
        fs::create_dir_all(&plugin_dir).unwrap();
        let plugin_a = plugin_dir.join("a");
        fs::create_dir(&plugin_a).unwrap();
        let mut manifest = base_manifest();
        manifest.capabilities.push(Capability::FileSystemWrite);
        write_manifest(&plugin_a, &manifest);

        let inventory = discover(&plugin_dir, &CapabilityPolicy::locked_down()).unwrap();
        assert!(inventory.plugins.is_empty());
        assert_eq!(inventory.failures.len(), 1);
        assert!(
            inventory.failures[0]
                .message
                .contains("capability 'fs-write'"),
            "expected capability denial message"
        );
    }

    #[test]
    fn missing_module_reports_failure() {
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join("plugins");
        fs::create_dir_all(&plugin_dir).unwrap();
        let plugin_a = plugin_dir.join("a");
        fs::create_dir(&plugin_a).unwrap();
        let manifest = base_manifest();
        let manifest_path = plugin_a.join(MANIFEST_FILE);
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let inventory = discover(&plugin_dir, &CapabilityPolicy::locked_down()).unwrap();
        assert!(inventory.plugins.is_empty());
        assert_eq!(inventory.failures.len(), 1);
        assert!(
            inventory.failures[0]
                .message
                .contains("referenced wasm module missing"),
            "expected missing module message"
        );
    }
}
