//! MP4 output: the practice screen, rendered frame by frame.
//!
//! The score is engraved with the same engraver as the web page and
//! rasterized with resvg; the audio is the same synthesizer's WAV. Frames go
//! to ffmpeg as raw RGBA on its standard input, and ffmpeg muxes them with the
//! WAV into an H.264/AAC MP4.

use crate::wav;
use anyhow::{bail, Context, Result};
use musiex_core::duration::PPQ;
use musiex_core::perform::micros_per_quarter;
use musiex_core::{Bar, Settings};
use musiex_engrave::{engrave_system, pack, NoteBox, SystemOptions};
use resvg::tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BACKGROUND: [u8; 3] = [0xfb, 0xfa, 0xf7];
const INK: &str = "#1d1b18";
const ACCENT: &str = "#c2410c";
const ACCENT_RGB: [u8; 3] = [0xc2, 0x41, 0x0c];
/// Seconds a page turn takes, from one system to the next.
const SCROLL_SECS: f64 = 0.35;
/// Seconds of picture after the last note, matching the WAV's tail.
const TAIL_SECS: f64 = 1.0;

#[derive(Debug, Clone)]
pub struct Options {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// Pixels per staff space.
    pub zoom: f32,
    pub metronome: bool,
    pub count_in: u32,
    pub soundfont: Option<PathBuf>,
    pub ffmpeg: PathBuf,
}

/// One engraved system and where it sits on the (endless) page.
struct Line {
    svg: String,
    notes: Vec<NoteBox>,
    start: u64,
    end: u64,
    /// Top of the system on the page, and its height, in pixels.
    y: f32,
    height: f32,
    /// Where the staff's top line is, from the top of the system, in pixels.
    staff_top: f32,
    /// Where the cursor stops after the last note, in pixels from the left edge.
    end_x: f32,
    plain: Pixmap,
}

/// Everything needed to draw any frame.
pub struct Movie {
    opt: Options,
    margin: f32,
    lines: Vec<Line>,
    /// Ticks per second, and the count-in length in ticks.
    tps: f64,
    offset: f64,
    total_ticks: f64,
    highlighted: Option<(usize, usize, Pixmap)>,
}

/// Rasterizes an engraved system, drawing note `highlight` in the accent colour.
fn rasterize(svg: &str, highlight: Option<usize>) -> Result<Pixmap> {
    let mut svg = svg.replace("currentColor", INK);
    if let Some(i) = highlight {
        let group = format!(r#"<g class="note" data-note="{i}">"#);
        let colored =
            format!(r#"<g class="note" data-note="{i}" fill="{ACCENT}" stroke="{ACCENT}">"#);
        svg = svg.replacen(&group, &colored, 1);
    }
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
        .context("rasterizing the score")?;
    let size = tree.size().to_int_size();
    let mut pixmap = Pixmap::new(size.width(), size.height()).context("empty system")?;
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    Ok(pixmap)
}

fn ease(p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    p * p * (3.0 - 2.0 * p)
}

impl Movie {
    pub fn new(s: &Settings, bars: &[Bar], opt: Options) -> Result<Self> {
        if opt.width < 160 || opt.height < 120 {
            bail!("the video must be at least 160x120");
        }
        let margin = (opt.width as f32 * 0.04).round();
        let width = (opt.width as f32 - 2.0 * margin) / opt.zoom;
        let bar_len = u64::from(s.meter.bar_len());

        let mut lines = Vec::new();
        let mut rest = bars;
        let mut y = margin;
        let mut tie_in = false;
        while !rest.is_empty() {
            let first = lines.is_empty();
            let n = pack(s, rest, width, first).clamp(1, rest.len());
            let (these, later) = rest.split_at(n);
            let system = engrave_system(
                s,
                these,
                &SystemOptions {
                    width,
                    time_signature: first,
                    tie_in,
                },
            );
            tie_in = these
                .last()
                .and_then(|b| b.notes().last())
                .is_some_and(|n| n.tie);
            let svg = system.scene.to_svg(opt.zoom);
            let plain = rasterize(&svg, None)?;
            let height = system.scene.height() * opt.zoom;
            lines.push(Line {
                notes: system.notes,
                start: these[0].index * bar_len,
                end: (these[0].index + n as u64) * bar_len,
                y,
                height,
                staff_top: -system.scene.top * opt.zoom,
                end_x: (width - 0.3) * opt.zoom,
                svg,
                plain,
            });
            y += height;
            rest = later;
        }

        let tps = 1_000_000.0 / f64::from(micros_per_quarter(s)) * f64::from(PPQ);
        Ok(Movie {
            margin,
            tps,
            offset: f64::from(opt.count_in) * bar_len as f64,
            total_ticks: (bars.len() as u64 * bar_len) as f64,
            lines,
            highlighted: None,
            opt,
        })
    }

    pub fn duration(&self) -> f64 {
        (self.offset + self.total_ticks) / self.tps + TAIL_SECS
    }

    pub fn frames(&self) -> u64 {
        (self.duration() * f64::from(self.opt.fps)).ceil() as u64
    }

    /// The system playing at `tick` (the first one during the count-in).
    fn line_at(&self, tick: f64) -> usize {
        self.lines
            .iter()
            .position(|l| tick < l.end as f64)
            .unwrap_or(self.lines.len() - 1)
    }

    /// Draws frame `index` into `frame`, which is `width` x `height`.
    pub fn draw(&mut self, index: u64, frame: &mut Pixmap) -> Result<()> {
        let t = index as f64 / f64::from(self.opt.fps);
        let tick = t * self.tps - self.offset;
        let current = self.line_at(tick);

        // Scroll so the current system is at the top, easing in each page turn.
        let target = |i: usize| self.lines[i].y - self.margin;
        let mut scroll = target(current);
        if current > 0 {
            let since = (tick - self.lines[current].start as f64) / self.tps;
            let p = ease(since / SCROLL_SECS);
            scroll = target(current - 1) + (scroll - target(current - 1)) * p as f32;
        }

        // The note under the cursor, and how far the cursor is towards the next.
        let line = &self.lines[current];
        let playing = tick >= 0.0 && tick < self.total_ticks;
        let mut i = 0;
        while i + 1 < line.notes.len() && line.notes[i + 1].start as f64 <= tick {
            i += 1;
        }
        let note = &line.notes[i];
        let frac = if tick < 0.0 {
            0.0
        } else {
            ((tick - note.start as f64) / note.dur as f64).clamp(0.0, 1.0) as f32
        };
        let x0 = note.x * self.opt.zoom;
        let x1 = line
            .notes
            .get(i + 1)
            .map_or(line.end_x, |n| n.x * self.opt.zoom);
        let cursor_x = x0 + (x1 - x0) * frac;
        let highlight = (playing && !note.rest).then_some((current, i));

        if self.highlighted.as_ref().map(|(l, n, _)| (*l, *n)) != highlight {
            self.highlighted = match highlight {
                Some((l, n)) => Some((l, n, rasterize(&self.lines[l].svg, Some(n))?)),
                None => None,
            };
        }

        let [r, g, b] = BACKGROUND;
        frame.fill(Color::from_rgba8(r, g, b, 255));
        let paint = PixmapPaint::default();
        for (li, l) in self.lines.iter().enumerate() {
            let top = l.y - scroll;
            if top > self.opt.height as f32 || top + l.height < 0.0 {
                continue;
            }
            if li == current && tick < self.total_ticks {
                self.cursor(frame, self.margin + cursor_x, top + l.staff_top);
            }
            let pixmap = match &self.highlighted {
                Some((hl, _, p)) if *hl == li => p,
                _ => &l.plain,
            };
            frame.draw_pixmap(
                self.margin as i32,
                top.round() as i32,
                pixmap.as_ref(),
                &paint,
                Transform::identity(),
                None,
            );
        }
        self.progress(frame, (tick / self.total_ticks).clamp(0.0, 1.0) as f32);
        Ok(())
    }

    /// The soft box that follows the music, as on the web page.
    fn cursor(&self, frame: &mut Pixmap, x: f32, staff_top: f32) {
        let z = self.opt.zoom;
        let mut pb = PathBuilder::new();
        let Some(rect) = Rect::from_xywh(x - 0.4 * z, staff_top - z, 2.0 * z, 6.0 * z) else {
            return;
        };
        pb.push_rect(rect);
        let Some(path) = pb.finish() else { return };
        let mut paint = Paint::default();
        let [r, g, b] = ACCENT_RGB;
        paint.set_color_rgba8(r, g, b, 36);
        frame.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    /// A thin bar along the bottom showing how much of the exercise has played.
    fn progress(&self, frame: &mut Pixmap, done: f32) {
        let h = (self.opt.height as f32 / 180.0).max(2.0);
        let w = self.opt.width as f32 * done;
        let Some(rect) = Rect::from_xywh(0.0, self.opt.height as f32 - h, w, h) else {
            return;
        };
        let mut paint = Paint::default();
        let [r, g, b] = ACCENT_RGB;
        paint.set_color_rgba8(r, g, b, 255);
        frame.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

pub fn write(path: &Path, s: &Settings, bars: &[Bar], opt: &Options) -> Result<()> {
    let mut movie = Movie::new(s, bars, opt.clone())?;

    let audio = std::env::temp_dir().join(format!("musiex-{}.wav", std::process::id()));
    wav::write(
        &audio,
        s,
        bars,
        opt.metronome,
        opt.count_in,
        opt.soundfont.as_deref(),
    )?;
    let result = encode(path, &mut movie, &audio);
    let _ = std::fs::remove_file(&audio);
    result
}

fn encode(path: &Path, movie: &mut Movie, audio: &Path) -> Result<()> {
    let opt = movie.opt.clone();
    let mut child = Command::new(&opt.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(["-f", "rawvideo", "-pix_fmt", "rgba"])
        .args(["-s", &format!("{}x{}", opt.width, opt.height)])
        .args(["-r", &opt.fps.to_string(), "-i", "-"])
        .arg("-i")
        .arg(audio)
        .args(["-c:v", "libx264", "-preset", "medium", "-crf", "20"])
        .args(["-pix_fmt", "yuv420p", "-movflags", "+faststart"])
        .args(["-c:a", "aac", "-b:a", "192k", "-shortest"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "could not run {}; install ffmpeg or point --ffmpeg at it",
                opt.ffmpeg.display()
            )
        })?;

    let mut stdin = child.stdin.take().expect("stdin is piped");
    let mut frame = Pixmap::new(opt.width, opt.height).context("bad video size")?;
    let frames = movie.frames();
    for i in 0..frames {
        movie.draw(i, &mut frame)?;
        if stdin.write_all(frame.data()).is_err() {
            // ffmpeg stopped reading; its exit status says why.
            break;
        }
    }
    drop(stdin);
    let status = child.wait()?;
    if !status.success() {
        bail!("ffmpeg failed ({status})");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiex_core::Generator;

    fn movie(bars: usize) -> Movie {
        let s = Settings {
            tempo: 120,
            ..Settings::default()
        };
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(bars).collect();
        let opt = Options {
            width: 640,
            height: 360,
            fps: 10,
            zoom: 8.0,
            metronome: true,
            count_in: 1,
            soundfont: None,
            ffmpeg: "ffmpeg".into(),
        };
        Movie::new(&s, &bars, opt).unwrap()
    }

    fn accent_pixels(p: &Pixmap) -> usize {
        p.data()
            .chunks(4)
            .filter(|c| c[0] > 0xa0 && c[1] < 0x70 && c[2] < 0x40)
            .count()
    }

    #[test]
    fn duration_covers_count_in_music_and_tail() {
        let m = movie(8);
        // 1 + 8 bars of 4/4 at 120 bpm is 18 s, plus the tail.
        assert!((m.duration() - 19.0).abs() < 1e-9);
        assert_eq!(m.frames(), 190);
        assert!(m.lines.len() > 1);
    }

    #[test]
    fn highlights_only_while_playing() {
        let mut m = movie(8);
        let mut frame = Pixmap::new(640, 360).unwrap();
        // During the count-in nothing is highlighted, only the progress bar is empty.
        m.draw(5, &mut frame).unwrap();
        assert!(m.highlighted.is_none());
        let quiet = accent_pixels(&frame);
        // Half a second into the music the first note is lit.
        m.draw(25, &mut frame).unwrap();
        assert!(matches!(m.highlighted, Some((0, 0, _))));
        assert!(accent_pixels(&frame) > quiet + 20);
        let lit = &m.highlighted.as_ref().unwrap().2;
        assert_eq!(accent_pixels(&m.lines[0].plain), 0);
        assert!(accent_pixels(lit) > 20);
        // After the end the score is plain again.
        m.draw(m.frames() - 1, &mut frame).unwrap();
        assert!(m.highlighted.is_none());
    }

    #[test]
    fn turns_to_the_next_system() {
        let mut m = movie(16);
        let second = m.lines[1].start as f64;
        let frame_at = |tick: f64| ((tick + m.offset) / m.tps * 10.0).ceil() as u64;
        let mut frame = Pixmap::new(640, 360).unwrap();
        m.draw(frame_at(second) + 5, &mut frame).unwrap();
        assert!(matches!(m.highlighted, Some((1, _, _))));
    }
}
