//! Properties every generated exercise must have, checked over many seeds and settings.

use musiex_core::duration::is_writable;
use musiex_core::{abc, Event, Generator, Note, Settings};

fn variants() -> Vec<Settings> {
    let meters = [
        "2/4", "3/4", "4/4", "5/4", "3/8", "6/8", "7/8", "9/8", "12/8", "2/2",
    ];
    let keys = ["C", "G", "F", "Bb", "D", "Em", "C#m", "Ab"];
    let mut out = Vec::new();
    for (i, m) in meters.iter().enumerate() {
        for level in 1..=4 {
            let all = i % 2 == 0;
            out.push(Settings {
                meter: m.parse().unwrap(),
                key: keys[(i + level as usize) % keys.len()].parse().unwrap(),
                level,
                rests: all,
                triplets: all,
                dotted: !all || level == 4,
                ties: true,
                sharps: all,
                flats: !all,
                max_leap: 1 + (level % 4),
                seed: (i as u64) * 31 + u64::from(level),
                ..Settings::default()
            });
        }
    }
    out
}

fn all_notes(events: &[Event]) -> Vec<&Note> {
    events
        .iter()
        .flat_map(|e| match e {
            Event::Note(n) => vec![n],
            Event::Tuplet(t) => t.notes.iter().collect(),
            Event::Rest(_) => vec![],
        })
        .collect()
}

#[test]
fn bars_fill_the_meter_exactly() {
    for s in variants() {
        for bar in Generator::new(s.clone()).unwrap().take(200) {
            assert_eq!(
                bar.dur(),
                s.meter.bar_len(),
                "meter {} bar {:?}",
                s.meter,
                bar
            );
        }
    }
}

#[test]
fn every_value_is_writable_and_not_below_the_level() {
    for s in variants() {
        for bar in Generator::new(s.clone()).unwrap().take(200) {
            for e in &bar.events {
                match e {
                    Event::Note(n) => assert!(is_writable(n.dur), "{:?}", n),
                    Event::Rest(d) => assert!(is_writable(*d), "rest {d}"),
                    Event::Tuplet(t) => assert!(t.notes.iter().all(|n| is_writable(n.dur))),
                }
            }
            // A bar group shorter than the level's smallest value (a 3/8
            // group at level 1, say) cannot respect it; everything else must.
            if s.meter.beat() >= s.smallest() {
                for n in all_notes(&bar.events) {
                    assert!(
                        n.dur >= s.smallest(),
                        "{:?} meter {} level {}",
                        n,
                        s.meter,
                        s.level
                    );
                }
            }
        }
    }
}

#[test]
fn pitches_stay_in_range_and_leaps_are_bounded() {
    for s in variants() {
        let mut prev: Option<i32> = None;
        for bar in Generator::new(s.clone()).unwrap().take(200) {
            for n in all_notes(&bar.events) {
                assert!((s.low.step..=s.high.step).contains(&n.pitch.step));
                if let Some(p) = prev {
                    assert!((n.pitch.step - p).abs() <= i32::from(s.max_leap));
                }
                prev = Some(n.pitch.step);
            }
        }
    }
}

#[test]
fn ties_join_equal_pitches_and_never_reach_rests() {
    for s in variants() {
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(200).collect();
        let events: Vec<&Event> = bars.iter().flat_map(|b| b.events.iter()).collect();
        for pair in events.windows(2) {
            match pair {
                [Event::Note(a), next] if a.tie => match next {
                    Event::Note(b) => assert_eq!(a.pitch, b.pitch),
                    other => panic!("tie from {:?} into {:?}", a, other),
                },
                _ => {}
            }
        }
        for e in &events {
            if let Event::Tuplet(t) = e {
                assert!(t.notes.iter().all(|n| !n.tie));
            }
        }
    }
}

#[test]
fn accidentals_only_when_asked() {
    let s = Settings {
        key: "D".parse().unwrap(),
        ..Settings::default()
    };
    for bar in Generator::new(s.clone()).unwrap().take(200) {
        for n in bar.notes() {
            assert_eq!(n.pitch, s.key.diatonic(n.pitch.step));
        }
    }
}

#[test]
fn same_seed_same_music_different_seed_different_music() {
    let s = Settings {
        level: 3,
        rests: true,
        triplets: true,
        ..Settings::default()
    };
    let a: Vec<_> = Generator::new(s.clone()).unwrap().take(50).collect();
    let b: Vec<_> = Generator::new(s.clone()).unwrap().take(50).collect();
    assert_eq!(a, b);
    let c: Vec<_> = Generator::new(Settings { seed: 1, ..s })
        .unwrap()
        .take(50)
        .collect();
    assert_ne!(a, c);
}

#[test]
fn taking_more_bars_does_not_change_earlier_ones() {
    let s = Settings::default();
    let short: Vec<_> = Generator::new(s.clone()).unwrap().take(10).collect();
    let long: Vec<_> = Generator::new(s).unwrap().take(100).collect();
    assert_eq!(short[..9], long[..9]);
}

#[test]
fn levels_produce_their_note_values() {
    use musiex_core::duration::{EIGHTH, SIXTEENTH};
    let shortest = |level| {
        let s = Settings {
            level,
            ..Settings::default()
        };
        Generator::new(s)
            .unwrap()
            .take(100)
            .flat_map(|b| b.notes().map(|n| n.dur).collect::<Vec<_>>())
            .min()
            .unwrap()
    };
    assert_eq!(shortest(3), EIGHTH);
    assert_eq!(shortest(4), SIXTEENTH);
}

#[test]
fn abc_has_one_barline_per_bar() {
    for s in variants() {
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(17).collect();
        let text = abc::tune(&s, &abc::Header::default(), &bars, 4);
        let body: String = text
            .lines()
            .filter(|l| !l.contains(':'))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(body.matches('|').count(), 17, "{body}");
        assert!(body.trim_end().ends_with("|]"));
        assert!(!body.contains("- |]"), "tie at the very end: {body}");
    }
}

#[test]
fn rejects_bad_settings() {
    let bad = |s: Settings| Generator::new(s).is_err();
    assert!(bad(Settings {
        level: 0,
        ..Settings::default()
    }));
    assert!(bad(Settings {
        tempo: 5,
        ..Settings::default()
    }));
    assert!(bad(Settings {
        max_leap: 0,
        ..Settings::default()
    }));
    let s = Settings::default();
    assert!(bad(Settings {
        low: s.high,
        high: s.low,
        ..s
    }));
}
