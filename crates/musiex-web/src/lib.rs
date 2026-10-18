//! The practice screen's core, compiled to WebAssembly.
//!
//! A [`Practice`] owns an endless generator, the engraver and the audio
//! player. The page asks it for one engraved system at a time, just ahead of
//! where the music is, and pulls audio from it block by block; every system
//! also reports where each note sits so the page can follow playback.

use musiex_audio::{BuiltinSynth, Player};
use musiex_core::duration::PPQ;
use musiex_core::perform::{micros_per_quarter, Performer};
use musiex_core::{Bar, Generator, Pitch, Settings};
use musiex_engrave::{engrave_system, pack, SystemOptions};
use serde::Deserialize;
use std::collections::VecDeque;
use wasm_bindgen::prelude::*;

/// Settings as the page sends them: the same names as the CLI options.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub meter: String,
    pub key: String,
    pub clef: String,
    pub tempo: u32,
    pub level: u8,
    pub low: Option<String>,
    pub high: Option<String>,
    pub leap: u8,
    pub rests: bool,
    pub triplets: bool,
    pub dotted: bool,
    pub ties: bool,
    pub sharps: bool,
    pub flats: bool,
    pub seed: u64,
    pub metronome: bool,
    pub count_in: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            meter: "4/4".into(),
            key: "C".into(),
            clef: "treble".into(),
            tempo: 60,
            level: 2,
            low: None,
            high: None,
            leap: 3,
            rests: false,
            triplets: false,
            dotted: false,
            ties: false,
            sharps: false,
            flats: false,
            seed: 0,
            metronome: true,
            count_in: 1,
        }
    }
}

impl Config {
    pub fn settings(&self) -> Result<Settings, String> {
        let e = |x: musiex_core::ParseError| x.to_string();
        let clef = self.clef.parse().map_err(e)?;
        let (mut low, mut high) = musiex_core::Clef::default_range(clef);
        if let Some(l) = self.low.as_deref().filter(|s| !s.is_empty()) {
            low = l.parse::<Pitch>().map_err(e)?;
        }
        if let Some(h) = self.high.as_deref().filter(|s| !s.is_empty()) {
            high = h.parse::<Pitch>().map_err(e)?;
        }
        let s = Settings {
            meter: self.meter.parse().map_err(e)?,
            key: self.key.parse().map_err(e)?,
            clef,
            tempo: self.tempo,
            low,
            high,
            level: self.level,
            max_leap: self.leap,
            rests: self.rests,
            triplets: self.triplets,
            dotted: self.dotted,
            ties: self.ties,
            sharps: self.sharps,
            flats: self.flats,
            seed: self.seed,
        };
        s.validate().map_err(e)?;
        Ok(s)
    }
}

/// Bars generated ahead of the engraver, so packing can look ahead.
const LOOKAHEAD_BARS: usize = 16;

pub struct Session {
    settings: Settings,
    generator: Generator,
    pending: VecDeque<Bar>,
    performer: Performer,
    player: Player<BuiltinSynth>,
    first_system: bool,
    tie_in: bool,
    /// Length of the count-in, in ticks.
    offset: u64,
}

impl Session {
    pub fn new(config: &Config, sample_rate: u32) -> Result<Self, String> {
        let settings = config.settings()?;
        let generator = Generator::new(settings.clone()).map_err(|e| e.to_string())?;
        let mut performer = Performer::new(&settings, config.metronome);
        let mut player = Player::new(
            BuiltinSynth::new(sample_rate),
            sample_rate,
            micros_per_quarter(&settings),
        );
        player.schedule(&performer.count_in(config.count_in));
        let offset = performer.position();
        Ok(Session {
            settings,
            generator,
            pending: VecDeque::new(),
            performer,
            player,
            first_system: true,
            tie_in: false,
            offset,
        })
    }

    /// Engraves the next system and schedules its audio. Returns JSON:
    /// `{svg, height, startTick, endTick, notes: [{start, dur, x, rest, letter,
    /// alter, key}]}`, where `letter` is 0 for C to 6 for B and `key` the MIDI key
    /// with ticks counted from the first bar and x in pixels.
    pub fn next_system(&mut self, width_px: f32, px_per_space: f32) -> String {
        while self.pending.len() < LOOKAHEAD_BARS {
            let bar = self.generator.next().expect("the generator never ends");
            self.pending.push_back(bar);
        }
        let width = width_px / px_per_space;
        let candidates: Vec<Bar> = self.pending.iter().cloned().collect();
        let n = pack(&self.settings, &candidates, width, self.first_system);
        let bars: Vec<Bar> = self.pending.drain(..n).collect();
        let opt = SystemOptions {
            width,
            time_signature: self.first_system,
            tie_in: self.tie_in,
        };
        let system = engrave_system(&self.settings, &bars, &opt);
        for b in &bars {
            let notes = self.performer.push_bar(b);
            self.player.schedule(&notes);
        }
        self.first_system = false;
        self.tie_in = bars
            .last()
            .and_then(|b| b.notes().last())
            .is_some_and(|n| n.tie);

        let bar_len = u64::from(self.settings.meter.bar_len());
        let first = bars.first().map_or(0, |b| b.index);
        let notes: Vec<_> = system
            .notes
            .iter()
            .map(|nb| {
                serde_json::json!({
                    "start": nb.start,
                    "dur": nb.dur,
                    "x": nb.x * px_per_space,
                    "rest": nb.rest,
                    "letter": nb.pitch.map(|p| p.letter()),
                    "alter": nb.pitch.map(|p| p.alter),
                    "key": nb.pitch.map(|p| p.midi()),
                })
            })
            .collect();
        serde_json::json!({
            "svg": system.scene.to_svg(px_per_space),
            "height": system.scene.height() * px_per_space,
            "startTick": first * bar_len,
            "endTick": (first + bars.len() as u64) * bar_len,
            "endX": (width - 0.3) * px_per_space,
            "notes": notes,
        })
        .to_string()
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.player.render(left, right);
    }

    /// Playback position in ticks from the first bar; negative during the count-in.
    pub fn position(&self) -> f64 {
        self.player.position() - self.offset as f64
    }

    pub fn set_tempo(&mut self, bpm: u32) {
        self.settings.tempo = bpm.clamp(20, 400);
        self.player.set_tempo(micros_per_quarter(&self.settings));
    }

    pub fn ticks_per_second(&self) -> f64 {
        1_000_000.0 / f64::from(micros_per_quarter(&self.settings)) * f64::from(PPQ)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }
}

/// The practice session, as JavaScript sees it.
#[wasm_bindgen]
pub struct Practice(Session);

#[wasm_bindgen]
impl Practice {
    /// `config` is a JSON object with the fields of [`Config`].
    #[wasm_bindgen(constructor)]
    pub fn new(config: &str, sample_rate: u32) -> Result<Practice, JsError> {
        let config: Config = serde_json::from_str(config)
            .map_err(|e| JsError::new(&format!("bad settings: {e}")))?;
        Session::new(&config, sample_rate)
            .map(Practice)
            .map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = nextSystem)]
    pub fn next_system(&mut self, width_px: f32, px_per_space: f32) -> String {
        self.0.next_system(width_px, px_per_space)
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.0.render(left, right);
    }

    pub fn position(&self) -> f64 {
        self.0.position()
    }

    #[wasm_bindgen(js_name = setTempo)]
    pub fn set_tempo(&mut self, bpm: u32) {
        self.0.set_tempo(bpm);
    }

    #[wasm_bindgen(js_name = ticksPerSecond)]
    pub fn ticks_per_second(&self) -> f64 {
        self.0.ticks_per_second()
    }

    #[wasm_bindgen(js_name = barTicks)]
    pub fn bar_ticks(&self) -> f64 {
        f64::from(self.0.settings().meter.bar_len())
    }

    /// The lowest and highest MIDI keys the exercise can use.
    #[wasm_bindgen(js_name = keyRange)]
    pub fn key_range(&self) -> Vec<i32> {
        let s = self.0.settings();
        vec![s.low.midi(), s.high.midi()]
    }

    #[wasm_bindgen(js_name = beatTicks)]
    pub fn beat_ticks(&self) -> f64 {
        f64::from(self.0.settings().meter.beat())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(json: &str) -> Session {
        let config: Config = serde_json::from_str(json).unwrap();
        Session::new(&config, 8000).unwrap()
    }

    #[test]
    fn systems_follow_each_other_without_gaps() {
        let mut s = session(r#"{"level": 3, "ties": true, "rests": true, "triplets": true}"#);
        let mut expected_start = 0;
        for _ in 0..20 {
            let v: serde_json::Value = serde_json::from_str(&s.next_system(900.0, 10.0)).unwrap();
            assert_eq!(v["startTick"], expected_start);
            let notes = v["notes"].as_array().unwrap();
            assert_eq!(notes[0]["start"], expected_start);
            expected_start = v["endTick"].as_u64().unwrap();
            assert!(v["svg"].as_str().unwrap().starts_with("<svg"));
        }
    }

    #[test]
    fn audio_starts_with_the_count_in() {
        let mut s = session(r#"{"tempo": 120, "countIn": 1}"#);
        s.next_system(900.0, 10.0);
        assert_eq!(s.position(), -3840.0);
        let mut l = vec![0.0; 8000];
        let mut r = vec![0.0; 8000];
        // One second at 120 bpm is two beats.
        s.render(&mut l, &mut r);
        assert!((s.position() - -1920.0).abs() < 1.0);
        assert!(l.iter().any(|x| x.abs() > 0.01));
    }

    #[test]
    fn notes_carry_names_and_keys() {
        let mut s = session(r#"{"rests": true, "sharps": true, "flats": true}"#);
        let (low, high) = (s.settings().low.midi(), s.settings().high.midi());
        let v: serde_json::Value = serde_json::from_str(&s.next_system(900.0, 10.0)).unwrap();
        for n in v["notes"].as_array().unwrap() {
            if n["rest"].as_bool().unwrap() {
                assert!(n["key"].is_null() && n["letter"].is_null());
            } else {
                let key = n["key"].as_i64().unwrap() as i32;
                assert!((low - 1..=high + 1).contains(&key));
                assert!(n["letter"].as_u64().unwrap() < 7);
            }
        }
    }

    #[test]
    fn rejects_bad_settings() {
        let bad: Config = serde_json::from_str(r#"{"key": "H"}"#).unwrap();
        assert!(Session::new(&bad, 8000).is_err());
        let bad: Config = serde_json::from_str(r#"{"low": "C6", "high": "C4"}"#).unwrap();
        assert!(Session::new(&bad, 8000).is_err());
    }

    #[test]
    fn tempo_changes_speed() {
        let mut s = session(r#"{"tempo": 60}"#);
        let slow = s.ticks_per_second();
        s.set_tempo(120);
        assert!((s.ticks_per_second() - 2.0 * slow).abs() < 1e-6);
    }
}
