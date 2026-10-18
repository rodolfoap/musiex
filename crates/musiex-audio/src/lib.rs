//! Audio for musiex.
//!
//! A [`Player`] turns timed notes from `musiex_core::perform` into samples
//! through an [`Engine`]: the dependency-free [`BuiltinSynth`], or
//! [`SoundFontSynth`] for General MIDI SoundFonts. Rendering works block by
//! block, so the same code serves offline WAV export and live playback, and
//! the tempo can change while playing.

mod builtin;
mod player;
mod soundfont;

pub use builtin::BuiltinSynth;
pub use player::{render_all, Player};
pub use soundfont::SoundFontSynth;

/// Something that makes sound from note-on and note-off messages.
pub trait Engine {
    fn note_on(&mut self, channel: u8, key: u8, velocity: u8);
    fn note_off(&mut self, channel: u8, key: u8);
    /// Fills both buffers (same length) with the next samples, overwriting them.
    fn render(&mut self, left: &mut [f32], right: &mut [f32]);
}

impl<E: Engine + ?Sized> Engine for Box<E> {
    fn note_on(&mut self, channel: u8, key: u8, velocity: u8) {
        (**self).note_on(channel, key, velocity)
    }
    fn note_off(&mut self, channel: u8, key: u8) {
        (**self).note_off(channel, key)
    }
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        (**self).render(left, right)
    }
}
