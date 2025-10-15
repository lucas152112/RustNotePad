use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rustnotepad_core::{Document, Encoding, LineEnding};

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
    let mut document = Document::open(input)
        .with_context(|| format!("failed to open {}", input.display()))?;

    if let Some(expected) = from {
        let expected_encoding: Encoding = expected.into();
        if document.encoding() != expected_encoding {
            bail!(
                "input {} is detected as {:?} but --from {:?} was supplied",
                input.display(),
                document.encoding(),
                expected_encoding
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
