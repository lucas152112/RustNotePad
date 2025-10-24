use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use ron::ser::PrettyConfig;
use rustnotepad_printing::display::Color;
use rustnotepad_printing::job::{Margin, Orientation, PaperId, PaperSize};
use rustnotepad_printing::{
    DisplayCommand, HighlightSpan, LayoutInput, LayoutOptions, Paginator, PrintDisplayList,
    SimplePaginator, WrapMode,
};
use serde::Serialize;

struct SnapshotInput;

impl LayoutInput for SnapshotInput {
    fn line_count(&self) -> usize {
        2
    }

    fn line_text(&self, index: usize) -> Option<&str> {
        match index {
            0 => Some("fn main() {"),
            1 => Some("    println!(\"hi\");"),
            _ => None,
        }
    }

    fn highlight_spans(&self, index: usize) -> Vec<HighlightSpan> {
        match index {
            0 => vec![
                HighlightSpan::new(0, 2, Color::new(0.9, 0.2, 0.2, 1.0), None),
                HighlightSpan::new(3, 7, Color::new(0.2, 0.6, 0.9, 1.0), None),
            ],
            1 => vec![HighlightSpan::new(
                4,
                11,
                Color::new(0.8, 0.4, 0.1, 1.0),
                Some(Color::new(0.95, 0.95, 0.7, 1.0)),
            )],
            _ => Vec::new(),
        }
    }
}

fn options() -> LayoutOptions {
    LayoutOptions {
        paper: PaperSize::new(PaperId::A4, 210.0, 297.0),
        orientation: Orientation::Portrait,
        margins: Margin {
            top: 36.0,
            bottom: 36.0,
            left: 36.0,
            right: 36.0,
        },
        wrap_mode: WrapMode::NoWrap,
        dpi: 96.0,
        font_family: "JetBrains Mono".into(),
        font_size_pt: 11.0,
        line_height_pt: 14.0,
        average_char_width_pt: 7.0,
    }
}

#[test]
fn display_list_matches_snapshot() {
    let paginator = SimplePaginator::default();
    let layout = paginator.paginate(&SnapshotInput, &options());
    assert!(!layout.pages.is_empty(), "expected at least one page");

    let display_list = layout.pages[0].display_list.clone();
    let pretty = PrettyConfig::new()
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let snapshot = SnapshotDisplayList::from_display_list(&display_list);
    let actual = ron::ser::to_string_pretty(&snapshot, pretty).expect("serialize display list");

    let snapshot_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/simple_display.ron");
    let expected = match fs::read_to_string(&snapshot_path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            if let Some(parent) = snapshot_path.parent() {
                fs::create_dir_all(parent).expect("create snapshot directory");
            }
            fs::write(&snapshot_path, actual.as_bytes()).expect("write new snapshot file");
            panic!(
                "snapshot created at {:?}; review the contents and rerun the test",
                snapshot_path
            );
        }
        Err(err) => panic!("failed to read snapshot {:?}: {err}", snapshot_path),
    };

    if actual.trim() != expected.trim() {
        panic!(
            "display list snapshot mismatch.\n--- actual ---\n{}\n--- expected ---\n{}",
            actual, expected
        );
    }
}

#[derive(Serialize)]
struct SnapshotDisplayList {
    commands: Vec<SnapshotCommand>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum SnapshotCommand {
    GlyphRun {
        text: String,
        font_family: String,
        font_size_pt: f32,
        x: f32,
        y: f32,
        color: SnapshotColor,
        background: Option<SnapshotColor>,
    },
    BackgroundRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: SnapshotColor,
    },
    HorizontalRule {
        start: SnapshotPoint,
        end: SnapshotPoint,
        stroke: SnapshotStroke,
    },
}

#[derive(Serialize)]
struct SnapshotPoint {
    x: f32,
    y: f32,
}

#[derive(Serialize)]
struct SnapshotStroke {
    width: f32,
    color: SnapshotColor,
}

#[derive(Serialize)]
struct SnapshotColor {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl SnapshotDisplayList {
    fn from_display_list(list: &PrintDisplayList) -> Self {
        let commands = list
            .commands
            .iter()
            .map(|command| match command {
                DisplayCommand::GlyphRun(run) => SnapshotCommand::GlyphRun {
                    text: run.text.clone(),
                    font_family: run.font_family.clone(),
                    font_size_pt: run.font_size_pt,
                    x: run.position.x,
                    y: run.position.y,
                    color: snapshot_color(run.color),
                    background: run.background.map(snapshot_color),
                },
                DisplayCommand::BackgroundRect(rect) => SnapshotCommand::BackgroundRect {
                    x: rect.origin.x,
                    y: rect.origin.y,
                    width: rect.size.width,
                    height: rect.size.height,
                    color: snapshot_color(rect.color),
                },
                DisplayCommand::HorizontalRule { start, end, stroke } => {
                    SnapshotCommand::HorizontalRule {
                        start: SnapshotPoint {
                            x: start.x,
                            y: start.y,
                        },
                        end: SnapshotPoint { x: end.x, y: end.y },
                        stroke: SnapshotStroke {
                            width: stroke.width,
                            color: snapshot_color(stroke.color),
                        },
                    }
                }
            })
            .collect();

        Self { commands }
    }
}

fn snapshot_color(color: Color) -> SnapshotColor {
    SnapshotColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}
