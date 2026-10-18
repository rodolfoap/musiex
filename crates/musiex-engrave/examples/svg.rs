//! Prints an HTML page of engraved systems, for eyeballing the layout.
//! Usage: cargo run -p musiex-engrave --example svg -- [seed] > page.html

use musiex_core::{Generator, Settings};
use musiex_engrave::{engrave_system, pack, SystemOptions};

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let presets = [
        Settings {
            seed,
            ..Settings::default()
        },
        Settings {
            seed,
            level: 4,
            key: "D".parse().unwrap(),
            rests: true,
            triplets: true,
            dotted: true,
            ties: true,
            sharps: true,
            flats: true,
            ..Settings::default()
        },
        Settings {
            seed,
            meter: "6/8".parse().unwrap(),
            key: "Eb".parse().unwrap(),
            clef: "bass".parse().unwrap(),
            low: "E2".parse().unwrap(),
            high: "C4".parse().unwrap(),
            level: 3,
            ties: true,
            rests: true,
            dotted: true,
            ..Settings::default()
        },
        Settings {
            seed,
            meter: "3/4".parse().unwrap(),
            key: "A".parse().unwrap(),
            level: 3,
            triplets: true,
            ties: true,
            low: "A3".parse().unwrap(),
            high: "C6".parse().unwrap(),
            max_leap: 5,
            ..Settings::default()
        },
    ];
    println!("<!doctype html><html><body style='background:#fff;color:#111;margin:20px'>");
    for s in presets {
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(12).collect();
        let mut rest = &bars[..];
        let mut first = true;
        let mut tie_in = false;
        while !rest.is_empty() {
            let n = pack(&s, rest, 80.0, first);
            let opt = SystemOptions {
                time_signature: first,
                tie_in,
                ..SystemOptions::default()
            };
            println!(
                "<div>{}</div>",
                engrave_system(&s, &rest[..n], &opt).scene.to_svg(14.0)
            );
            tie_in = rest[n - 1].notes().last().is_some_and(|n| n.tie);
            rest = &rest[n..];
            first = false;
        }
        println!("<hr>");
    }
    println!("</body></html>");
}
