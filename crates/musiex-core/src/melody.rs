//! Melody generation: a bounded random walk over staff positions.

use crate::duration::Dur;
use crate::model::Event;
use crate::pitch::Pitch;
use crate::rng::Rng;
use crate::settings::Settings;

const P_TIE: f64 = 0.15;
const P_ACCIDENTAL: f64 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// A note outside tuplets; `on_beat` when it starts on a beat.
    Plain {
        on_beat: bool,
    },
    /// A note inside a tuplet; ties into or out of tuplets are not generated.
    InTuplet,
    Rest,
}

fn slots(events: &[Event], beat: Dur) -> Vec<Slot> {
    let mut out = Vec::new();
    let mut onset = 0;
    for e in events {
        match e {
            Event::Note(_) => out.push(Slot::Plain {
                on_beat: onset % beat == 0,
            }),
            Event::Rest(_) => out.push(Slot::Rest),
            Event::Tuplet(t) => out.extend(std::iter::repeat(Slot::InTuplet).take(t.notes.len())),
        }
        onset += e.dur();
    }
    out
}

#[derive(Debug, Clone)]
pub(crate) struct MelodyGen {
    prev: Option<Pitch>,
    tie_pending: bool,
}

impl MelodyGen {
    pub(crate) fn new() -> Self {
        MelodyGen {
            prev: None,
            tie_pending: false,
        }
    }

    /// Fills in pitches and ties for one bar. `next_bar` is the following bar's
    /// rhythm, needed to decide whether the last note can tie over the barline.
    pub(crate) fn pitch_bar(
        &mut self,
        s: &Settings,
        rng: &mut Rng,
        bar: &mut [Event],
        next_bar: &[Event],
    ) {
        let beat = s.meter.beat();
        let kinds = slots(bar, beat);
        let next_first = slots(next_bar, beat).first().copied();
        let mut i = 0;
        for e in bar.iter_mut() {
            match e {
                Event::Rest(_) => {
                    self.tie_pending = false;
                    i += 1;
                }
                Event::Note(n) => {
                    n.pitch = self.next_pitch(s, rng);
                    let next = kinds.get(i + 1).copied().or(next_first);
                    // Ties are only generated into a beat or across the
                    // barline, where they teach syncopation; a tie inside a
                    // beat would just be a longer note written the hard way.
                    let into_beat = next == Some(Slot::Plain { on_beat: true });
                    self.tie_pending = s.ties && into_beat && rng.chance(P_TIE);
                    n.tie = self.tie_pending;
                    i += 1;
                }
                Event::Tuplet(t) => {
                    for n in &mut t.notes {
                        n.pitch = self.next_pitch(s, rng);
                        n.tie = false;
                        self.tie_pending = false;
                        i += 1;
                    }
                }
            }
        }
    }

    fn next_pitch(&mut self, s: &Settings, rng: &mut Rng) -> Pitch {
        let pitch = match self.prev {
            Some(prev) if self.tie_pending => prev,
            Some(prev) => {
                let step = walk(prev.step, s, rng);
                with_accidental(step, s, rng)
            }
            None => s.key.diatonic((s.low.step + s.high.step) / 2),
        };
        self.prev = Some(pitch);
        pitch
    }
}

fn walk(from: i32, s: &Settings, rng: &mut Rng) -> i32 {
    let (lo, hi) = (s.low.step, s.high.step);
    let leap = i32::from(s.max_leap);
    let mut step = from + rng.range(-leap, leap);
    // Reflect off the range edges so the melody turns around instead of
    // sticking to the boundary.
    if step < lo {
        step = lo + (lo - step);
    }
    if step > hi {
        step = hi - (step - hi);
    }
    step.clamp(lo, hi)
}

fn with_accidental(step: i32, s: &Settings, rng: &mut Rng) -> Pitch {
    let natural = s.key.diatonic(step);
    if !(s.sharps || s.flats) || !rng.chance(P_ACCIDENTAL) {
        return natural;
    }
    // E#, B#, Cb and Fb are correct spellings but rarely what a reader
    // practises, so a raised E or B, or a lowered C or F, is not generated.
    let letter = natural.letter_name();
    let mut options = Vec::with_capacity(2);
    if s.sharps && natural.alter < 1 && !(natural.alter == 0 && matches!(letter, 'E' | 'B')) {
        options.push(natural.alter + 1);
    }
    if s.flats && natural.alter > -1 && !(natural.alter == 0 && matches!(letter, 'C' | 'F')) {
        options.push(natural.alter - 1);
    }
    if options.is_empty() {
        return natural;
    }
    let alter = options[rng.below(options.len() as u32) as usize];
    Pitch::new(step, alter)
}
