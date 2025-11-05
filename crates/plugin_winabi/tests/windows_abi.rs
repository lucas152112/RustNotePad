#![cfg(target_os = "windows")]

use std::collections::HashSet;
use std::env;
use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};

use libloading::Library;
use rustnotepad_plugin_winabi::{winconst, LoadedPlugin, PluginCommand, WindowsMessage};
use tempfile::tempdir;

#[derive(Debug, Clone, Copy)]
struct PluginCase {
    module: &'static str,
    label: &'static str,
    shortcut_key: Option<char>,
    init_checked: bool,
}

static PLUGIN_CASES: &[PluginCase] = &[
    PluginCase {
        module: "compat_alpha",
        label: "Compat Alpha",
        shortcut_key: Some('A'),
        init_checked: true,
    },
    PluginCase {
        module: "compat_beta",
        label: "Compat Beta",
        shortcut_key: Some('B'),
        init_checked: false,
    },
    PluginCase {
        module: "compat_gamma",
        label: "Compat Gamma",
        shortcut_key: None,
        init_checked: false,
    },
];

#[test]
fn windows_plugin_abi_roundtrip() {
    for case in PLUGIN_CASES {
        let dll_path = build_sample_plugin(case);
        run_plugin_validation(case, &dll_path);
    }
}

fn run_plugin_validation(case: &PluginCase, dll_path: &Path) {
    unsafe {
        let plugin = LoadedPlugin::load(dll_path).expect("load compiled plugin");
        assert_eq!(plugin.name(), case.label);
        assert!(plugin.is_unicode(), "plugins must report Unicode support");

        let commands = plugin.commands();
        assert_eq!(commands.len(), 2, "expected two exported commands");
        assert_command(
            commands,
            0,
            "Command One",
            case.shortcut_key.map(|key| (true, false, true, key as u8)),
            case.init_checked,
        );
        assert_command(commands, 1, "Command Two", None, false);

        let lib = Library::new(dll_path).expect("open library for state inspection");
        let reset_state: libloading::Symbol<unsafe extern "C" fn()> =
            lib.get(b"reset_state\0").expect("reset_state export");
        reset_state();

        let set_info_handles = (0xAA11isize, 0xBB22isize, 0xCC33isize);
        plugin.set_info(rustnotepad_plugin_winabi::NppData {
            npp_handle: set_info_handles.0,
            scintilla_main_handle: set_info_handles.1,
            scintilla_second_handle: set_info_handles.2,
        });

        let get_last_npp: libloading::Symbol<unsafe extern "C" fn() -> isize> = lib
            .get(b"get_last_npp_handle\0")
            .expect("get last npp handle");
        let get_last_scintilla_main: libloading::Symbol<unsafe extern "C" fn() -> isize> = lib
            .get(b"get_last_scintilla_main\0")
            .expect("get last scintilla main");
        let get_last_scintilla_second: libloading::Symbol<unsafe extern "C" fn() -> isize> = lib
            .get(b"get_last_scintilla_second\0")
            .expect("get last scintilla second");
        assert_eq!(get_last_npp(), set_info_handles.0);
        assert_eq!(get_last_scintilla_main(), set_info_handles.1);
        assert_eq!(get_last_scintilla_second(), set_info_handles.2);

        // Command invocation propagates through the DLL.
        let get_last_command: libloading::Symbol<unsafe extern "C" fn() -> i32> = lib
            .get(b"get_last_command_id\0")
            .expect("get last command id");
        commands[0].invoke();
        assert_eq!(get_last_command(), 1);
        commands[1].invoke();
        assert_eq!(get_last_command(), 2);

        // Windows message bridge
        let get_last_message: libloading::Symbol<unsafe extern "C" fn() -> u32> = lib
            .get(b"get_last_message_id\0")
            .expect("get last message id");
        let get_last_wparam: libloading::Symbol<unsafe extern "C" fn() -> u64> =
            lib.get(b"get_last_wparam\0").expect("get last wparam");
        let get_last_lparam: libloading::Symbol<unsafe extern "C" fn() -> i64> =
            lib.get(b"get_last_lparam\0").expect("get last lparam");

        let message = WindowsMessage::new(winconst::WM_COMMAND, 0xDEAD, 0xBEEF);
        plugin.dispatch_message(message);
        assert_eq!(get_last_message(), winconst::WM_COMMAND);
        assert_eq!(get_last_wparam(), 0xDEAD);
        assert_eq!(get_last_lparam(), 0xBEEF);

        // Scintilla/notification bridge
        let get_last_notification: libloading::Symbol<unsafe extern "C" fn() -> i32> = lib
            .get(b"get_last_notification\0")
            .expect("get last notification");
        let mut notification_code: i32 = 404;
        plugin.be_notified(&mut notification_code as *mut _ as *mut c_void);
        assert_eq!(get_last_notification(), 404);
    }
}

fn assert_command(
    commands: &[PluginCommand],
    index: usize,
    expected_name: &str,
    expected_shortcut: Option<(bool, bool, bool, u8)>,
    expected_checked: bool,
) {
    let command = commands
        .get(index)
        .unwrap_or_else(|| panic!("command index {index} missing"));
    assert_eq!(command.name(), expected_name);
    assert_eq!(command.initially_checked(), expected_checked);
    match (command.shortcut(), expected_shortcut) {
        (Some(shortcut), Some((ctrl, alt, shift, key))) => {
            assert_eq!(shortcut.ctrl, ctrl);
            assert_eq!(shortcut.alt, alt);
            assert_eq!(shortcut.shift, shift);
            assert_eq!(shortcut.key, key);
        }
        (None, None) => {}
        (actual, expected) => panic!(
            "shortcut mismatch for {}: actual={actual:?}, expected={expected:?}",
            command.name()
        ),
    }
}

fn build_sample_plugin(case: &PluginCase) -> PathBuf {
    let source_dir = tempdir().expect("source dir");
    let source_path = source_dir.path().join(format!("{}_plugin.c", case.module));
    fs::write(&source_path, generate_plugin_source(case)).expect("write plugin source");

    compile_plugin(&source_path, case.module)
}

fn compile_plugin(source_path: &Path, module: &str) -> PathBuf {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let before = collect_dlls(&out_dir);

    let mut build = cc::Build::new();
    build
        .file(source_path)
        .shared_flag(true)
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .flag_if_supported("/std:c11")
        .flag_if_supported("-std=c11")
        .compile(module);

    let after = collect_dlls(&out_dir);
    let mut produced: Vec<PathBuf> = after
        .difference(&before)
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.contains(module))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    if produced.is_empty() {
        panic!("failed to locate compiled DLL for module {module}");
    }
    if produced.len() > 1 {
        produced.sort();
    }
    produced.remove(0)
}

fn collect_dlls(root: &Path) -> HashSet<PathBuf> {
    let mut results = HashSet::new();
    if root.is_dir() {
        let entries = fs::read_dir(root).expect("read_dir");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(collect_dlls(&path));
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("dll"))
                .unwrap_or(false)
            {
                let canonical = path.canonicalize().unwrap_or(path.clone());
                results.insert(canonical);
            }
        }
    }
    results
}

fn generate_plugin_source(case: &PluginCase) -> String {
    let shortcut_block = case
        .shortcut_key
        .map(|key| {
            format!(
                "    SHORTCUTS[0].isCtrl = 1;\n    SHORTCUTS[0].isAlt = 0;\n    SHORTCUTS[0].isShift = 1;\n    SHORTCUTS[0].key = '{}';\n    FUNC_ITEMS[0].shortcut = &SHORTCUTS[0];\n",
                key
            )
        })
        .unwrap_or_else(|| "    FUNC_ITEMS[0].shortcut = NULL;\n".to_string());

    let init_to_check = if case.init_checked { 1 } else { 0 };

    format!(
        r#"#include <windows.h>
#include <stdint.h>

typedef struct {{
    unsigned char isCtrl;
    unsigned char isAlt;
    unsigned char isShift;
    unsigned char key;
}} ShortcutKey;

typedef struct {{
    wchar_t itemName[64];
    void (__stdcall *pFunc)(void);
    int cmdID;
    int initToCheck;
    ShortcutKey* shortcut;
}} FuncItem;

typedef struct {{
    intptr_t nppHandle;
    intptr_t scintillaMainHandle;
    intptr_t scintillaSecondHandle;
}} NppData;

static wchar_t PLUGIN_NAME[] = L"{label}";
static ShortcutKey SHORTCUTS[2];
static FuncItem FUNC_ITEMS[2];
static NppData LAST_INFO = {{0}};
static unsigned int LAST_MESSAGE = 0;
static unsigned long long LAST_WPARAM = 0;
static long long LAST_LPARAM = 0;
static int LAST_NOTIFICATION = -1;
static int LAST_COMMAND = 0;

static void __stdcall command_one(void) {{ LAST_COMMAND = 1; }}
static void __stdcall command_two(void) {{ LAST_COMMAND = 2; }}

static void copy_string(wchar_t *dest, size_t capacity, const wchar_t *source) {{
    size_t i = 0;
    if (!dest || !source || capacity == 0) {{
        return;
    }}
    for (; i + 1 < capacity && source[i] != L'\0'; ++i) {{
        dest[i] = source[i];
    }}
    dest[i] = L'\0';
}}

__declspec(dllexport) void reset_state(void) {{
    LAST_INFO.nppHandle = 0;
    LAST_INFO.scintillaMainHandle = 0;
    LAST_INFO.scintillaSecondHandle = 0;
    LAST_MESSAGE = 0;
    LAST_WPARAM = 0;
    LAST_LPARAM = 0;
    LAST_NOTIFICATION = -1;
    LAST_COMMAND = 0;
}}

__declspec(dllexport) void setInfo(NppData data) {{
    LAST_INFO = data;
}}

__declspec(dllexport) const wchar_t* getName(void) {{
    return PLUGIN_NAME;
}}

__declspec(dllexport) FuncItem* getFuncsArray(int* count) {{
{shortcut_block}    copy_string(FUNC_ITEMS[0].itemName, 64, L"Command One");
    FUNC_ITEMS[0].pFunc = command_one;
    FUNC_ITEMS[0].cmdID = 1;
    FUNC_ITEMS[0].initToCheck = {init_checked};

    copy_string(FUNC_ITEMS[1].itemName, 64, L"Command Two");
    FUNC_ITEMS[1].pFunc = command_two;
    FUNC_ITEMS[1].cmdID = 2;
    FUNC_ITEMS[1].initToCheck = 0;
    FUNC_ITEMS[1].shortcut = NULL;

    *count = 2;
    return FUNC_ITEMS;
}}

__declspec(dllexport) void beNotified(void* notification) {{
    if (notification) {{
        LAST_NOTIFICATION = *((int*)notification);
    }} else {{
        LAST_NOTIFICATION = -1;
    }}
}}

__declspec(dllexport) intptr_t messageProc(unsigned int message, unsigned long long wParam, long long lParam) {{
    LAST_MESSAGE = message;
    LAST_WPARAM = wParam;
    LAST_LPARAM = lParam;
    return 0;
}}

__declspec(dllexport) int isUnicode(void) {{
    return 1;
}}

__declspec(dllexport) intptr_t get_last_npp_handle(void) {{ return LAST_INFO.nppHandle; }}
__declspec(dllexport) intptr_t get_last_scintilla_main(void) {{ return LAST_INFO.scintillaMainHandle; }}
__declspec(dllexport) intptr_t get_last_scintilla_second(void) {{ return LAST_INFO.scintillaSecondHandle; }}
__declspec(dllexport) unsigned int get_last_message_id(void) {{ return LAST_MESSAGE; }}
__declspec(dllexport) unsigned long long get_last_wparam(void) {{ return LAST_WPARAM; }}
__declspec(dllexport) long long get_last_lparam(void) {{ return LAST_LPARAM; }}
__declspec(dllexport) int get_last_notification(void) {{ return LAST_NOTIFICATION; }}
__declspec(dllexport) int get_last_command_id(void) {{ return LAST_COMMAND; }}

BOOL APIENTRY DllMain(HMODULE module, DWORD reason, LPVOID reserved) {{
    (void)module;
    (void)reserved;
    switch (reason) {{
        case DLL_PROCESS_ATTACH:
            break;
        default:
            break;
    }}
    return TRUE;
}}
"#,
        label = case.label,
        shortcut_block = shortcut_block,
        init_checked = init_to_check
    )
}
