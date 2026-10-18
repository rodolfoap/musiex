//! The endless bar generator.

use crate::melody::MelodyGen;
use crate::model::{Bar, Event};
use crate::pitch::ParseError;
use crate::rhythm::RhythmGen;
use crate::rng::Rng;
use crate::settings::Settings;

/// Produces bars forever. The same settings (seed included) always produce
/// the same bars, so callers can take as many as they need, when they need them.
#[derive(Debug, Clone)]
pub struct Generator {
    settings: Settings,
    rng: Rng,
    melody: MelodyGen,
    /// The next bar's rhythm, generated one bar early so ties can cross barlines.
    lookahead: Vec<Event>,
    index: u64,
}

impl Generator {
    pub fn new(settings: Settings) -> Result<Self, ParseError> {
        settings.validate()?;
        let mut rng = Rng::new(settings.seed);
        let lookahead = RhythmGen::new(&settings).bar(&mut rng);
        Ok(Generator {
            settings,
            rng,
            melody: MelodyGen::new(),
            lookahead,
            index: 0,
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }
}

impl Iterator for Generator {
    type Item = Bar;

    fn next(&mut self) -> Option<Bar> {
        let next = RhythmGen::new(&self.settings).bar(&mut self.rng);
        let mut events = std::mem::replace(&mut self.lookahead, next);
        self.melody
            .pitch_bar(&self.settings, &mut self.rng, &mut events, &self.lookahead);
        let bar = Bar {
            index: self.index,
            events,
        };
        self.index += 1;
        Some(bar)
    }
}
