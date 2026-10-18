//! A seed must keep producing the same exercise: shared links and saved
//! settings depend on it. If a change to the generator is intended, refresh
//! the files with `UPDATE_GOLDEN=1 cargo test -p musiex-core --test golden`.

use musiex_core::{abc, Generator, Settings};
use std::path::PathBuf;

fn check(name: &str, settings: Settings) {
    let bars: Vec<_> = Generator::new(settings.clone()).unwrap().take(16).collect();
    let header = abc::Header {
        title: name.to_string(),
        ..abc::Header::default()
    };
    let text = abc::tune(&settings, &header, &bars, 4);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/{name}.abc"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        text, expected,
        "{name} changed; see the note at the top of this file"
    );
}

#[test]
fn quarters_in_c() {
    check(
        "quarters_in_c",
        Settings {
            seed: 42,
            ..Settings::default()
        },
    );
}

#[test]
fn everything_in_six_eight() {
    check(
        "everything_in_six_eight",
        Settings {
            meter: "6/8".parse().unwrap(),
            key: "Eb".parse().unwrap(),
            level: 4,
            rests: true,
            triplets: true,
            dotted: true,
            ties: true,
            sharps: true,
            flats: true,
            seed: 42,
            ..Settings::default()
        },
    );
}

#[test]
fn eighths_in_bass_clef() {
    let clef = "bass".parse().unwrap();
    let (low, high) = musiex_core::Clef::default_range(clef);
    check(
        "eighths_in_bass_clef",
        Settings {
            meter: "3/4".parse().unwrap(),
            key: "Dm".parse().unwrap(),
            clef,
            low,
            high,
            level: 3,
            rests: true,
            triplets: true,
            ties: true,
            seed: 42,
            ..Settings::default()
        },
    );
}
