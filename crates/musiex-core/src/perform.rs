//! Turning bars into timed notes for MIDI and audio, with an optional metronome.
//!
//! Times are in ticks at `duration::PPQ` ticks per quarter note.

use crate::duration::Dur;
use crate::model::{Bar, Event, Note};
use crate::settings::Settings;

pub const MELODY_CHANNEL: u8 = 0;
/// General MIDI percussion channel (channel 10, counted from 1).
pub const CLICK_CHANNEL: u8 = 9;
const CLICK_ACCENT: u8 = 76; // Hi Wood Block
const CLICK_BEAT: u8 = 77; // Low Wood Block
const CLICK_LEN: u64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayedNote {
    pub start: u64,
    pub dur: u64,
    pub key: u8,
    pub velocity: u8,
    pub channel: u8,
}

/// Microseconds per quarter note for the settings' tempo, as MIDI files want it.
pub fn micros_per_quarter(s: &Settings) -> u32 {
    let beat_quarters = f64::from(s.meter.beat()) / f64::from(crate::duration::QUARTER);
    (60_000_000.0 / f64::from(s.tempo) / beat_quarters).round() as u32
}

/// Converts bars to notes one bar at a time, so playback can be fed from an
/// endless generator. Tied notes are merged into one sounding note.
#[derive(Debug, Clone)]
pub struct Performer {
    bar_len: Dur,
    beat: Dur,
    metronome: bool,
    /// Where the next bar starts.
    cursor: u64,
    /// A note whose tie continues into the next bar.
    held: Option<PlayedNote>,
}

impl Performer {
    pub fn new(s: &Settings, metronome: bool) -> Self {
        Performer {
            bar_len: s.meter.bar_len(),
            beat: s.meter.beat(),
            metronome,
            cursor: 0,
            held: None,
        }
    }

    /// Tick at which the next bar will start.
    pub fn position(&self) -> u64 {
        self.cursor
    }

    /// Bars of clicks only, before the music.
    pub fn count_in(&mut self, bars: u32) -> Vec<PlayedNote> {
        let mut out = Vec::new();
        for _ in 0..bars {
            self.clicks(&mut out);
            self.cursor += u64::from(self.bar_len);
        }
        out
    }

    pub fn push_bar(&mut self, bar: &Bar) -> Vec<PlayedNote> {
        let mut out = Vec::new();
        if self.metronome {
            self.clicks(&mut out);
        }
        let mut t = self.cursor;
        for e in &bar.events {
            match e {
                Event::Rest(d) => {
                    self.release(&mut out);
                    t += u64::from(*d);
                }
                Event::Note(n) => {
                    self.play(n, t, u64::from(n.dur), &mut out);
                    t += u64::from(n.dur);
                }
                Event::Tuplet(tup) => {
                    for n in &tup.notes {
                        let d = u64::from(tup.actual(n.dur));
                        self.play(n, t, d, &mut out);
                        t += d;
                    }
                }
            }
        }
        self.cursor += u64::from(self.bar_len);
        out.sort_by_key(|n| n.start);
        out
    }

    /// Releases any note still held by a tie.
    pub fn finish(&mut self) -> Vec<PlayedNote> {
        let mut out = Vec::new();
        self.release(&mut out);
        out
    }

    fn play(&mut self, n: &Note, start: u64, dur: u64, out: &mut Vec<PlayedNote>) {
        let key = n.pitch.midi().clamp(0, 127) as u8;
        match &mut self.held {
            Some(h) if h.key == key && h.start + h.dur == start => h.dur += dur,
            _ => {
                self.release(out);
                self.held = Some(PlayedNote {
                    start,
                    dur,
                    key,
                    velocity: 90,
                    channel: MELODY_CHANNEL,
                });
            }
        }
        if !n.tie {
            self.release(out);
        }
    }

    fn release(&mut self, out: &mut Vec<PlayedNote>) {
        if let Some(h) = self.held.take() {
            out.push(h);
        }
    }

    fn clicks(&self, out: &mut Vec<PlayedNote>) {
        let beats = self.bar_len / self.beat;
        for b in 0..beats {
            out.push(PlayedNote {
                start: self.cursor + u64::from(b * self.beat),
                dur: CLICK_LEN,
                key: if b == 0 { CLICK_ACCENT } else { CLICK_BEAT },
                velocity: if b == 0 { 110 } else { 80 },
                channel: CLICK_CHANNEL,
            });
        }
    }
}

/// Performs a finite list of bars in one go.
pub fn perform(s: &Settings, bars: &[Bar], metronome: bool, count_in: u32) -> Vec<PlayedNote> {
    let mut p = Performer::new(s, metronome);
    let mut out = p.count_in(count_in);
    for b in bars {
        out.extend(p.push_bar(b));
    }
    out.extend(p.finish());
    out.sort_by_key(|n| (n.start, n.channel));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::QUARTER;
    use crate::pitch::Pitch;

    #[test]
    fn tempo_in_compound_meter() {
        let s = Settings {
            meter: "6/8".parse().unwrap(),
            tempo: 60,
            ..Settings::default()
        };
        // A dotted quarter per second: a quarter lasts 2/3 s.
        assert_eq!(micros_per_quarter(&s), 666_667);
    }

    #[test]
    fn ties_merge_across_bars() {
        let s = Settings::default();
        let c = Pitch::MIDDLE_C;
        let note = |tie| {
            Event::Note(Note {
                pitch: c,
                dur: 2 * QUARTER,
                tie,
            })
        };
        let bars = vec![
            Bar {
                index: 0,
                events: vec![note(false), note(true)],
            },
            Bar {
                index: 1,
                events: vec![note(false), Event::Rest(2 * QUARTER)],
            },
        ];
        let played = perform(&s, &bars, false, 0);
        assert_eq!(played.len(), 2);
        assert_eq!(played[1].start, u64::from(2 * QUARTER));
        assert_eq!(played[1].dur, u64::from(4 * QUARTER));
    }

    #[test]
    fn count_in_and_clicks() {
        let s = Settings::default();
        let bars = vec![Bar {
            index: 0,
            events: vec![Event::Rest(4 * QUARTER)],
        }];
        let played = perform(&s, &bars, true, 1);
        let clicks: Vec<_> = played
            .iter()
            .filter(|n| n.channel == CLICK_CHANNEL)
            .collect();
        assert_eq!(clicks.len(), 8);
        assert_eq!(clicks[4].start, u64::from(4 * QUARTER));
        assert_eq!(clicks[4].key, CLICK_ACCENT);
    }
}
