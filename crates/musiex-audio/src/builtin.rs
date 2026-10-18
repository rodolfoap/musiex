//! A small built-in synthesizer: a soft piano-like tone and a woodblock-like
//! click. It needs no SoundFont, so it works everywhere, including the browser.

use crate::Engine;
use musiex_core::perform::CLICK_CHANNEL;
use std::f64::consts::TAU;

/// Relative strength of the first harmonics of the melody tone.
const HARMONICS: [f64; 6] = [1.0, 0.5, 0.25, 0.14, 0.08, 0.04];
const ATTACK: f64 = 0.004;
const RELEASE: f64 = 0.12;
const CLICK_DECAY: f64 = 0.012;
const SILENT: f64 = 1e-4;
const GAIN: f64 = 0.22;

#[derive(Debug, Clone)]
struct Voice {
    channel: u8,
    key: u8,
    freq: f64,
    amp: f64,
    click: bool,
    /// Samples since the note started.
    t: u64,
    /// Sample (in `t`) at which the key was released.
    released: Option<u64>,
}

impl Voice {
    fn sample(&self, rate: f64) -> f64 {
        let secs = self.t as f64 / rate;
        if self.click {
            return self.amp * (TAU * self.freq * secs).sin() * (-secs / CLICK_DECAY).exp();
        }
        let attack = (secs / ATTACK).min(1.0);
        let release = match self.released {
            Some(r) => (-((self.t - r) as f64 / rate) / (RELEASE / 5.0)).exp(),
            None => 1.0,
        };
        let mut s = 0.0;
        for (i, a) in HARMONICS.iter().enumerate() {
            let h = (i + 1) as f64;
            // Higher harmonics fade faster, as on a struck string.
            let decay = (-secs * (0.8 + 0.9 * h)).exp();
            s += a * decay * (TAU * self.freq * h * secs).sin();
        }
        self.amp * attack * release * s
    }

    fn finished(&self, rate: f64) -> bool {
        let secs = self.t as f64 / rate;
        if self.click {
            return secs > CLICK_DECAY * 10.0;
        }
        match self.released {
            Some(r) => (self.t - r) as f64 / rate > RELEASE,
            None => (-secs * 1.7).exp() < SILENT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinSynth {
    rate: f64,
    voices: Vec<Voice>,
}

impl BuiltinSynth {
    pub fn new(sample_rate: u32) -> Self {
        BuiltinSynth {
            rate: f64::from(sample_rate),
            voices: Vec::new(),
        }
    }
}

fn key_freq(key: u8) -> f64 {
    440.0 * 2f64.powf((f64::from(key) - 69.0) / 12.0)
}

impl Engine for BuiltinSynth {
    fn note_on(&mut self, channel: u8, key: u8, velocity: u8) {
        let click = channel == CLICK_CHANNEL;
        // Woodblock-like: accent (key 76) higher than other beats.
        let freq = if click {
            if key == 76 {
                1800.0
            } else {
                1300.0
            }
        } else {
            key_freq(key)
        };
        self.voices.push(Voice {
            channel,
            key,
            freq,
            amp: GAIN * f64::from(velocity) / 127.0,
            click,
            t: 0,
            released: None,
        });
    }

    fn note_off(&mut self, channel: u8, key: u8) {
        if let Some(v) = self
            .voices
            .iter_mut()
            .find(|v| v.channel == channel && v.key == key && v.released.is_none())
        {
            v.released = Some(v.t);
        }
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut s = 0.0;
            for v in &mut self.voices {
                s += v.sample(self.rate);
                v.t += 1;
            }
            *l = s as f32;
            *r = s as f32;
        }
        let rate = self.rate;
        self.voices.retain(|v| !v.finished(rate));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_440() {
        assert!((key_freq(69) - 440.0).abs() < 1e-9);
        assert!((key_freq(81) - 880.0).abs() < 1e-9);
    }

    #[test]
    fn sounds_then_goes_quiet() {
        let mut s = BuiltinSynth::new(8000);
        s.note_on(0, 60, 100);
        let mut l = vec![0.0; 800];
        let mut r = vec![0.0; 800];
        s.render(&mut l, &mut r);
        let peak = l.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.05 && peak < 1.0, "peak {peak}");
        s.note_off(0, 60);
        for _ in 0..3 {
            s.render(&mut l, &mut r);
        }
        assert!(s.voices.is_empty());
        assert!(l.iter().all(|x| *x == 0.0));
    }
}
