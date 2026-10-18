//! Rhythm generation: each bar is split recursively into note values.
//!
//! A span either stays as one note (or rest), becomes a special figure
//! (triplet, dotted pair), or splits into smaller spans. How a span may split
//! depends on whether it lives in a simple or a compound context: a dotted
//! quarter in 6/8 splits into three eighths, a half in 4/4 into two quarters.

use crate::duration::{is_binary, is_dotted, is_writable, Dur, HALF, WHOLE};
use crate::meter::Meter;
use crate::model::{Event, Note, Tuplet};
use crate::pitch::Pitch;
use crate::rng::Rng;
use crate::settings::Settings;

/// Probability that a single note value becomes a rest. Long rests are kept
/// rare: a reading exercise is about notes.
fn p_rest(d: Dur) -> f64 {
    if d >= HALF {
        0.04
    } else {
        0.15
    }
}
const P_TRIPLET: f64 = 0.12;
const P_DOTTED: f64 = 0.12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Simple,
    Compound,
}

type Partition = Vec<(Dur, Ctx)>;

/// Probability that a span which could still split stays as a single note.
/// Long spans usually split, so whole-bar notes stay rare.
fn p_keep(d: Dur) -> f64 {
    if d >= WHOLE {
        0.05
    } else if d >= HALF {
        0.2
    } else {
        0.4
    }
}

fn partitions(d: Dur, ctx: Ctx, meter: Meter) -> Vec<Partition> {
    use Ctx::*;
    match ctx {
        Simple if is_binary(d) => vec![vec![(d / 2, Simple); 2]],
        Simple if is_dotted(d) => {
            let t = d / 3;
            vec![
                vec![(t, Simple); 3],
                vec![(2 * t, Simple), (t, Simple)],
                vec![(t, Simple), (2 * t, Simple)],
            ]
        }
        Simple => Vec::new(),
        Compound => {
            let beat = meter.beat();
            let k = d / beat;
            if k == 1 {
                let t = d / 3;
                vec![vec![(t, Simple); 3], vec![(2 * t, Simple), (t, Simple)]]
            } else if k == 3 {
                vec![
                    vec![(beat, Compound); 3],
                    vec![(2 * beat, Compound), (beat, Compound)],
                ]
            } else if k.is_power_of_two() {
                vec![vec![(d / 2, Compound); 2]]
            } else {
                Vec::new()
            }
        }
    }
}

pub(crate) struct RhythmGen<'a> {
    settings: &'a Settings,
}

impl<'a> RhythmGen<'a> {
    pub(crate) fn new(settings: &'a Settings) -> Self {
        RhythmGen { settings }
    }

    /// One bar of rhythm. Notes carry a placeholder pitch for the melody pass.
    pub(crate) fn bar(&self, rng: &mut Rng) -> Vec<Event> {
        let meter = self.settings.meter;
        let ctx = if meter.is_compound() {
            Ctx::Compound
        } else {
            Ctx::Simple
        };
        let mut out = Vec::new();
        for beats in meter.groups() {
            self.fill(beats * meter.beat(), ctx, rng, &mut out);
        }
        out
    }

    fn fill(&self, d: Dur, ctx: Ctx, rng: &mut Rng, out: &mut Vec<Event>) {
        let s = self.settings;
        let min = s.smallest();
        let all = partitions(d, ctx, s.meter);
        let parts: Vec<Partition> = all
            .iter()
            .filter(|p| p.iter().all(|&(c, _)| c >= min))
            .cloned()
            .collect();

        if !is_writable(d) && parts.is_empty() {
            // The span cannot be one note and every split goes below the
            // smallest value (e.g. 9/8 at level 1): split anyway.
            let first = all
                .into_iter()
                .next()
                .expect("unwritable spans always split");
            for (c, cctx) in first {
                self.fill(c, cctx, rng, out);
            }
            return;
        }

        if is_writable(d) {
            let figure_ok = is_binary(d) && d <= HALF;
            // Compound meters already divide in three; triplets there read as
            // sextuplets, so they are kept to simple meters.
            if s.triplets
                && figure_ok
                && !s.meter.is_compound()
                && d / 2 >= min
                && rng.chance(P_TRIPLET)
            {
                out.push(Event::Tuplet(Tuplet {
                    p: 3,
                    q: 2,
                    notes: vec![note(d / 2); 3],
                }));
                return;
            }
            if s.dotted && figure_ok && d / 4 >= min && rng.chance(P_DOTTED) {
                out.push(Event::Note(note(d / 4 * 3)));
                out.push(Event::Note(note(d / 4)));
                return;
            }
            if parts.is_empty() || rng.chance(p_keep(d)) {
                if s.rests && rng.chance(p_rest(d)) {
                    out.push(Event::Rest(d));
                } else {
                    out.push(Event::Note(note(d)));
                }
                return;
            }
        }

        // The even split is the most common; the others add variety.
        let weights: Vec<u32> = (0..parts.len())
            .map(|i| if i == 0 { 3 } else { 1 })
            .collect();
        let chosen = rng.weighted(&weights).expect("parts is not empty here");
        let start = out.len();
        for &(c, cctx) in &parts[chosen] {
            self.fill(c, cctx, rng, out);
        }
        // Sibling rests read better as one: `z z` becomes `z2`.
        if is_writable(d) && out[start..].iter().all(|e| matches!(e, Event::Rest(_))) {
            out.truncate(start);
            out.push(Event::Rest(d));
        }
    }
}

fn note(dur: Dur) -> Note {
    Note {
        pitch: Pitch::MIDDLE_C,
        dur,
        tie: false,
    }
}
