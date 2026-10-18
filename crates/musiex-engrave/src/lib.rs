//! Engraving for musiex: lays out bars as one staff system and draws it as SVG.
//!
//! Exercises are a single voice on a single staff, which keeps the engraver
//! small: noteheads, stems, flags, beams, ties, tuplets, rests, accidentals,
//! clefs, key and time signatures. The same drawing feeds the practice screen
//! and video frames, and every note reports where it sits so playback can
//! follow along.
//!
//! Glyphs come from the Bravura font (SIL Open Font License 1.1, see
//! `BRAVURA-OFL.txt`), embedded as path data so no font is needed at runtime.

pub mod glyphs;
mod layout;
mod scene;

pub use layout::{bar_width, engrave_system, header_width, pack, NoteBox, System, SystemOptions};
pub use scene::{Element, Scene, Shape};

/// A font glyph as SVG path data, in staff spaces with y pointing down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    pub path: &'static str,
    /// x_min, y_min, x_max, y_max.
    pub bbox: (f32, f32, f32, f32),
    pub advance: f32,
}

impl Glyph {
    pub fn width(&self) -> f32 {
        self.bbox.2 - self.bbox.0
    }
}
