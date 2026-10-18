//! ABC notation output.
//!
//! The score is written with `L:1/8`, beams broken at every beat, and
//! accidentals written relative to the key signature and to earlier notes in
//! the same bar, as engraved music does.

use crate::duration::{Dur, EIGHTH, WHOLE};
use crate::model::{Bar, Event, Note};
use crate::notation;
use crate::pitch::Pitch;
use crate::settings::Settings;
use std::fmt::Write;

/// The unit note length, `L:1/8`.
const UNIT: Dur = EIGHTH;

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub title: String,
    pub composer: Option<String>,
    pub notes: Option<String>,
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// A duration as an ABC length suffix relative to `unit`: "", "2", "/", "3/2"...
fn length(d: Dur, unit: Dur) -> String {
    let g = gcd(d, unit);
    match (d / g, unit / g) {
        (1, 1) => String::new(),
        (n, 1) => n.to_string(),
        (1, 2) => "/".to_string(),
        (1, den) => format!("/{den}"),
        (n, den) => format!("{n}/{den}"),
    }
}

fn fraction(d: Dur) -> String {
    let g = gcd(d, WHOLE);
    format!("{}/{}", d / g, WHOLE / g)
}

fn pitch_name(p: Pitch) -> String {
    let letter = p.letter_name();
    let octave = p.octave();
    if octave >= 5 {
        let mut s = letter.to_ascii_lowercase().to_string();
        s.extend(std::iter::repeat_n('\'', (octave - 5) as usize));
        s
    } else {
        let mut s = letter.to_string();
        s.extend(std::iter::repeat_n(',', (4 - octave) as usize));
        s
    }
}

pub fn header(s: &Settings, h: &Header) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "X:1");
    let _ = writeln!(out, "T:{}", h.title);
    if let Some(c) = &h.composer {
        let _ = writeln!(out, "C:{c}");
    }
    if let Some(n) = &h.notes {
        let _ = writeln!(out, "N:{n}");
    }
    let _ = writeln!(out, "M:{}", s.meter);
    let _ = writeln!(out, "L:{}", fraction(UNIT));
    let _ = writeln!(out, "Q:{}={}", fraction(s.meter.beat()), s.tempo);
    let t = s.instrument.transpose();
    if t != 0 {
        // Written pitch on the page; players such as abcjs sound it transposed.
        let _ = writeln!(out, "%%MIDI transpose {t}");
    }
    let _ = writeln!(out, "K:{} clef={}", s.key, s.clef.name());
    out
}

/// Writes one bar, ending with its barline. `last` closes the piece with a
/// final barline and drops a tie that would lead nowhere.
pub fn bar(s: &Settings, bar: &Bar, last: bool) -> String {
    let mut accidentals = notation::accidentals(s.key, bar).into_iter();
    let mut note_text = |n: &Note, tie: bool| {
        let mut t = String::new();
        if let Some(alter) = accidentals.next().flatten() {
            t.push(match alter {
                1 => '^',
                -1 => '_',
                _ => '=',
            });
        }
        t.push_str(&pitch_name(n.pitch));
        t.push_str(&length(n.dur, UNIT));
        if tie {
            t.push('-');
        }
        t
    };

    let joined = notation::beamed_to_previous(s.meter, &bar.events);
    let mut out = String::new();
    let last_index = bar.events.len().saturating_sub(1);
    for (i, e) in bar.events.iter().enumerate() {
        let text = match e {
            Event::Note(n) => note_text(n, n.tie && !(last && i == last_index)),
            Event::Rest(d) => format!("z{}", length(*d, UNIT)),
            Event::Tuplet(t) => {
                let mut text = format!("({}", t.p);
                for n in &t.notes {
                    text.push_str(&note_text(n, false));
                }
                text
            }
        };
        if i > 0 && !joined[i] {
            out.push(' ');
        }
        out.push_str(&text);
    }
    out.push_str(if last { " |]" } else { " |" });
    out
}

/// A complete tune: header plus bars, `bars_per_line` bars on each line.
pub fn tune(s: &Settings, h: &Header, bars: &[Bar], bars_per_line: usize) -> String {
    let mut out = header(s, h);
    let per_line = bars_per_line.max(1);
    for (i, b) in bars.iter().enumerate() {
        let last = i + 1 == bars.len();
        out.push_str(&bar(s, b, last));
        out.push(if (i + 1).is_multiple_of(per_line) || last {
            '\n'
        } else {
            ' '
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::{HALF, QUARTER, SIXTEENTH};

    #[test]
    fn lengths() {
        assert_eq!(length(EIGHTH, UNIT), "");
        assert_eq!(length(QUARTER, UNIT), "2");
        assert_eq!(length(3 * QUARTER, UNIT), "6");
        assert_eq!(length(SIXTEENTH, UNIT), "/");
        assert_eq!(length(SIXTEENTH / 2, UNIT), "/4");
        assert_eq!(length(3 * SIXTEENTH, UNIT), "3/2");
        assert_eq!(length(HALF, UNIT), "4");
        assert_eq!(fraction(3 * EIGHTH), "3/8");
    }

    #[test]
    fn pitch_names() {
        let p = |s: &str| pitch_name(s.parse().unwrap());
        assert_eq!(p("C4"), "C");
        assert_eq!(p("B3"), "B,");
        assert_eq!(p("C2"), "C,,");
        assert_eq!(p("C5"), "c");
        assert_eq!(p("D6"), "d'");
    }

    fn n(p: &str, dur: Dur) -> Event {
        Event::Note(Note {
            pitch: p.parse().unwrap(),
            dur,
            tie: false,
        })
    }

    #[test]
    fn accidentals_follow_key_and_bar() {
        let s = Settings {
            key: "G".parse().unwrap(),
            ..Settings::default()
        };
        let b = Bar {
            index: 0,
            events: vec![
                n("F#4", QUARTER),
                n("F4", QUARTER),
                n("F4", QUARTER),
                n("F#4", QUARTER),
            ],
        };
        assert_eq!(bar(&s, &b, false), "F2 =F2 F2 ^F2 |");
    }

    #[test]
    fn beams_break_at_beats() {
        let s = Settings::default();
        let b = Bar {
            index: 0,
            events: vec![
                n("C4", EIGHTH),
                n("D4", EIGHTH),
                n("E4", EIGHTH),
                n("F4", EIGHTH),
                Event::Rest(QUARTER),
                n("G4", QUARTER),
            ],
        };
        assert_eq!(bar(&s, &b, true), "CD EF z2 G2 |]");
    }
}
