//! Standard MIDI file output.

use anyhow::Result;
use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use musiex_core::duration::PPQ;
use musiex_core::perform::{micros_per_quarter, perform, MELODY_CHANNEL};
use musiex_core::{Bar, Settings};

/// Acoustic Grand Piano.
const PROGRAM: u8 = 0;

pub fn write(s: &Settings, bars: &[Bar], metronome: bool, count_in: u32) -> Result<Vec<u8>> {
    let played = perform(s, bars, metronome, count_in);

    // (time, order, kind): note-offs sort before note-ons at the same tick so
    // repeated pitches retrigger cleanly.
    let mut timed: Vec<(u64, u8, TrackEventKind<'static>)> = Vec::new();
    let meta = |m| TrackEventKind::Meta(m);
    timed.push((
        0,
        0,
        meta(MetaMessage::Tempo(u24::new(micros_per_quarter(s)))),
    ));
    timed.push((
        0,
        0,
        meta(MetaMessage::TimeSignature(
            s.meter.num,
            s.meter.den.trailing_zeros() as u8,
            24,
            8,
        )),
    ));
    timed.push((
        0,
        0,
        meta(MetaMessage::KeySignature(
            s.key.fifths,
            s.key.mode == musiex_core::pitch::Mode::Minor,
        )),
    ));
    timed.push((
        0,
        0,
        TrackEventKind::Midi {
            channel: u4::new(MELODY_CHANNEL),
            message: MidiMessage::ProgramChange {
                program: u7::new(PROGRAM),
            },
        },
    ));
    let mut end = 0;
    for n in &played {
        let channel = u4::new(n.channel);
        let key = u7::new(n.key);
        timed.push((
            n.start,
            2,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn {
                    key,
                    vel: u7::new(n.velocity),
                },
            },
        ));
        timed.push((
            n.start + n.dur,
            1,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOff {
                    key,
                    vel: u7::new(0),
                },
            },
        ));
        end = end.max(n.start + n.dur);
    }
    timed.push((end, 3, meta(MetaMessage::EndOfTrack)));
    timed.sort_by_key(|(t, order, _)| (*t, *order));

    let mut track = Vec::with_capacity(timed.len());
    let mut last = 0;
    for (t, _, kind) in timed {
        track.push(TrackEvent {
            delta: u28::new((t - last) as u32),
            kind,
        });
        last = t;
    }

    let smf = Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(u15::new(PPQ as u16))),
        tracks: vec![track],
    };
    let mut out = Vec::new();
    smf.write_std(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiex_core::Generator;

    #[test]
    fn round_trips_through_a_midi_parser() {
        let s = Settings {
            ties: true,
            triplets: true,
            level: 3,
            ..Settings::default()
        };
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(16).collect();
        let bytes = write(&s, &bars, true, 1).unwrap();
        let smf = Smf::parse(&bytes).unwrap();
        assert_eq!(smf.tracks.len(), 1);
        let ons = smf.tracks[0]
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    TrackEventKind::Midi {
                        message: MidiMessage::NoteOn { .. },
                        ..
                    }
                )
            })
            .count();
        let offs = smf.tracks[0]
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    TrackEventKind::Midi {
                        message: MidiMessage::NoteOff { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(ons, offs);
        // 17 bars of four clicks, plus at least one note per bar.
        assert!(ons >= 17 * 4 + 16);
        let ticks: u64 = smf.tracks[0]
            .iter()
            .map(|e| u64::from(e.delta.as_int()))
            .sum();
        assert_eq!(ticks, 17 * 4 * u64::from(PPQ));
    }
}
