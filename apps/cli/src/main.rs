use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rustnotepad_core::{Document, Encoding, LegacyEncoding, LineEnding};
use rustnotepad_search::{
    FileSearchResult, ReplaceAllOutcome, SearchEngine, SearchMode, SearchOptions, SearchReport,
};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    name = "rustnotepad-cli",
    about = "Utility commands for RustNotePad editors",
    author,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 在不同編碼與行尾間轉換文字檔。 / Convert text files between encodings and line endings.
    Convert(ConvertArgs),
    /// 搜尋與選用的取代指令。 / Search (and optional replace) across files.
    Search(SearchArgs),
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

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Convert(args) => execute_convert(args),
        Commands::Search(args) => execute_search(args),
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
