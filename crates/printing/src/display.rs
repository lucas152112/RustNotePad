use std::fmt;

#[cfg(test)]
use serde::Serialize;

/// Represents the printable display list used by the platform adapters.
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone, Default)]
pub struct PrintDisplayList {
    pub commands: Vec<DisplayCommand>,
}

impl PrintDisplayList {
    /// Append a command to the display list.
    pub fn push(&mut self, command: DisplayCommand) {
        self.commands.push(command);
    }

    /// Returns true if the display list is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Low-level drawing commands emitted by the layout stage.
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone)]
pub enum DisplayCommand {
    GlyphRun(GlyphRun),
    BackgroundRect(Rectangle),
    HorizontalRule {
        start: Point,
        end: Point,
        stroke: Stroke,
    },
}

/// Describes an individual shaped glyph run.
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub text: String,
    pub font_family: String,
    pub font_size_pt: f32,
    pub position: Point,
    pub color: Color,
    pub background: Option<Color>,
}

/// Represents a rectangular region (e.g. for line background).
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub origin: Point,
    pub size: Size,
    pub color: Color,
}

/// 2D size representation.
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// 2D coordinate.
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// RGBA color stored in normalized floating-point form.
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// Stroke descriptor for simple line drawing.
#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Clone, Copy)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rgba({:.3}, {:.3}, {:.3}, {:.3})",
            self.r, self.g, self.b, self.a
        )
    }
}
