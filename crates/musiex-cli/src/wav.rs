//! WAV output through the musiex-audio synthesizers.

use anyhow::{Context, Result};
use musiex_audio::{render_all, BuiltinSynth, Engine, Player, SoundFontSynth};
use musiex_core::perform::{micros_per_quarter, perform};
use musiex_core::{Bar, Settings};
use std::path::Path;

const SAMPLE_RATE: u32 = 44_100;

pub fn write(
    path: &Path,
    s: &Settings,
    bars: &[Bar],
    metronome: bool,
    count_in: u32,
    soundfont: Option<&Path>,
) -> Result<()> {
    let engine: Box<dyn Engine> = match soundfont {
        Some(sf) => {
            let mut file = std::io::BufReader::new(
                std::fs::File::open(sf).with_context(|| format!("opening {}", sf.display()))?,
            );
            Box::new(SoundFontSynth::new(&mut file, SAMPLE_RATE).map_err(anyhow::Error::msg)?)
        }
        None => Box::new(BuiltinSynth::new(SAMPLE_RATE)),
    };
    let mut player = Player::new(engine, SAMPLE_RATE, micros_per_quarter(s));
    player.schedule(&perform(s, bars, metronome, count_in));
    let (left, right) = render_all(&mut player, 1.0);

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec)
        .with_context(|| format!("writing {}", path.display()))?;
    for (l, r) in left.iter().zip(&right) {
        for x in [l, r] {
            w.write_sample((x.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
        }
    }
    w.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiex_core::Generator;

    #[test]
    fn writes_an_audible_file_of_the_right_length() {
        let s = Settings {
            tempo: 120,
            ..Settings::default()
        };
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(2).collect();
        let path = std::env::temp_dir().join(format!("musiex-test-{}.wav", std::process::id()));
        write(&path, &s, &bars, true, 1, None).unwrap();
        let mut reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        let frames = reader.duration();
        let peak = reader
            .samples::<i16>()
            .map(|x| x.unwrap().unsigned_abs())
            .max()
            .unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(spec.channels, 2);
        // Three bars of 4/4 at 120 bpm is 6 s, plus 1 s of tail.
        assert_eq!(frames, 7 * SAMPLE_RATE);
        assert!(peak > 1000, "peak {peak}");
    }
}
