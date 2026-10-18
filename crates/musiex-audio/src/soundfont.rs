//! General MIDI SoundFont (.sf2) playback through rustysynth.

use crate::Engine;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::io::Read;
use std::sync::Arc;

pub struct SoundFontSynth {
    synth: Synthesizer,
}

impl SoundFontSynth {
    pub fn new(sf2: &mut impl Read, sample_rate: u32) -> Result<Self, String> {
        let font =
            SoundFont::new(sf2).map_err(|e| format!("could not read the SoundFont: {e:?}"))?;
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let synth = Synthesizer::new(&Arc::new(font), &settings)
            .map_err(|e| format!("could not start the synthesizer: {e:?}"))?;
        Ok(SoundFontSynth { synth })
    }
}

impl Engine for SoundFontSynth {
    fn note_on(&mut self, channel: u8, key: u8, velocity: u8) {
        self.synth
            .note_on(i32::from(channel), i32::from(key), i32::from(velocity));
    }

    fn note_off(&mut self, channel: u8, key: u8) {
        self.synth.note_off(i32::from(channel), i32::from(key));
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synth.render(left, right);
    }
}
