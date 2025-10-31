//! WASM plugin runtime for RustNotePad.
//! RustNotePad 的 WASM 外掛執行期。

use anyhow::anyhow;
use rustnotepad_plugin_wasm::{PluginCommand, PluginManifest, WasmPluginPackage};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use wasmtime::{Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

/// Host functions namespace used when linking plugins.
/// 外掛連結時使用的主機函式命名空間。
const HOST_NAMESPACE: &str = "host";

/// Entry function expected in the WASM module for command dispatch.
/// WASM 模組中負責命令派發的預期入口函式名稱。
const COMMAND_EXPORT: &str = "rn_command";

/// Optional initialization function executed after instantiation.
/// 模組實例化後可選擇執行的初始化函式名稱。
const ON_LOAD_EXPORT: &str = "rn_on_load";

/// Host function that allows plugins to emit log messages.
/// 允許外掛輸出記錄訊息的主機函式名稱。
const HOST_LOG_FN: &str = "log";

/// Outcome of invoking a plugin command.
/// 呼叫外掛命令後的結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Return status from the plugin (0 表示成功).
    /// 外掛返回的狀態碼（0 代表成功）。
    pub status: i32,
    /// Messages emitted via `host.log`.
    /// 外掛透過 `host.log` 輸出的訊息。
    pub logs: Vec<String>,
}

/// Loaded plugin instance ready for command execution.
/// 已載入且可供命令執行的外掛實例。
pub struct WasmPluginInstance {
    manifest: PluginManifest,
    module_path: PathBuf,
    store: Store<PluginState>,
    _instance: Instance,
    _memory: Memory,
    command_func: TypedFunc<i32, i32>,
}

impl WasmPluginInstance {
    /// Returns manifest metadata.
    /// 傳回外掛清單後設資料。
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    /// Returns the commands declared in the manifest.
    /// 傳回清單中宣告的命令列表。
    pub fn commands(&self) -> &[PluginCommand] {
        &self.manifest.commands
    }

    /// Executes the command with the provided identifier, when available.
    /// 執行指定識別碼的命令（若存在）。
    pub fn execute_command(&mut self, command_id: &str) -> Result<CommandOutcome, PluginHostError> {
        let index = self
            .manifest
            .commands
            .iter()
            .position(|command| command.id == command_id)
            .ok_or_else(|| PluginHostError::UnknownCommand(command_id.to_string()))?;

        self.store.data_mut().logs.clear();
        let status = self
            .command_func
            .call(&mut self.store, index as i32)
            .map_err(|err| PluginHostError::CommandRejected {
                plugin: self.manifest.id.clone(),
                command: command_id.to_string(),
                reason: err.to_string(),
            })?;

        Ok(CommandOutcome {
            status,
            logs: self.store.data().logs.clone(),
        })
    }

    /// Returns the path to the underlying WASM module.
    /// 傳回對應的 WASM 模組路徑。
    pub fn module_path(&self) -> &Path {
        &self.module_path
    }
}

/// Runtime managing loaded WASM plugins.
/// 管理已載入 WASM 外掛的執行期。
pub struct WasmPluginRuntime {
    engine: Engine,
    linker: Linker<PluginState>,
    plugins: HashMap<String, WasmPluginInstance>,
}

impl WasmPluginRuntime {
    /// Constructs an empty runtime.
    /// 建立空的執行期。
    pub fn new() -> Result<Self, PluginHostError> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        linker
            .func_wrap(
                HOST_NAMESPACE,
                HOST_LOG_FN,
                |mut caller: wasmtime::Caller<'_, PluginState>,
                 ptr: i32,
                 len: i32|
                 -> anyhow::Result<()> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|export| export.into_memory())
                        .ok_or_else(|| anyhow!("memory export not found"))?;
                    let bytes = memory
                        .data(&caller)
                        .get(ptr as usize..)
                        .and_then(|slice| slice.get(..len as usize))
                        .ok_or_else(|| anyhow!("invalid buffer passed to host.log"))?
                        .to_vec();
                    let message = String::from_utf8(bytes).map_err(|_| anyhow!("log not utf-8"))?;
                    caller.data_mut().logs.push(message);
                    Ok(())
                },
            )
            .map_err(|err| PluginHostError::HostRegistration(err.to_string()))?;

        Ok(Self {
            engine,
            linker,
            plugins: HashMap::new(),
        })
    }

    /// Loads plugins from the provided packages, replacing any existing entries.
    /// 依據提供的套件載入外掛，並取代既有的實例。
    pub fn load_packages(&mut self, packages: &[WasmPluginPackage]) -> Result<(), PluginHostError> {
        self.plugins.clear();
        for package in packages {
            let instance = self.instantiate_plugin(package)?;
            self.plugins.insert(package.manifest.id.clone(), instance);
        }
        Ok(())
    }

    /// Instantiates a single plugin package.
    /// 實例化單一外掛套件。
    fn instantiate_plugin(
        &self,
        package: &WasmPluginPackage,
    ) -> Result<WasmPluginInstance, PluginHostError> {
        let module = Module::from_file(&self.engine, &package.module_path).map_err(|err| {
            PluginHostError::ModuleLoad {
                plugin: package.manifest.id.clone(),
                reason: err.to_string(),
            }
        })?;

        let mut store = Store::new(&self.engine, PluginState { logs: Vec::new() });
        let instance = self
            .linker
            .instantiate(&mut store, &module)
            .map_err(|err| PluginHostError::Instantiation {
                plugin: package.manifest.id.clone(),
                reason: err.to_string(),
            })?;

        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            PluginHostError::MissingExport {
                plugin: package.manifest.id.clone(),
                export: "memory".into(),
            }
        })?;

        if let Some(on_load) = instance.get_func(&mut store, ON_LOAD_EXPORT) {
            let typed = on_load.typed::<(), ()>(&store).map_err(|err| {
                PluginHostError::InvalidExportSignature {
                    plugin: package.manifest.id.clone(),
                    export: ON_LOAD_EXPORT.into(),
                    reason: err.to_string(),
                }
            })?;
            typed
                .call(&mut store, ())
                .map_err(|err| PluginHostError::CommandRejected {
                    plugin: package.manifest.id.clone(),
                    command: ON_LOAD_EXPORT.into(),
                    reason: err.to_string(),
                })?;
        }

        let func = instance
            .get_func(&mut store, COMMAND_EXPORT)
            .ok_or_else(|| PluginHostError::MissingExport {
                plugin: package.manifest.id.clone(),
                export: COMMAND_EXPORT.into(),
            })?;
        let typed = func.typed::<i32, i32>(&store).map_err(|err| {
            PluginHostError::InvalidExportSignature {
                plugin: package.manifest.id.clone(),
                export: COMMAND_EXPORT.into(),
                reason: err.to_string(),
            }
        })?;

        Ok(WasmPluginInstance {
            manifest: package.manifest.clone(),
            module_path: package.module_path.clone(),
            store,
            _instance: instance,
            _memory: memory,
            command_func: typed,
        })
    }

    /// Returns the identifiers for loaded plugins.
    /// 取得所有已載入外掛的識別碼。
    pub fn plugin_ids(&self) -> impl Iterator<Item = &String> {
        self.plugins.keys()
    }

    /// Retrieves a mutable handle to a loaded plugin.
    /// 取得外掛的可變參照。
    pub fn plugin_mut(&mut self, plugin_id: &str) -> Option<&mut WasmPluginInstance> {
        self.plugins.get_mut(plugin_id)
    }
}

#[derive(Default)]
struct PluginState {
    logs: Vec<String>,
}

/// Errors emitted by the plugin host runtime.
/// 外掛執行期可能產生的錯誤。
#[derive(Debug, Error)]
pub enum PluginHostError {
    #[error("failed to register host function: {0}")]
    HostRegistration(String),
    #[error("failed to load module for plugin {plugin}: {reason}")]
    ModuleLoad { plugin: String, reason: String },
    #[error("failed to instantiate plugin {plugin}: {reason}")]
    Instantiation { plugin: String, reason: String },
    #[error("missing required export '{export}' in plugin {plugin}")]
    MissingExport { plugin: String, export: String },
    #[error("export '{export}' in plugin {plugin} has an unexpected signature: {reason}")]
    InvalidExportSignature {
        plugin: String,
        export: String,
        reason: String,
    },
    #[error("plugin {plugin} rejected command '{command}': {reason}")]
    CommandRejected {
        plugin: String,
        command: String,
        reason: String,
    },
    #[error("unknown command '{0}'")]
    UnknownCommand(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;
    use wat::parse_str;

    fn build_test_plugin(dir: &Path) -> WasmPluginPackage {
        let manifest = json!({
            "id": "dev.rustnotepad.test",
            "name": "Test Plugin",
            "version": "1.0.0",
            "entry": "plugin.wasm",
            "capabilities": ["register-command"],
            "commands": [
                {
                    "id": "test.run",
                    "name": "Run Test",
                    "description": "Execute test command"
                }
            ]
        });
        std::fs::create_dir_all(dir).expect("create plugin dir");
        std::fs::write(dir.join("plugin.json"), manifest.to_string()).expect("write manifest");
        let wat = r#"
        (module
            (import "host" "log" (func $log (param i32 i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "command executed")
            (func (export "rn_command") (param $id i32) (result i32)
                (drop (local.get $id))
                (call $log (i32.const 0) (i32.const 16))
                (i32.const 0))
        )
        "#;
        let wasm = parse_str(wat).expect("compile wat");
        std::fs::write(dir.join("plugin.wasm"), wasm).expect("write wasm");
        let package = WasmPluginPackage::new(
            serde_json::from_slice(&std::fs::read(dir.join("plugin.json")).unwrap()).unwrap(),
            dir.to_path_buf(),
        )
        .expect("package");
        package
    }

    #[test]
    fn loads_and_executes_wasm_command() {
        let temp = tempdir().unwrap();
        let package = build_test_plugin(temp.path());
        let mut runtime = WasmPluginRuntime::new().unwrap();
        runtime.load_packages(&[package]).unwrap();
        let plugin_id = "dev.rustnotepad.test";
        let plugin = runtime.plugin_mut(plugin_id).expect("plugin loaded");
        assert_eq!(plugin.commands().len(), 1);
        let outcome = plugin.execute_command("test.run").expect("execute command");
        assert_eq!(outcome.status, 0);
        assert_eq!(outcome.logs, vec!["command executed".to_string()]);
    }
}
