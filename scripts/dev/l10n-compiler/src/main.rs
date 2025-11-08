use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser;
use rustnotepad_settings::LocalizationManager;
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(
    name = "l10n-compiler",
    about = "Validates RustNotePad localization packs",
    version
)]
struct Args {
    /// 語言包資料夾路徑；預設為 assets/langs。 / Directory that contains locale JSON files (defaults to assets/langs).
    #[arg(value_name = "DIR", default_value = "assets/langs")]
    directory: PathBuf,
    /// 預設回退語系代碼。 / Default fallback locale code.
    #[arg(long, default_value = "en-US")]
    default_locale: String,
    /// 遇到缺少鍵時使程序失敗。 / Fail when locales are missing keys relative to fallback.
    #[arg(long)]
    fail_on_missing: bool,
    /// 比對參考鍵清單確保覆蓋率。 / Optional reference key list to compare against.
    #[arg(long, value_name = "FILE")]
    reference: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("l10n-compiler error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let manager = LocalizationManager::load_from_dir(&args.directory, &args.default_locale)
        .with_context(|| format!("load localization files from {}", args.directory.display()))?;

    let fallback = manager.fallback_code().to_string();
    println!(
        "Loaded {} locale(s); fallback locale: {}",
        manager.locale_summaries().len(),
        fallback
    );

    let mut total_missing = 0usize;
    for stats in manager.catalog_stats() {
        let mut line = format!(
            " - {} [{}]: {} strings ({} plural)",
            stats.display_name, stats.code, stats.total_entries, stats.plural_entries
        );
        if stats.code == fallback {
            line.push_str(" [fallback]");
            println!("{line}");
            continue;
        }

        let missing = manager
            .missing_keys(&stats.code)
            .unwrap_or_else(|| Vec::new());
        if missing.is_empty() {
            println!("{line}");
        } else {
            line.push_str(&format!(" — missing {} key(s)", missing.len()));
            println!("{line}");
            for key in missing.iter().take(5) {
                println!("     · {key}");
            }
            if missing.len() > 5 {
                println!("     · ... {} more", missing.len() - 5);
            }
        }
        total_missing += missing.len();
    }

    if total_missing > 0 {
        eprintln!(
            "Found {total_missing} missing localization key(s) relative to fallback '{}'",
            fallback
        );
        if args.fail_on_missing {
            bail!("missing localization keys detected");
        }
    }

    if let Some(reference) = args.reference.as_ref() {
        verify_reference_keys(&manager, reference)?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ReferenceSpec {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    keys: Vec<String>,
}

fn verify_reference_keys(manager: &LocalizationManager, path: &Path) -> Result<()> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("read localization reference {}", path.display()))?;
    let spec: ReferenceSpec = serde_json::from_str(&contents)
        .with_context(|| format!("parse reference {}", path.display()))?;
    let locale = spec
        .locale
        .unwrap_or_else(|| manager.fallback_code().to_string());
    let mut missing = Vec::new();
    for key in spec.keys.iter() {
        if !manager.locale_has_key(&locale, key) {
            missing.push(key.clone());
        }
    }
    if missing.is_empty() {
        if let Some(source) = spec.source {
            println!(
                "Reference coverage OK for locale '{}' against {} ({} keys)",
                locale,
                source,
                spec.keys.len()
            );
        } else {
            println!(
                "Reference coverage OK for locale '{}' ({} keys)",
                locale,
                spec.keys.len()
            );
        }
        return Ok(());
    }
    eprintln!(
        "Localization reference check failed for locale '{}'; missing {} key(s)",
        locale,
        missing.len()
    );
    for key in missing.iter() {
        eprintln!("  · {key}");
    }
    bail!("reference coverage mismatch detected");
}
