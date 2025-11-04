//! Plugin administration utilities for installing, updating, and removing plugins.
//!
//! 供 RustNotePad 管理外掛（安裝/更新/移除）的工具函式。

use rustnotepad_plugin_wasm::{
    PluginManifest, PluginTrust, WasmPluginPackage, DEFAULT_RELATIVE_ROOT as WASM_ROOT,
    MANIFEST_FILE as WASM_MANIFEST_FILE,
};
use rustnotepad_plugin_winabi::DEFAULT_RELATIVE_ROOT as WIN_ROOT;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Controls how installation should behave when the target already exists.
/// 控制安裝遇到既有目標時的行為。
#[derive(Debug, Clone, Copy, Default)]
pub struct InstallOptions {
    pub overwrite: bool,
}

/// Result of installing a plugin.
/// 外掛安裝的結果。
#[derive(Debug, Clone)]
pub enum InstallOutcome {
    Wasm {
        manifest: PluginManifest,
        dest_dir: PathBuf,
    },
    Windows {
        dll_name: String,
        dest_path: PathBuf,
    },
}

/// Errors that may arise while managing plugins.
/// 管理外掛時可能發生的錯誤。
#[derive(Debug, Error)]
pub enum PluginAdminError {
    #[error("source path '{0}' does not exist")]
    SourceMissing(PathBuf),
    #[error("manifest '{0}' missing")]
    ManifestMissing(PathBuf),
    #[error("failed to read manifest {0}")]
    ManifestRead(PathBuf, #[source] io::Error),
    #[error("failed to parse manifest {0}")]
    ManifestParse(PathBuf, #[source] serde_json::Error),
    #[error("invalid manifest {path}: {reason}")]
    ManifestInvalid { path: PathBuf, reason: String },
    #[error("target '{0}' already exists (use overwrite)")]
    TargetExists(PathBuf),
    #[error("failed to copy from {from} to {to}")]
    CopyFailed {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to remove '{0}'")]
    RemoveFailed(PathBuf, #[source] io::Error),
    #[error("no DLL found under '{0}'")]
    DllNotFound(PathBuf),
    #[error("I/O error on '{0}'")]
    Io(PathBuf, #[source] io::Error),
}

/// Installs (or updates) a WASM plugin from the provided directory.
/// 將指定資料夾中的 WASM 外掛安裝/更新至工作區。
pub fn install_wasm_plugin(
    workspace_root: &Path,
    source_dir: &Path,
    options: InstallOptions,
) -> Result<InstallOutcome, PluginAdminError> {
    if !source_dir.exists() {
        return Err(PluginAdminError::SourceMissing(source_dir.to_path_buf()));
    }
    let manifest_path = source_dir.join(WASM_MANIFEST_FILE);
    if !manifest_path.exists() {
        return Err(PluginAdminError::ManifestMissing(manifest_path));
    }
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|err| PluginAdminError::ManifestRead(manifest_path.clone(), err))?;
    let manifest: PluginManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| PluginAdminError::ManifestParse(manifest_path.clone(), err))?;
    manifest
        .validate()
        .map_err(|reason| PluginAdminError::ManifestInvalid {
            path: manifest_path.clone(),
            reason,
        })?;

    let target_dir = workspace_root.join(WASM_ROOT).join(&manifest.id);
    prepare_destination_dir(&target_dir, options.overwrite)?;
    copy_dir_recursive(source_dir, &target_dir)?;

    // Ensure referenced module exists after copy.
    WasmPluginPackage::new(manifest.clone(), target_dir.clone(), PluginTrust::Unsigned).map_err(
        |err| PluginAdminError::ManifestInvalid {
            path: manifest_path,
            reason: err.to_string(),
        },
    )?;

    Ok(InstallOutcome::Wasm {
        manifest,
        dest_dir: target_dir,
    })
}

/// Installs (or updates) a Windows DLL plugin using a file or directory source.
/// 從檔案或資料夾安裝/更新 Windows DLL 外掛。
pub fn install_windows_plugin(
    workspace_root: &Path,
    source: &Path,
    options: InstallOptions,
) -> Result<InstallOutcome, PluginAdminError> {
    if !source.exists() {
        return Err(PluginAdminError::SourceMissing(source.to_path_buf()));
    }
    let (dll_path, dll_name) = resolve_dll_source(source)?;
    let target_root = workspace_root.join(WIN_ROOT);
    fs::create_dir_all(&target_root)
        .map_err(|err| PluginAdminError::Io(target_root.clone(), err))?;
    let target_path = target_root.join(&dll_name);
    if target_path.exists() {
        if options.overwrite {
            fs::remove_file(&target_path)
                .map_err(|err| PluginAdminError::RemoveFailed(target_path.clone(), err))?;
        } else {
            return Err(PluginAdminError::TargetExists(target_path));
        }
    }
    fs::copy(&dll_path, &target_path).map_err(|err| PluginAdminError::CopyFailed {
        from: dll_path.clone(),
        to: target_path.clone(),
        source: err,
    })?;

    Ok(InstallOutcome::Windows {
        dll_name,
        dest_path: target_path,
    })
}

/// Removes the WASM plugin directory for the given identifier.
/// 移除指定識別碼的 WASM 外掛。
pub fn remove_wasm_plugin(workspace_root: &Path, plugin_id: &str) -> Result<(), PluginAdminError> {
    let target_dir = workspace_root.join(WASM_ROOT).join(plugin_id);
    if !target_dir.exists() {
        return Err(PluginAdminError::SourceMissing(target_dir));
    }
    fs::remove_dir_all(&target_dir).map_err(|err| PluginAdminError::RemoveFailed(target_dir, err))
}

/// Removes the Windows plugin DLL with the given filename.
/// 移除指定檔名的 Windows 外掛。
pub fn remove_windows_plugin(
    workspace_root: &Path,
    dll_name: &str,
) -> Result<(), PluginAdminError> {
    let target_root = workspace_root.join(WIN_ROOT);
    let target_path = target_root.join(dll_name);
    if target_path.is_file() {
        fs::remove_file(&target_path)
            .map_err(|err| PluginAdminError::RemoveFailed(target_path.clone(), err))?;
    } else if target_path.is_dir() {
        fs::remove_dir_all(&target_path)
            .map_err(|err| PluginAdminError::RemoveFailed(target_path.clone(), err))?;
    } else {
        return Err(PluginAdminError::SourceMissing(target_path));
    }
    Ok(())
}

fn prepare_destination_dir(dest: &Path, overwrite: bool) -> Result<(), PluginAdminError> {
    if dest.exists() {
        if overwrite {
            fs::remove_dir_all(dest)
                .map_err(|err| PluginAdminError::RemoveFailed(dest.to_path_buf(), err))?;
        } else {
            return Err(PluginAdminError::TargetExists(dest.to_path_buf()));
        }
    }
    fs::create_dir_all(dest).map_err(|err| PluginAdminError::Io(dest.to_path_buf(), err))
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), PluginAdminError> {
    for entry in fs::read_dir(src).map_err(|err| PluginAdminError::Io(src.to_path_buf(), err))? {
        let entry = entry.map_err(|err| PluginAdminError::Io(src.to_path_buf(), err))?;
        let entry_path = entry.path();
        let target = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|err| PluginAdminError::Io(entry_path.clone(), err))?;
        if file_type.is_dir() {
            fs::create_dir_all(&target).map_err(|err| PluginAdminError::Io(target.clone(), err))?;
            copy_dir_recursive(&entry_path, &target)?;
        } else if file_type.is_file() {
            fs::copy(&entry_path, &target).map_err(|err| PluginAdminError::CopyFailed {
                from: entry_path.clone(),
                to: target.clone(),
                source: err,
            })?;
        }
    }
    Ok(())
}

fn resolve_dll_source(source: &Path) -> Result<(PathBuf, String), PluginAdminError> {
    if source.is_file() {
        return verify_dll_file(source);
    }
    let mut candidates = Vec::new();
    for entry in
        fs::read_dir(source).map_err(|err| PluginAdminError::Io(source.to_path_buf(), err))?
    {
        let entry = entry.map_err(|err| PluginAdminError::Io(source.to_path_buf(), err))?;
        let path = entry.path();
        if path.is_file() {
            if is_dll(&path) {
                candidates.push(path);
            }
        }
    }
    let dll_path = candidates
        .into_iter()
        .next()
        .ok_or_else(|| PluginAdminError::DllNotFound(source.to_path_buf()))?;
    verify_dll_file(&dll_path)
}

fn verify_dll_file(path: &Path) -> Result<(PathBuf, String), PluginAdminError> {
    if !is_dll(path) {
        return Err(PluginAdminError::DllNotFound(path.to_path_buf()));
    }
    let dll_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PluginAdminError::DllNotFound(path.to_path_buf()))?
        .to_string();
    Ok((path.to_path_buf(), dll_name))
}

fn is_dll(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("dll"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn installs_wasm_plugin_from_directory() {
        let workspace = tempdir().unwrap();
        let source = workspace.path().join("source");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(source.join("bin")).unwrap();
        fs::write(source.join("bin/module.wasm"), b"test").unwrap();
        let manifest = json!({
            "id": "dev.rustnotepad.sample",
            "name": "Sample",
            "version": "1.0.0",
            "entry": "bin/module.wasm",
            "capabilities": ["buffer-read"]
        });
        fs::write(
            source.join(WASM_MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let outcome = install_wasm_plugin(workspace.path(), &source, InstallOptions::default())
            .expect("install wasm");
        match outcome {
            InstallOutcome::Wasm { manifest, dest_dir } => {
                assert_eq!(manifest.id, "dev.rustnotepad.sample");
                assert!(dest_dir.join("bin/module.wasm").exists());
            }
            _ => panic!("unexpected outcome"),
        }
    }

    #[test]
    fn installs_windows_plugin_from_file() {
        let workspace = tempdir().unwrap();
        let source = workspace.path().join("plugin.dll");
        fs::write(&source, b"dll").unwrap();
        let outcome = install_windows_plugin(workspace.path(), &source, InstallOptions::default())
            .expect("install windows");
        match outcome {
            InstallOutcome::Windows {
                dll_name,
                dest_path,
            } => {
                assert_eq!(dll_name, "plugin.dll");
                assert!(dest_path.exists());
            }
            _ => panic!("unexpected outcome"),
        }
    }

    #[test]
    fn removes_plugins() {
        let workspace = tempdir().unwrap();
        let wasm_root = workspace.path().join(WASM_ROOT).join("demo");
        fs::create_dir_all(&wasm_root).unwrap();
        fs::write(wasm_root.join("file"), b"x").unwrap();
        remove_wasm_plugin(workspace.path(), "demo").expect("remove wasm");
        assert!(!wasm_root.exists());

        let win_root = workspace.path().join(WIN_ROOT);
        fs::create_dir_all(&win_root).unwrap();
        let dll_path = win_root.join("demo.dll");
        fs::write(&dll_path, b"dll").unwrap();
        remove_windows_plugin(workspace.path(), "demo.dll").expect("remove windows");
        assert!(!dll_path.exists());
    }
}
