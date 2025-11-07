use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rustnotepad_core::{Document, Encoding, LegacyEncoding, LineEnding};
use rustnotepad_plugin_admin as plugin_admin;
use rustnotepad_plugin_admin::{
    InstallOptions as PluginInstallOptions, InstallOutcome as PluginInstallOutcome,
};
use rustnotepad_plugin_wasm::MANIFEST_FILE as WASM_MANIFEST_FILE;
#[cfg(target_os = "windows")]
use rustnotepad_plugin_winabi::LoadedPlugin;
use rustnotepad_search::{
    FileSearchResult, ReplaceAllOutcome, SearchEngine, SearchMode, SearchOptions, SearchReport,
};
use rustnotepad_settings::{PreferencesStore, ThemeDefinition, ThemeManager};
use serde_json::Value as SerdeValue;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    name = "rustnotepad-cli",
    about = "Utility commands for RustNotePad editors",
    author,
    version
)]
struct Cli {
    /// 指定工作區根目錄；預設為目前目錄。 / Workspace root (defaults to current directory).
    #[arg(long, global = true, value_name = "PATH")]
    workspace: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 在不同編碼與行尾間轉換文字檔。 / Convert text files between encodings and line endings.
    Convert(ConvertArgs),
    /// 搜尋與選用的取代指令。 / Search (and optional replace) across files.
    Search(SearchArgs),
    /// 管理 RustNotePad 外掛（安裝/移除）。 / Manage RustNotePad plugins (install/remove).
    #[command(subcommand)]
    Plugin(PluginCommand),
    /// 管理語言包。 / Manage localization packs.
    #[command(subcommand)]
    Localization(LocalizationCommand),
    /// 匯入/匯出主題。 / Import or export themes.
    #[command(subcommand)]
    Themes(ThemesCommand),
    /// 匯入/匯出偏好設定。 / Import or export preferences.
    #[command(subcommand)]
    Preferences(PreferencesCommand),
}

#[derive(Args)]
struct ConvertArgs {
    /// 需要轉換的輸入檔案。 / Input files to convert.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// 預期的輸入編碼；若略過則採自動偵測。 / Expected encoding of the input files; detection is used when omitted.
    #[arg(long)]
    from: Option<EncodingChoice>,

    /// 輸出的目標編碼。 / Target encoding for the output.
    #[arg(long, value_name = "ENCODING")]
    to: EncodingChoice,

    /// 輸出的目標行尾類型。 / Target line ending for the output.
    #[arg(long, value_name = "LINE_ENDING")]
    line_ending: Option<LineEndingChoice>,

    /// 是否在輸出中包含 BOM；預設沿用輸入設定。 / Whether the output should include a BOM; defaults to preserving the input BOM.
    #[arg(long, value_name = "true|false")]
    bom: Option<bool>,

    /// 是否就地覆寫原始檔案。 / Write results in place, overwriting the source files.
    #[arg(long)]
    in_place: bool,

    /// 單一檔案轉換時指定輸出路徑。 / Output file path when converting a single file.
    #[arg(long, conflicts_with = "in_place")]
    output: Option<PathBuf>,

    /// 批次轉換時的輸出資料夾。 / Output directory for batch conversions.
    #[arg(long, conflicts_with = "in_place")]
    output_dir: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum EncodingChoice {
    #[value(alias = "utf-8")]
    Utf8,
    #[value(name = "utf16-le", aliases = ["utf16le", "utf16_le"])]
    Utf16Le,
    #[value(name = "utf16-be", aliases = ["utf16be", "utf16_be"])]
    Utf16Be,
    #[value(name = "windows-1252", aliases = ["cp1252", "windows1252", "latin1"])]
    Windows1252,
    #[value(name = "shift-jis", aliases = ["shiftjis", "sjis"])]
    ShiftJis,
    #[value(name = "gbk", aliases = ["gb2312"])]
    Gbk,
    #[value(name = "big5")]
    Big5,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LineEndingChoice {
    #[value(alias = "unix")]
    Lf,
    #[value(name = "crlf", aliases = ["cr-lf", "dos"])]
    CrLf,
    #[value(alias = "mac")]
    Cr,
}

impl From<EncodingChoice> for Encoding {
    fn from(choice: EncodingChoice) -> Self {
        match choice {
            EncodingChoice::Utf8 => Encoding::Utf8,
            EncodingChoice::Utf16Le => Encoding::Utf16Le,
            EncodingChoice::Utf16Be => Encoding::Utf16Be,
            EncodingChoice::Windows1252 => Encoding::Legacy(LegacyEncoding::Windows1252),
            EncodingChoice::ShiftJis => Encoding::Legacy(LegacyEncoding::ShiftJis),
            EncodingChoice::Gbk => Encoding::Legacy(LegacyEncoding::Gbk),
            EncodingChoice::Big5 => Encoding::Legacy(LegacyEncoding::Big5),
        }
    }
}

impl From<LineEndingChoice> for LineEnding {
    fn from(choice: LineEndingChoice) -> Self {
        match choice {
            LineEndingChoice::Lf => LineEnding::Lf,
            LineEndingChoice::CrLf => LineEnding::CrLf,
            LineEndingChoice::Cr => LineEnding::Cr,
        }
    }
}

#[derive(Args)]
struct SearchArgs {
    /// 搜尋樣式（文字或 regex） / Pattern to search for (literal or regex).
    pattern: String,

    /// 指定搜尋路徑（檔案或資料夾）；預設為目前目錄。 / Files or directories to search; defaults to current directory.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// 使用正規表示式。 / Interpret pattern as regex.
    #[arg(long)]
    regex: bool,

    /// 區分大小寫。 / Case sensitive search.
    #[arg(long)]
    case_sensitive: bool,

    /// 限制完整字詞。 / Match whole words only.
    #[arg(long)]
    whole_word: bool,

    /// 讓 '.' 匹配換行字元。 / Treat '.' as matching newlines (regex only).
    #[arg(long)]
    dot_matches_newline: bool,

    /// 以指定文字取代。 / Replacement text to apply.
    #[arg(long, value_name = "TEXT")]
    replace: Option<String>,

    /// 實際覆寫檔案（需搭配 --replace）。 / Persist replacements to disk (requires --replace).
    #[arg(long, requires = "replace")]
    apply: bool,
}

#[derive(Subcommand)]
enum PluginCommand {
    /// 安裝或更新外掛。 / Install or update a plugin.
    Install(PluginInstallArgs),
    /// 移除既有外掛。 / Remove an installed plugin.
    Remove(PluginRemoveArgs),
    /// 驗證 Windows DLL 外掛相容性（僅限 Windows）。 / Verify Windows DLL plugin compatibility (Windows only).
    #[cfg(target_os = "windows")]
    Verify(PluginVerifyArgs),
}

#[derive(Args)]
struct PluginInstallArgs {
    /// 插件來源路徑（WASM 資料夾或 DLL 檔案/資料夾）。 / Plugin source path (WASM directory or DLL file/folder).
    #[arg(value_name = "PATH")]
    source: PathBuf,

    /// 外掛來源類型；預設自動判斷。 / Plugin source kind; defaults to auto-detect.
    #[arg(long, value_enum, default_value_t = PluginInstallKind::Auto)]
    kind: PluginInstallKind,

    /// 覆寫既有外掛。 / Overwrite existing plugin.
    #[arg(long)]
    overwrite: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum PluginInstallKind {
    Auto,
    Wasm,
    Windows,
}

#[derive(Args)]
struct PluginRemoveArgs {
    /// 移除指定 ID 的 WASM 外掛。 / Remove a WASM plugin by id.
    #[arg(long, value_name = "PLUGIN_ID", conflicts_with = "dll")]
    wasm: Option<String>,
    /// 移除指定 DLL 名稱的 Windows 外掛。 / Remove a Windows plugin by DLL name.
    #[arg(long, value_name = "DLL_NAME", conflicts_with = "wasm")]
    dll: Option<String>,
}

#[cfg(target_os = "windows")]
#[derive(Args)]
struct PluginVerifyArgs {
    /// DLL 檔案或包含 DLL 的資料夾路徑。 / Path to the DLL or a directory containing it.
    #[arg(value_name = "PATH")]
    source: PathBuf,

    /// 列出命令與快捷鍵。 / Print exported command table.
    #[arg(long)]
    show_commands: bool,
}

#[derive(Subcommand)]
enum LocalizationCommand {
    /// 安裝語言包 JSON。 / Install a localization JSON file.
    Install(LocalizationInstallArgs),
}

#[derive(Args)]
struct LocalizationInstallArgs {
    /// 語言包 JSON 路徑。 / Path to the localization JSON file.
    #[arg(value_name = "FILE")]
    source: PathBuf,
}

#[derive(Subcommand)]
enum ThemesCommand {
    /// 匯入 Notepad++/tmTheme/Sublime 主題。 / Import a theme file.
    Import(ThemeImportArgs),
    /// 匯出目前可用的主題。 / Export a theme to JSON.
    Export(ThemeExportArgs),
    /// 列出已安裝主題。 / List available themes.
    List,
}

#[derive(Args)]
struct ThemeImportArgs {
    /// 主題來源檔案。 / Theme file to import.
    #[arg(value_name = "FILE")]
    source: PathBuf,
}

#[derive(Args)]
struct ThemeExportArgs {
    /// 要匯出的主題名稱。 / Theme name to export.
    #[arg(long, short = 'n', value_name = "NAME")]
    name: String,
    /// 輸出檔案路徑。 / Destination file path.
    #[arg(long, value_name = "FILE")]
    output: PathBuf,
}

#[derive(Subcommand)]
enum PreferencesCommand {
    /// 匯出目前偏好設定。 / Export current preferences.
    Export(PreferencesExportArgs),
    /// 匯入偏好設定 JSON。 / Import preferences from JSON.
    Import(PreferencesImportArgs),
}

#[derive(Args)]
struct PreferencesExportArgs {
    /// 輸出檔案路徑。 / Destination file path.
    #[arg(long, value_name = "FILE")]
    output: PathBuf,
}

#[derive(Args)]
struct PreferencesImportArgs {
    /// 輸入檔案路徑。 / Source preferences JSON.
    #[arg(value_name = "FILE")]
    input: PathBuf,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let Cli { workspace, command } = Cli::parse();
    match command {
        Commands::Convert(args) => execute_convert(args),
        Commands::Search(args) => execute_search(args),
        Commands::Plugin(subcommand) => {
            let workspace_root = resolve_workspace(workspace.clone())?;
            execute_plugin_command(subcommand, &workspace_root)
        }
        Commands::Localization(subcommand) => {
            let workspace_root = resolve_workspace(workspace.clone())?;
            execute_localization_command(subcommand, &workspace_root)
        }
        Commands::Themes(subcommand) => {
            let workspace_root = resolve_workspace(workspace.clone())?;
            execute_themes_command(subcommand, &workspace_root)
        }
        Commands::Preferences(subcommand) => {
            let workspace_root = resolve_workspace(workspace)?;
            execute_preferences_command(subcommand, &workspace_root)
        }
    }
}

fn execute_convert(args: ConvertArgs) -> Result<()> {
    if args.inputs.len() > 1 {
        if args.output.is_some() {
            bail!("--output can only be used when converting a single file");
        }
        if !args.in_place && args.output_dir.is_none() {
            bail!("batch conversions require --output-dir or --in-place");
        }
    }

    if args.in_place && args.output_dir.is_some() {
        bail!("--output-dir cannot be used with --in-place");
    }

    if args.in_place && args.output.is_some() {
        bail!("--output cannot be used with --in-place");
    }

    for input in &args.inputs {
        convert_single(
            input,
            args.from,
            args.to,
            args.line_ending,
            args.bom,
            args.in_place,
            args.output.as_ref(),
            args.output_dir.as_ref(),
        )?;
    }

    Ok(())
}

fn execute_plugin_command(command: PluginCommand, workspace_root: &Path) -> Result<()> {
    match command {
        PluginCommand::Install(args) => install_plugin(args, workspace_root),
        PluginCommand::Remove(args) => remove_plugin(args, workspace_root),
        #[cfg(target_os = "windows")]
        PluginCommand::Verify(args) => verify_plugin(args),
    }
}

fn install_plugin(args: PluginInstallArgs, workspace_root: &Path) -> Result<()> {
    let source = resolve_input_path(&args.source)?;
    if !source.exists() {
        bail!("plugin source '{}' does not exist", source.display());
    }
    let resolved_kind = match args.kind {
        PluginInstallKind::Auto => detect_plugin_kind(&source)?,
        other => other,
    };
    let options = PluginInstallOptions {
        overwrite: args.overwrite,
    };
    match resolved_kind {
        PluginInstallKind::Wasm => {
            match plugin_admin::install_wasm_plugin(workspace_root, &source, options)? {
                PluginInstallOutcome::Wasm { manifest, dest_dir } => {
                    println!(
                        "Installed WASM plugin '{}' to {}",
                        manifest.id,
                        dest_dir.display()
                    );
                }
                other => return Err(anyhow!("unexpected install outcome: {:?}", other)),
            }
        }
        PluginInstallKind::Windows => {
            match plugin_admin::install_windows_plugin(workspace_root, &source, options)? {
                PluginInstallOutcome::Windows {
                    dll_name,
                    dest_path,
                } => {
                    println!(
                        "Installed Windows plugin '{}' to {}",
                        dll_name,
                        dest_path.display()
                    );
                }
                other => return Err(anyhow!("unexpected install outcome: {:?}", other)),
            }
        }
        PluginInstallKind::Auto => unreachable!("auto kind should be resolved above"),
    }
    Ok(())
}

fn remove_plugin(args: PluginRemoveArgs, workspace_root: &Path) -> Result<()> {
    match (args.wasm, args.dll) {
        (Some(id), None) => {
            plugin_admin::remove_wasm_plugin(workspace_root, &id)?;
            println!("Removed WASM plugin '{id}'");
            Ok(())
        }
        (None, Some(dll_name)) => {
            plugin_admin::remove_windows_plugin(workspace_root, &dll_name)?;
            println!("Removed Windows plugin '{dll_name}'");
            Ok(())
        }
        _ => bail!("specify --wasm <PLUGIN_ID> or --dll <DLL_NAME>"),
    }
}

#[cfg(target_os = "windows")]
fn verify_plugin(args: PluginVerifyArgs) -> Result<()> {
    let source = resolve_input_path(&args.source)?;
    let dll_path = resolve_windows_plugin_source(&source)
        .with_context(|| format!("locate DLL within {}", source.display()))?;
    let plugin = unsafe { LoadedPlugin::load(&dll_path) }
        .with_context(|| format!("load plugin {}", dll_path.display()))?;
    println!("Plugin name: {}", plugin.name());
    println!(
        "Source DLL: {}",
        dll_path
            .canonicalize()
            .unwrap_or(dll_path.clone())
            .display()
    );
    println!(
        "Unicode support: {}",
        if plugin.is_unicode() { "yes" } else { "no" }
    );
    if args.show_commands {
        println!("Exported commands:");
        for command in plugin.commands() {
            let shortcut = command.shortcut().map_or_else(
                || "none".to_string(),
                |s| {
                    format!(
                        "{}{}{}{}",
                        if s.ctrl { "Ctrl+" } else { "" },
                        if s.alt { "Alt+" } else { "" },
                        if s.shift { "Shift+" } else { "" },
                        (s.key as char)
                    )
                },
            );
            println!(
                "  - id: {:>3} | checked: {:<5} | shortcut: {:<8} | {}",
                command.command_id(),
                command.initially_checked(),
                shortcut,
                command.name()
            );
        }
    } else {
        println!("Exported commands: {}", plugin.commands().len());
    }
    Ok(())
}

fn execute_localization_command(command: LocalizationCommand, workspace_root: &Path) -> Result<()> {
    match command {
        LocalizationCommand::Install(args) => install_localization(args, workspace_root),
    }
}

fn install_localization(args: LocalizationInstallArgs, workspace_root: &Path) -> Result<()> {
    let source = resolve_input_path(&args.source)?;
    if !source.exists() {
        bail!("localization source '{}' does not exist", source.display());
    }
    let data = fs::read_to_string(&source)
        .with_context(|| format!("failed to read {}", source.display()))?;
    let parsed: SerdeValue = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse localization file {}", source.display()))?;
    let locale_code = parsed
        .get("locale")
        .and_then(|value| value.as_str())
        .map(|code| sanitize_locale_code(code));
    let file_name = if let Some(code) = &locale_code {
        format!("{code}.json")
    } else {
        source
            .file_name()
            .and_then(|name| name.to_str())
            .map(|value| value.to_string())
            .ok_or_else(|| anyhow!("source path must include a file name"))?
    };
    let dest_dir = localization_dir(workspace_root);
    fs::create_dir_all(&dest_dir)
        .with_context(|| format!("failed to create {}", dest_dir.display()))?;
    let dest_path = dest_dir.join(&file_name);
    fs::write(&dest_path, data.into_bytes())
        .with_context(|| format!("failed to write {}", dest_path.display()))?;
    println!(
        "Installed localization '{}' to {}",
        locale_code.as_deref().unwrap_or_else(|| file_name.as_str()),
        dest_path.display()
    );
    Ok(())
}

fn execute_themes_command(command: ThemesCommand, workspace_root: &Path) -> Result<()> {
    match command {
        ThemesCommand::Import(args) => import_theme(args, workspace_root),
        ThemesCommand::Export(args) => export_theme(args, workspace_root),
        ThemesCommand::List => list_themes(workspace_root),
    }
}

fn import_theme(args: ThemeImportArgs, workspace_root: &Path) -> Result<()> {
    let source = resolve_input_path(&args.source)?;
    if !source.exists() {
        bail!("theme source '{}' does not exist", source.display());
    }
    let definition = load_theme_definition(&source)?;
    let user_dir = theme_dir(workspace_root);
    fs::create_dir_all(&user_dir)
        .with_context(|| format!("failed to create {}", user_dir.display()))?;
    let slug = ThemeDefinition::slug_for(&definition.name);
    let dest_path = user_dir.join(format!("{slug}.json"));
    fs::write(&dest_path, definition.to_json_string())
        .with_context(|| format!("failed to write {}", dest_path.display()))?;
    println!(
        "Imported theme '{}' to {}",
        definition.name,
        dest_path.display()
    );
    Ok(())
}

fn export_theme(args: ThemeExportArgs, workspace_root: &Path) -> Result<()> {
    let manager = load_theme_manager(workspace_root)?;
    let theme = manager
        .themes()
        .find(|theme| theme.name == args.name)
        .ok_or_else(|| anyhow!("theme '{}' was not found", args.name))?;
    let output = resolve_input_path(&args.output)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output, theme.to_json_string())
        .with_context(|| format!("failed to write {}", output.display()))?;
    println!("Exported theme '{}' to {}", theme.name, output.display());
    Ok(())
}

fn list_themes(workspace_root: &Path) -> Result<()> {
    let manager = load_theme_manager(workspace_root)?;
    println!("Available themes:");
    for name in manager.theme_names() {
        println!("  - {name}");
    }
    Ok(())
}

fn load_theme_definition(path: &Path) -> Result<ThemeDefinition> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let definition = match extension.as_str() {
        "tmtheme" => ThemeDefinition::from_tmtheme_file(path),
        "xml" => ThemeDefinition::from_notepad_xml(path),
        "sublime-color-scheme" | "sublime-syntax" => {
            ThemeDefinition::from_sublime_color_scheme(path)
        }
        other => bail!(
            "unsupported theme format '{}'; expected .tmTheme, .xml, or .sublime-color-scheme",
            other
        ),
    }
    .with_context(|| format!("failed to import theme {}", path.display()))?;
    Ok(definition)
}

fn execute_preferences_command(command: PreferencesCommand, workspace_root: &Path) -> Result<()> {
    match command {
        PreferencesCommand::Export(args) => export_preferences(args, workspace_root),
        PreferencesCommand::Import(args) => import_preferences(args, workspace_root),
    }
}

fn export_preferences(args: PreferencesExportArgs, workspace_root: &Path) -> Result<()> {
    let prefs_path = preferences_path(workspace_root);
    let store = PreferencesStore::load(&prefs_path)
        .with_context(|| format!("failed to load preferences from {}", prefs_path.display()))?;
    let output = resolve_input_path(&args.output)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    store
        .export_to(&output)
        .with_context(|| format!("failed to export preferences to {}", output.display()))?;
    println!("Exported preferences to {}", output.display());
    Ok(())
}

fn import_preferences(args: PreferencesImportArgs, workspace_root: &Path) -> Result<()> {
    let prefs_path = preferences_path(workspace_root);
    let mut store = PreferencesStore::load(&prefs_path)
        .with_context(|| format!("failed to load preferences from {}", prefs_path.display()))?;
    let input = resolve_input_path(&args.input)?;
    if !input.exists() {
        bail!("preferences file '{}' does not exist", input.display());
    }
    store
        .import_from(&input)
        .with_context(|| format!("failed to import preferences from {}", input.display()))?;
    println!("Imported preferences from {}", input.display());
    Ok(())
}

fn localization_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".rustnotepad").join("langs")
}

fn theme_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".rustnotepad").join("themes")
}

fn preferences_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".rustnotepad").join("preferences.json")
}

fn sanitize_locale_code(input: &str) -> String {
    let mut slug = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            slug.push(ch);
        } else if ch.is_ascii_whitespace() || ch == '.' {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    if slug.is_empty() {
        "locale".to_string()
    } else {
        slug
    }
}

fn load_theme_manager(workspace_root: &Path) -> Result<ThemeManager> {
    let dirs = vec![PathBuf::from("assets/themes"), theme_dir(workspace_root)];
    ThemeManager::load_from_dirs(dirs).map_err(|err| anyhow!("failed to load themes: {err}"))
}

fn detect_plugin_kind(source: &Path) -> Result<PluginInstallKind> {
    let metadata =
        fs::metadata(source).with_context(|| format!("read metadata from {}", source.display()))?;
    if metadata.is_file() {
        if is_dll(source) {
            return Ok(PluginInstallKind::Windows);
        }
        bail!(
            "file '{}' does not look like a Windows plugin; specify --kind",
            source.display()
        );
    }
    if metadata.is_dir() {
        if source.join(WASM_MANIFEST_FILE).exists() {
            return Ok(PluginInstallKind::Wasm);
        }
        for entry in
            fs::read_dir(source).with_context(|| format!("scan directory {}", source.display()))?
        {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_file() && is_dll(&entry_path) {
                return Ok(PluginInstallKind::Windows);
            }
        }
    }
    bail!(
        "unable to infer plugin kind from '{}'; specify --kind",
        source.display()
    );
}

fn is_dll(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("dll"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn resolve_windows_plugin_source(source: &Path) -> Result<PathBuf> {
    if source.is_file() {
        if is_dll(source) {
            return Ok(source.to_path_buf());
        }
        bail!(
            "expected a DLL file, got '{}' (use --kind wasm for WASM plugins)",
            source.display()
        );
    }
    if source.is_dir() {
        let mut dlls = Vec::new();
        for entry in
            fs::read_dir(source).with_context(|| format!("scan directory {}", source.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && is_dll(&path) {
                dlls.push(path);
            }
        }
        match dlls.len() {
            0 => bail!("did not find a DLL under '{}'", source.display()),
            1 => return Ok(dlls.remove(0)),
            _ => bail!(
                "found multiple DLLs under '{}'; specify the DLL file directly",
                source.display()
            ),
        }
    }
    bail!(
        "expected a DLL file or directory containing one at '{}'",
        source.display()
    );
}

fn resolve_workspace(workspace: Option<PathBuf>) -> Result<PathBuf> {
    match workspace {
        Some(path) => {
            if path.is_absolute() {
                Ok(path)
            } else {
                Ok(std::env::current_dir()
                    .context("determine current directory")?
                    .join(path))
            }
        }
        None => std::env::current_dir().context("determine current directory"),
    }
}

fn resolve_input_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("determine current directory")?
            .join(path))
    }
}

fn convert_single(
    input: &Path,
    from: Option<EncodingChoice>,
    to: EncodingChoice,
    line_ending: Option<LineEndingChoice>,
    bom: Option<bool>,
    in_place: bool,
    single_output: Option<&PathBuf>,
    output_dir: Option<&PathBuf>,
) -> Result<()> {
    let mut document =
        Document::open(input).with_context(|| format!("failed to open {}", input.display()))?;

    if let Some(expected) = from {
        let expected_encoding: Encoding = expected.into();
        if document.encoding() != expected_encoding {
            bail!(
                "input {} is detected as {} but --from {} was supplied",
                input.display(),
                document.encoding().name(),
                expected_encoding.name()
            );
        }
    }

    let target_encoding: Encoding = to.into();
    document.set_encoding(target_encoding);

    if let Some(choice) = line_ending {
        document.set_line_ending(choice.into());
    }

    if let Some(include_bom) = bom {
        document.set_bom(include_bom);
    }

    if in_place {
        document
            .save()
            .with_context(|| format!("failed to overwrite {}", input.display()))?;
        return Ok(());
    }

    let output_path = resolve_output_path(input, single_output, output_dir)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    document
        .save_as(&output_path)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    Ok(())
}

fn resolve_output_path(
    input: &Path,
    single_output: Option<&PathBuf>,
    output_dir: Option<&PathBuf>,
) -> Result<PathBuf> {
    if let Some(path) = single_output {
        if output_dir.is_some() {
            bail!("--output and --output-dir cannot be combined");
        }
        return Ok(path.clone());
    }

    if let Some(dir) = output_dir {
        let file_name = input
            .file_name()
            .ok_or_else(|| anyhow!("input {} has no file name", input.display()))?;
        return Ok(dir.join(file_name));
    }

    bail!("missing --output or --output-dir for conversion");
}

fn execute_search(mut args: SearchArgs) -> Result<()> {
    let mut options = SearchOptions::new(args.pattern);
    if args.regex {
        options.mode = SearchMode::Regex;
    }
    options.case_sensitive = args.case_sensitive;
    options.whole_word = args.whole_word;
    options.dot_matches_newline = args.dot_matches_newline;
    options.wrap_around = false;

    if args.paths.is_empty() {
        let cwd = std::env::current_dir().context("failed to determine current directory")?;
        args.paths.push(cwd);
    }

    let targets = collect_target_files(&args.paths)?;
    if targets.is_empty() {
        println!("No files to search.");
        return Ok(());
    }

    let mut entries = Vec::new();
    let mut applied = Vec::new();

    for path in targets {
        match handle_file(&path, &options, args.replace.as_deref(), args.apply) {
            Ok(Some((result, applied_count))) => {
                if let Some(count) = applied_count {
                    applied.push((path.clone(), count));
                }
                entries.push(result);
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {}", path.display(), err);
            }
        }
    }

    let report = SearchReport::new(entries);
    if report.is_empty() {
        println!("No matches found.");
        return Ok(());
    }

    print_search_report(&report, &options);

    if args.replace.is_some() {
        if args.apply {
            for (path, count) in applied {
                println!("Applied {} replacements to {}", count, path.display());
            }
        } else {
            println!("Dry run only; re-run with --apply to write changes.");
        }
    }

    Ok(())
}

fn collect_target_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            for entry in WalkDir::new(path) {
                match entry {
                    Ok(entry) => {
                        if entry.file_type().is_file() {
                            files.push(entry.path().to_path_buf());
                        }
                    }
                    Err(err) => {
                        eprintln!("warning: {}: {}", path.display(), err);
                    }
                }
            }
        } else {
            eprintln!("warning: {} does not exist", path.display());
        }
    }
    Ok(files)
}

fn handle_file(
    path: &Path,
    options: &SearchOptions,
    replacement: Option<&str>,
    apply: bool,
) -> Result<Option<(FileSearchResult, Option<usize>)>> {
    let mut document =
        Document::open(path).with_context(|| format!("failed to open {}", path.display()))?;

    if let Some(replacement_text) = replacement {
        let outcome = {
            let engine = SearchEngine::new(document.contents());
            engine.replace_all(replacement_text, options)?
        };

        if outcome.matches.is_empty() {
            return Ok(None);
        }

        let ReplaceAllOutcome {
            replaced_text,
            replacements,
            matches,
        } = outcome;

        if apply {
            document.set_contents(replaced_text);
            document
                .save()
                .with_context(|| format!("failed to write {}", path.display()))?;
        }

        return Ok(Some((
            FileSearchResult::new(Some(path.to_path_buf()), matches),
            if apply { Some(replacements) } else { None },
        )));
    }

    let matches = {
        let engine = SearchEngine::new(document.contents());
        engine.find_all(options)?
    };

    if matches.is_empty() {
        return Ok(None);
    }

    Ok(Some((
        FileSearchResult::new(Some(path.to_path_buf()), matches),
        None,
    )))
}

fn print_search_report(report: &SearchReport, options: &SearchOptions) {
    let summary = report.summary();
    println!(
        "Search \"{}\" ({} hits in {} files)",
        options.pattern, summary.total_matches, summary.files_with_matches
    );
    for entry in &report.results {
        let label = entry
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unsaved>".to_string());
        println!("  {} ({} hits)", label, entry.matches.len());
        for m in &entry.matches {
            println!("    Line {} (Col {}): {}", m.line, m.column, m.line_text);
        }
    }
}
