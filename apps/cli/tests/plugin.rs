use std::error::Error;
use std::fs;

use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn plugin_install_and_remove_roundtrip() -> Result<(), Box<dyn Error>> {
    let workspace = tempdir()?;
    let workspace_path = workspace.path();

    let wasm_source = tempdir()?;
    let wasm_dir = wasm_source.path();
    fs::create_dir_all(wasm_dir.join("bin"))?;
    fs::write(wasm_dir.join("bin/module.wasm"), b"\0asm")?;
    fs::write(
        wasm_dir.join("plugin.json"),
        r#"{
  "id": "dev.rustnotepad.cli.sample",
  "name": "CLI Sample",
  "version": "1.0.0",
  "entry": "bin/module.wasm",
  "capabilities": ["buffer-read"]
}
"#,
    )?;

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "--workspace",
            workspace_path.to_str().unwrap(),
            "plugin",
            "install",
            wasm_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let installed_wasm = workspace_path.join("plugins/wasm/dev.rustnotepad.cli.sample");
    assert!(installed_wasm.exists(), "WASM plugin should be installed");

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "--workspace",
            workspace_path.to_str().unwrap(),
            "plugin",
            "remove",
            "--wasm",
            "dev.rustnotepad.cli.sample",
        ])
        .assert()
        .success();
    assert!(
        !installed_wasm.exists(),
        "WASM plugin directory should be removed"
    );

    let win_source = tempdir()?;
    let dll_path = win_source.path().join("CliPlugin.dll");
    fs::write(&dll_path, b"dll")?;

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "--workspace",
            workspace_path.to_str().unwrap(),
            "plugin",
            "install",
            dll_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let installed_dll = workspace_path.join("plugins/win32/CliPlugin.dll");
    assert!(
        installed_dll.exists(),
        "Windows DLL plugin should be copied into workspace"
    );

    Command::cargo_bin("rustnotepad-cli")?
        .args([
            "--workspace",
            workspace_path.to_str().unwrap(),
            "plugin",
            "remove",
            "--dll",
            "CliPlugin.dll",
        ])
        .assert()
        .success();
    assert!(
        !installed_dll.exists(),
        "Windows plugin DLL should be removed"
    );

    Ok(())
}
