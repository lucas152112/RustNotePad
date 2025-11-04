//! WASM plugin discovery and manifest validation for RustNotePad.
//! RustNotePad 的 WASM 外掛掃描與清單驗證。

use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
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

/// Default signature file stored next to the manifest.
/// 清單旁預設的簽章檔案名稱。
pub const SIGNATURE_FILE: &str = "signature.json";

/// Built-in signer identifier used for first-party plugins.
/// 官方外掛預設簽署者的識別碼。
pub const DEFAULT_SIGNER_ID: &str = "rustnotepad.dev";

const DEFAULT_SIGNER_KEY: [u8; 32] = [
    0x24, 0x2d, 0x12, 0xe3, 0x84, 0x3c, 0xa8, 0x40, 0xe9, 0x99, 0x50, 0x59, 0x19, 0xe1, 0xce, 0x0e,
    0xca, 0xb4, 0x5d, 0xf5, 0x57, 0xf3, 0xed, 0xb4, 0x4d, 0xf9, 0x57, 0x81, 0xec, 0x37, 0xa5, 0xb2,
];

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
    pub trust: PluginTrust,
}

impl WasmPluginPackage {
    /// Creates a new package from a manifest and root directory.
    /// 依據清單與根目錄建立新的套件。
    pub fn new(
        manifest: PluginManifest,
        root_dir: PathBuf,
        trust: PluginTrust,
    ) -> Result<Self, WasmPluginError> {
        let module_path = manifest.module_path(&root_dir);
        if !module_path.exists() {
            return Err(WasmPluginError::ModuleMissing(module_path));
        }
        Ok(Self {
            manifest,
            root_dir,
            module_path,
            trust,
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

/// Trust state associated with a loaded plugin package.
/// 外掛套件的信任狀態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginTrust {
    /// Signature verified against a trusted signer.
    /// 已通過信任簽章驗證。
    Trusted { signer: String },
    /// Plugin allowed even though it lacks a signature (policy permits it).
    /// 策略允許的未簽章外掛。
    Unsigned,
}

impl PluginTrust {
    /// Returns the signer identifier when the plugin is trusted.
    /// 回傳已驗證簽章的簽署者識別碼（若有）。
    pub fn signer(&self) -> Option<&str> {
        match self {
            PluginTrust::Trusted { signer } => Some(signer.as_str()),
            PluginTrust::Unsigned => None,
        }
    }
}

/// Trusted signer registry definition.
/// 信任簽署者的登錄資訊。
#[derive(Debug, Clone)]
pub struct TrustedSigner {
    pub id: String,
    pub key: VerifyingKey,
}

impl TrustedSigner {
    /// Creates a new trusted signer entry.
    /// 建立新的信任簽署者條目。
    pub fn new(id: impl Into<String>, key: VerifyingKey) -> Self {
        Self { id: id.into(), key }
    }
}

/// Policy governing plugin signature requirements.
/// 控制外掛簽章要求的策略。
#[derive(Debug, Clone)]
pub struct TrustPolicy {
    trusted_signers: HashMap<String, VerifyingKey>,
    allow_unsigned: bool,
}

impl TrustPolicy {
    /// Creates a strict policy that requires a valid signature from the supplied signers.
    /// 建立嚴格策略，僅接受已註冊簽署者的有效簽章。
    pub fn strict(signers: impl IntoIterator<Item = TrustedSigner>) -> Self {
        let trusted_signers = signers
            .into_iter()
            .map(|signer| (signer.id, signer.key))
            .collect();
        Self {
            trusted_signers,
            allow_unsigned: false,
        }
    }

    /// Creates a policy that allows unsigned plugins (useful for development).
    /// 建立允許未簽章外掛的策略（適用於開發階段）。
    pub fn allow_unsigned(signers: impl IntoIterator<Item = TrustedSigner>) -> Self {
        let trusted_signers = signers
            .into_iter()
            .map(|signer| (signer.id, signer.key))
            .collect();
        Self {
            trusted_signers,
            allow_unsigned: true,
        }
    }

    /// Returns the default strict policy with built-in RustNotePad signers.
    /// 回傳預設的嚴格策略與內建 RustNotePad 簽署者。
    pub fn release_defaults() -> Self {
        let key = VerifyingKey::from_bytes(&DEFAULT_SIGNER_KEY).expect("valid built-in key");
        Self::strict([TrustedSigner::new(DEFAULT_SIGNER_ID, key)])
    }

    /// Adds or replaces a trusted signer entry.
    /// 新增或取代信任簽署者資料。
    pub fn insert_signer(&mut self, signer: TrustedSigner) {
        self.trusted_signers.insert(signer.id, signer.key);
    }

    /// Returns true when the policy permits unsigned plugins.
    /// 是否允許未簽章的外掛。
    pub fn allows_unsigned(&self) -> bool {
        self.allow_unsigned
    }

    /// Evaluates signature metadata and returns the resulting trust state.
    /// 驗證簽章並回傳信任狀態。
    pub fn evaluate(
        &self,
        plugin_dir: &Path,
        manifest_bytes: &[u8],
    ) -> Result<PluginTrust, WasmPluginError> {
        let signature_path = plugin_dir.join(SIGNATURE_FILE);
        let bytes = match fs::read(&signature_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return if self.allow_unsigned {
                    Ok(PluginTrust::Unsigned)
                } else {
                    Err(WasmPluginError::SignatureMissing(signature_path))
                };
            }
            Err(err) => {
                return Err(WasmPluginError::SignatureRead(signature_path, err));
            }
        };

        let metadata: SignatureMetadata = serde_json::from_slice(&bytes)
            .map_err(|err| WasmPluginError::SignatureParse(signature_path.clone(), err))?;

        let algorithm = metadata.algorithm.to_ascii_lowercase();
        if algorithm != "ed25519" {
            return Err(WasmPluginError::SignatureUnsupportedAlgorithm {
                algorithm: metadata.algorithm,
            });
        }

        let signer_key = self.trusted_signers.get(&metadata.signer).ok_or_else(|| {
            WasmPluginError::SignatureUntrusted {
                signer: metadata.signer.clone(),
            }
        })?;

        let signature_bytes = Base64
            .decode(metadata.signature.as_bytes())
            .map_err(|err| WasmPluginError::SignatureInvalid {
                signer: metadata.signer.clone(),
                reason: format!("invalid base64: {err}"),
            })?;
        if signature_bytes.len() != Signature::BYTE_SIZE {
            return Err(WasmPluginError::SignatureInvalid {
                signer: metadata.signer.clone(),
                reason: format!(
                    "expected {} bytes, got {}",
                    Signature::BYTE_SIZE,
                    signature_bytes.len()
                ),
            });
        }
        let signature = Signature::from_slice(&signature_bytes).map_err(|err| {
            WasmPluginError::SignatureInvalid {
                signer: metadata.signer.clone(),
                reason: err.to_string(),
            }
        })?;

        signer_key
            .verify(manifest_bytes, &signature)
            .map_err(|err| WasmPluginError::SignatureInvalid {
                signer: metadata.signer.clone(),
                reason: err.to_string(),
            })?;

        Ok(PluginTrust::Trusted {
            signer: metadata.signer,
        })
    }
}

#[derive(Debug, Deserialize)]
struct SignatureMetadata {
    #[serde(default = "default_signature_version")]
    _version: u32,
    signer: String,
    #[serde(default = "default_signature_algorithm")]
    algorithm: String,
    signature: String,
}

fn default_signature_version() -> u32 {
    1
}

fn default_signature_algorithm() -> String {
    "ed25519".to_string()
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
    /// 無法讀取外掛簽章資料。
    #[error("failed to read signature file {0}")]
    SignatureRead(PathBuf, #[source] std::io::Error),
    /// 無法解析外掛簽章內容。
    #[error("failed to parse signature file {0}")]
    SignatureParse(PathBuf, #[source] serde_json::Error),
    /// 外掛缺少必要簽章。
    #[error("signature required but missing: {0}")]
    SignatureMissing(PathBuf),
    /// 外掛簽署者不在信任清單中。
    #[error("plugin signer '{signer}' is not trusted")]
    SignatureUntrusted { signer: String },
    /// 外掛簽章驗證失敗。
    #[error("signature verification failed for signer '{signer}': {reason}")]
    SignatureInvalid { signer: String, reason: String },
    /// 遇到不支援的簽章演算法。
    #[error("unsupported signature algorithm '{algorithm}'")]
    SignatureUnsupportedAlgorithm { algorithm: String },
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
pub fn discover(
    root_dir: &Path,
    policy: &CapabilityPolicy,
    trust: &TrustPolicy,
) -> Result<Inventory, std::io::Error> {
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
        match load_plugin_dir(&path, policy, trust) {
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
    trust: &TrustPolicy,
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
    let trust_state = trust.evaluate(dir, &manifest_bytes)?;
    WasmPluginPackage::new(manifest, dir.to_path_buf(), trust_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::fs;
    use tempfile::tempdir;

    const TEST_SIGNING_KEY: [u8; 32] = [
        0xff, 0x89, 0x5e, 0xee, 0x69, 0x21, 0xfb, 0x40, 0x0c, 0x92, 0xf6, 0x0d, 0xd9, 0x5f, 0x97,
        0x01, 0xcb, 0xa0, 0xfa, 0xc6, 0x77, 0x6b, 0x7b, 0x30, 0x88, 0xdf, 0xe6, 0xcd, 0xbb, 0xa4,
        0x32, 0xcf,
    ];

    fn write_manifest(dir: &Path, manifest: &PluginManifest) {
        let manifest_path = dir.join(MANIFEST_FILE);
        let json = serde_json::to_string_pretty(manifest).expect("serialize manifest");
        fs::write(&manifest_path, &json).expect("write manifest");
        let module_path = dir.join(&manifest.entry);
        if let Some(parent) = module_path.parent() {
            fs::create_dir_all(parent).expect("create module dir");
        }
        fs::write(module_path, b"\0asm").expect("write wasm stub");
        write_signature(dir, json.as_bytes());
    }

    fn write_signature(dir: &Path, manifest_bytes: &[u8]) {
        let signing_key = SigningKey::from_bytes(&TEST_SIGNING_KEY);
        let signature = signing_key.sign(manifest_bytes);
        let encoded_signature = Base64.encode(signature.to_bytes());
        let metadata = serde_json::json!({
            "version": 1,
            "signer": DEFAULT_SIGNER_ID,
            "algorithm": "ed25519",
            "signature": encoded_signature,
        });
        fs::write(
            dir.join(SIGNATURE_FILE),
            serde_json::to_string_pretty(&metadata).expect("serialize signature"),
        )
        .expect("write signature");
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
        let trust = TrustPolicy::release_defaults();
        let inventory = discover(&plugin_dir, &policy, &trust).unwrap();
        assert_eq!(inventory.plugins.len(), 2);
        assert!(inventory.failures.is_empty());
        let ids: Vec<_> = inventory
            .plugins
            .iter()
            .map(|pkg| pkg.manifest.id.as_str())
            .collect();
        assert!(ids.contains(&"dev.rustnotepad.hello"));
        assert!(ids.contains(&"dev.rustnotepad.goodbye"));
        for pkg in &inventory.plugins {
            assert!(
                matches!(pkg.trust, PluginTrust::Trusted { .. }),
                "expected trusted plugin"
            );
        }
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

        let trust = TrustPolicy::release_defaults();
        let inventory = discover(&plugin_dir, &CapabilityPolicy::locked_down(), &trust).unwrap();
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
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        fs::write(&manifest_path, &manifest_bytes).unwrap();
        write_signature(&plugin_a, &manifest_bytes);

        let trust = TrustPolicy::release_defaults();
        let inventory = discover(&plugin_dir, &CapabilityPolicy::locked_down(), &trust).unwrap();
        assert!(inventory.plugins.is_empty());
        assert_eq!(inventory.failures.len(), 1);
        assert!(
            inventory.failures[0]
                .message
                .contains("referenced wasm module missing"),
            "expected missing module message"
        );
    }

    #[test]
    fn unsigned_plugins_are_rejected_when_policy_requires_signature() {
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join("plugins");
        fs::create_dir_all(&plugin_dir).unwrap();
        let plugin_a = plugin_dir.join("a");
        fs::create_dir(&plugin_a).unwrap();
        let manifest = base_manifest();
        let manifest_path = plugin_a.join(MANIFEST_FILE);
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        fs::write(&manifest_path, &manifest_bytes).unwrap();
        let module_path = plugin_a.join(&manifest.entry);
        fs::write(module_path, b"\0asm").unwrap();

        let trust = TrustPolicy::release_defaults();
        let inventory = discover(&plugin_dir, &CapabilityPolicy::locked_down(), &trust).unwrap();
        assert!(inventory.plugins.is_empty());
        assert_eq!(inventory.failures.len(), 1);
        assert!(
            inventory.failures[0].message.contains("signature required"),
            "expected signature missing failure"
        );
    }

    #[test]
    fn unsigned_plugins_load_when_policy_allows_it() {
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join("plugins");
        fs::create_dir_all(&plugin_dir).unwrap();
        let plugin_a = plugin_dir.join("a");
        fs::create_dir(&plugin_a).unwrap();
        let manifest = base_manifest();
        let manifest_path = plugin_a.join(MANIFEST_FILE);
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        fs::write(&manifest_path, &manifest_bytes).unwrap();
        let module_path = plugin_a.join(&manifest.entry);
        fs::write(module_path, b"\0asm").unwrap();

        let trust = TrustPolicy::allow_unsigned([TrustedSigner::new(
            DEFAULT_SIGNER_ID,
            VerifyingKey::from_bytes(&DEFAULT_SIGNER_KEY).unwrap(),
        )]);
        let inventory = discover(&plugin_dir, &CapabilityPolicy::locked_down(), &trust).unwrap();
        assert_eq!(inventory.plugins.len(), 1);
        assert!(inventory.failures.is_empty());
        assert!(matches!(inventory.plugins[0].trust, PluginTrust::Unsigned));
    }
}
