# musiex

Endless sight-reading exercises. A clean rewrite of
[ABCMusiEx](https://github.com/rodolfoap/abcmusiex) in Rust.

musiex generates reading material on demand: you choose the meter, key,
clef, range, smallest note value and which figures to include (rests,
triplets, dotted pairs, ties, accidentals), and it writes as many bars as you
want. The same seed always gives the same exercise.

## Status

Done: the generator core; a command line tool that writes ABC notation, a
MIDI file with a metronome track, a WAV file and an MP4 video; and the
practice screen.

## Practice screen

A web page that plays the exercise and follows along: the current note lights
up, a cursor moves with the music, finished lines scroll away and new bars are
generated as you go, without limit. Tempo, settings, a new exercise and a
shareable link (settings and seed in the URL) are all on the same screen.
Everything runs in the browser; there is no server.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
./web/build.sh
python3 -m http.server -d web 8000   # then open http://localhost:8000
```

Space plays and pauses, N starts a new exercise.

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

### Video

```sh
# 16 bars at 90 bpm as a 1280x720 MP4, with a one-bar count-in
cargo run --release -p musiex-cli -- video -n 16 -t 90 -l 3 --rests -o lesson1.mp4
```

The video shows the same score as the practice screen: the current note
lights up, a cursor follows the music, finished lines scroll away, and a thin
bar along the bottom shows progress. The audio comes from the same synth
(`--soundfont` works here too). `--size`, `--fps` and `--zoom` set the frame
size, frame rate and score size. It needs `ffmpeg` on the `PATH`, or
`--ffmpeg PATH`.

### Options

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
- `crates/musiex-engrave`: draws systems as SVG (noteheads, stems, beams,
  ties, tuplets, rests, accidentals, clefs, key and time signatures) and
  reports where each note sits. Glyphs are Bravura's, embedded as paths.
- `crates/musiex-web`: the practice screen's WebAssembly core; `web/` is the
  page around it.
- `crates/musiex-cli`: the `musiex` command; `video` rasterizes the engraved
  systems with resvg and pipes frames to ffmpeg.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

`tests/golden` pins the output of a few seeds. If a change to the generator is
intended, refresh them with `UPDATE_GOLDEN=1 cargo test -p musiex-core --test golden`.

CI also generates 200 exercises across meters, keys and levels and checks them
with abcjs (`tools/check-abc.js`), plays the practice screen in headless
Chromium (`tools/web-smoke.js`), and renders a short video with ffmpeg.

`cargo run -p musiex-engrave --example svg > page.html` prints a page of
engraved systems in several meters, keys and clefs, for checking the layout
by eye.

## License

GPL-3.0-or-later, like the original. The Bravura music font is © Steinberg
Media Technologies GmbH under the SIL Open Font License 1.1
(`crates/musiex-engrave/BRAVURA-OFL.txt`).
