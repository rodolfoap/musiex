//! Sequencing timed notes into an [`Engine`].

use crate::Engine;
use musiex_core::duration::PPQ;
use musiex_core::perform::PlayedNote;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    // Offs sort before ons at the same tick so repeated pitches retrigger.
    Off,
    On(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Event {
    tick: u64,
    kind: Kind,
    channel: u8,
    key: u8,
}

impl Event {
    fn order(&self) -> (u64, u8) {
        (self.tick, matches!(self.kind, Kind::On(_)) as u8)
    }
}

/// Plays scheduled notes through an engine. Time is kept in ticks, so the
/// tempo can change at any moment without rescheduling anything.
pub struct Player<E: Engine> {
    engine: E,
    sample_rate: u32,
    samples_per_tick: f64,
    /// Current playback position, in ticks.
    tick: f64,
    events: VecDeque<Event>,
}

impl<E: Engine> Player<E> {
    pub fn new(engine: E, sample_rate: u32, micros_per_quarter: u32) -> Self {
        let mut p = Player {
            engine,
            sample_rate,
            samples_per_tick: 0.0,
            tick: 0.0,
            events: VecDeque::new(),
        };
        p.set_tempo(micros_per_quarter);
        p
    }

    pub fn set_tempo(&mut self, micros_per_quarter: u32) {
        self.samples_per_tick = f64::from(self.sample_rate) * f64::from(micros_per_quarter)
            / 1_000_000.0
            / f64::from(PPQ);
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Playback position in ticks, for following the score.
    pub fn position(&self) -> f64 {
        self.tick
    }

    /// Tick of the last scheduled event, or the position if nothing is pending.
    pub fn scheduled_until(&self) -> f64 {
        self.events
            .back()
            .map_or(self.tick, |e| (e.tick as f64).max(self.tick))
    }

    pub fn engine_mut(&mut self) -> &mut E {
        &mut self.engine
    }

    /// Adds notes to play. Notes may arrive in any order and at any time, as
    /// long as they do not start before the current position.
    pub fn schedule(&mut self, notes: &[PlayedNote]) {
        for n in notes {
            self.insert(Event {
                tick: n.start,
                kind: Kind::On(n.velocity),
                channel: n.channel,
                key: n.key,
            });
            self.insert(Event {
                tick: n.start + n.dur,
                kind: Kind::Off,
                channel: n.channel,
                key: n.key,
            });
        }
    }

    fn insert(&mut self, e: Event) {
        // Usually appended at the end: notes come in time order.
        let at = self.events.partition_point(|x| x.order() <= e.order());
        self.events.insert(at, e);
    }

    /// Renders the next `left.len()` samples into both buffers.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len(), "channel buffers differ in length");
        let n = left.len();
        let mut done = 0;
        while done < n {
            self.fire_due();
            let until = match self.events.front() {
                Some(e) => {
                    // The tolerance keeps rounding error from pushing an event a sample late.
                    let ahead = ((e.tick as f64 - self.tick) * self.samples_per_tick - 1e-6).ceil();
                    done + (ahead.max(1.0) as usize).min(n - done)
                }
                None => n,
            };
            self.engine
                .render(&mut left[done..until], &mut right[done..until]);
            self.tick += (until - done) as f64 / self.samples_per_tick;
            done = until;
        }
        self.fire_due();
    }

    fn fire_due(&mut self) {
        while let Some(e) = self.events.front() {
            if e.tick as f64 > self.tick + 1e-6 {
                break;
            }
            match e.kind {
                Kind::On(vel) => self.engine.note_on(e.channel, e.key, vel),
                Kind::Off => self.engine.note_off(e.channel, e.key),
            }
            self.events.pop_front();
        }
    }
}

/// Renders everything scheduled, plus `tail_secs` of release, into two channels.
pub fn render_all<E: Engine>(player: &mut Player<E>, tail_secs: f64) -> (Vec<f32>, Vec<f32>) {
    const BLOCK: usize = 1024;
    let end_tick = player.scheduled_until();
    let total = ((end_tick - player.position()) * player.samples_per_tick
        + tail_secs * f64::from(player.sample_rate))
    .ceil() as usize;
    let mut left = vec![0.0; total];
    let mut right = vec![0.0; total];
    for (l, r) in left.chunks_mut(BLOCK).zip(right.chunks_mut(BLOCK)) {
        player.render(l, r);
    }
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records when messages arrive, in samples.
    #[derive(Default)]
    struct Recorder {
        now: usize,
        log: Vec<(usize, &'static str, u8)>,
    }

    impl Engine for Recorder {
        fn note_on(&mut self, _: u8, key: u8, _: u8) {
            self.log.push((self.now, "on", key));
        }
        fn note_off(&mut self, _: u8, key: u8) {
            self.log.push((self.now, "off", key));
        }
        fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
            left.fill(0.0);
            right.fill(0.0);
            self.now += left.len();
        }
    }

    fn note(start: u64, dur: u64, key: u8) -> PlayedNote {
        PlayedNote {
            start,
            dur,
            key,
            velocity: 90,
            channel: 0,
        }
    }

    #[test]
    fn fires_events_on_the_right_sample() {
        // 1000 Hz and a quarter per second: one quarter (960 ticks) is 1000 samples.
        let mut p = Player::new(Recorder::default(), 1000, 1_000_000);
        p.schedule(&[note(0, 960, 60), note(960, 480, 62)]);
        let mut l = vec![0.0; 700];
        let mut r = vec![0.0; 700];
        for _ in 0..3 {
            p.render(&mut l, &mut r);
        }
        assert_eq!(
            p.engine_mut().log,
            vec![
                (0, "on", 60),
                (1000, "off", 60),
                (1000, "on", 62),
                (1500, "off", 62)
            ]
        );
    }

    #[test]
    fn tempo_changes_apply_from_now_on() {
        let mut p = Player::new(Recorder::default(), 1000, 1_000_000);
        p.schedule(&[note(0, 1920, 60)]);
        let mut l = vec![0.0; 500];
        let mut r = vec![0.0; 500];
        p.render(&mut l, &mut r); // half a quarter at 60 bpm
        p.set_tempo(500_000); // double speed: the remaining 1.5 quarters take 750 samples
        for _ in 0..3 {
            p.render(&mut l, &mut r);
        }
        assert_eq!(p.engine_mut().log[1], (1250, "off", 60));
    }
}
