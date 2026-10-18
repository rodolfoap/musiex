# musiex

Endless sight-reading exercises. A clean rewrite of
[ABCMusiEx](https://github.com/rodolfoap/abcmusiex) in Rust.

musiex generates reading material on demand: you choose the meter, key,
clef, range, smallest note value and which figures to include (rests,
triplets, dotted pairs, ties, accidentals), and it writes as many bars as you
want. The same seed always gives the same exercise.

## Status

Done: the generator core, and a command line tool that writes ABC notation,
a MIDI file with a metronome track, and a WAV file. Coming next:

1. A practice screen: the score scrolls and follows the playback, all
   controls on one screen, bars generated on the fly without limit.
2. Video export (`musiex video`) from the same renderer and synthesizer.

## Usage

```sh
cargo run --release -p musiex-cli -- generate --help

# 32 bars in G major, eighth notes, with rests and triplets
cargo run --release -p musiex-cli -- generate -k G -l 3 --rests --triplets -o lesson1

# 6/8 in the bass clef, reproducible
cargo run --release -p musiex-cli -- generate -m 6/8 -c bass -k Dm -l 3 --ties -s 1234
```

That writes `lesson1.abc` and `lesson1.mid`. Add `--wav` for `lesson1.wav`,
rendered with the built-in synth or, with `--soundfont FILE.sf2`, with any
General MIDI SoundFont. Any ABC tool can print the
score, for example [abcjs](https://www.abcjs.net/abcjs-editor.html) in the
browser or `abcm2ps`.

Options carried over from ABCMusiEx: tempo `-t`, key `-k`, clef `-c`,
bars `-n`, level `-l` (1 halves, 2 quarters, 3 eighths, 4 sixteenths), leap
`-d` (the old "dispersion", in staff steps), `--rests`, `--triplets`,
`--dotted`, `--ties` (alias `--legato`), `--sharps`, `--flats`, `--seed`.
The range is now given as note names, `-r C4..A5`, and defaults to a
comfortable range for the clef.

## Layout

- `crates/musiex-core`: the music model and generator, with no I/O.
  - `generator`: an endless iterator of bars.
  - `rhythm`: recursive bar subdivision, aware of simple and compound meters.
  - `melody`: a bounded random walk over staff positions, key-aware
    accidentals, ties into beats and across barlines.
  - `abc`: ABC notation output with beaming and bar-local accidentals.
  - `perform`: timed notes with metronome and count-in, bar by bar.
- `crates/musiex-audio`: a tick-based sequencer (`Player`) that renders block
  by block and accepts tempo changes while playing, a dependency-free
  built-in synth, and SoundFont playback through rustysynth.
- `crates/musiex-cli`: the `musiex` command.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

`tests/golden` pins the output of a few seeds. If a change to the generator is
intended, refresh them with `UPDATE_GOLDEN=1 cargo test -p musiex-core --test golden`.

CI also generates 200 exercises across meters, keys and levels and checks them
with abcjs (`tools/check-abc.js`).

## License

GPL-3.0-or-later, like the original.
