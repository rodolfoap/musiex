//! Engraving decisions shared by every score output (ABC, SVG): which
//! accidentals are shown and where beams break.

use crate::duration::{Dur, QUARTER};
use crate::meter::Meter;
use crate::model::{Bar, Event};
use crate::pitch::Key;
use std::collections::HashMap;

/// The accidental to print before each note of `bar` (in `Bar::notes` order),
/// or `None` when the key signature and earlier notes in the bar already
/// imply the pitch. `Some(0)` is a natural sign.
pub fn accidentals(key: Key, bar: &Bar) -> Vec<Option<i8>> {
    let mut state: HashMap<i32, i8> = HashMap::new();
    bar.notes()
        .map(|n| {
            let effective = state
                .get(&n.pitch.step)
                .copied()
                .unwrap_or_else(|| key.alter_for(n.pitch.letter()));
            if n.pitch.alter == effective {
                None
            } else {
                state.insert(n.pitch.step, n.pitch.alter);
                Some(n.pitch.alter)
            }
        })
        .collect()
}

/// Whether an event can share a beam: notes (or tuplets of notes) shorter
/// than a quarter.
pub fn beamable(e: &Event) -> bool {
    match e {
        Event::Note(n) => n.dur < QUARTER,
        Event::Rest(_) => false,
        Event::Tuplet(t) => t.notes.iter().all(|n| n.dur < QUARTER),
    }
}

/// For each event, whether it is beamed to the previous one. Beams never
/// cross a beat, so the pulse of the meter stays visible.
pub fn beamed_to_previous(meter: Meter, events: &[Event]) -> Vec<bool> {
    let beat = meter.beat();
    let mut out = Vec::with_capacity(events.len());
    let mut onset: Dur = 0;
    let mut prev = false;
    for (i, e) in events.iter().enumerate() {
        let b = beamable(e);
        out.push(i > 0 && b && prev && !onset.is_multiple_of(beat));
        onset += e.dur();
        prev = b;
    }
    out
}
