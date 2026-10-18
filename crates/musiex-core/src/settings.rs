//! Everything that shapes an exercise.

use crate::duration::{Dur, EIGHTH, HALF, QUARTER, SIXTEENTH};
use crate::instrument::Instrument;
use crate::meter::Meter;
use crate::pitch::{Clef, Key, ParseError, Pitch};

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub meter: Meter,
    pub key: Key,
    pub clef: Clef,
    /// Beats per minute, where the beat is `meter.beat()` (a dotted quarter in 6/8).
    pub tempo: u32,
    /// Lowest and highest written pitch, inclusive (by staff position).
    pub low: Pitch,
    pub high: Pitch,
    /// Smallest note value: 1 = halves, 2 = quarters, 3 = eighths, 4 = sixteenths.
    pub level: u8,
    /// Largest jump between consecutive notes, in staff steps (2 = a third).
    pub max_leap: u8,
    pub rests: bool,
    pub triplets: bool,
    /// Dotted pairs such as a dotted eighth plus a sixteenth.
    pub dotted: bool,
    pub ties: bool,
    pub sharps: bool,
    pub flats: bool,
    /// Written for a transposing instrument: the sound follows `instrument.transpose()`.
    pub instrument: Instrument,
    pub seed: u64,
}

impl Default for Settings {
    fn default() -> Self {
        let (low, high) = Clef::Treble.default_range();
        Settings {
            meter: Meter { num: 4, den: 4 },
            key: Key::C_MAJOR,
            clef: Clef::Treble,
            tempo: 60,
            low,
            high,
            level: 2,
            max_leap: 3,
            rests: false,
            triplets: false,
            dotted: false,
            ties: false,
            sharps: false,
            flats: false,
            instrument: Instrument::Concert,
            seed: 0,
        }
    }
}

impl Settings {
    pub fn smallest(&self) -> Dur {
        match self.level {
            1 => HALF,
            2 => QUARTER,
            3 => EIGHTH,
            _ => SIXTEENTH,
        }
    }

    pub fn validate(&self) -> Result<(), ParseError> {
        let fail = |m: String| Err(ParseError(m));
        if !(1..=4).contains(&self.level) {
            return fail(format!("level must be 1 to 4, got {}", self.level));
        }
        if !(20..=400).contains(&self.tempo) {
            return fail(format!("tempo must be 20 to 400, got {}", self.tempo));
        }
        if self.low.step >= self.high.step {
            return fail(format!(
                "the range {}..{} is empty: the low note must be below the high note",
                self.low, self.high
            ));
        }
        let t = self.instrument.transpose();
        let sounding = [
            self.low.midi(),
            self.high.midi(),
            self.low.midi() + t,
            self.high.midi() + t,
        ];
        if sounding.iter().any(|m| !(0..=127).contains(m)) {
            return fail("the range must stay within MIDI notes 0 to 127".into());
        }
        if self.max_leap == 0 {
            return fail("max leap must be at least 1".into());
        }
        Ok(())
    }
}
