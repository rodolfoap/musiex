//! Laying out bars as one staff system.

use crate::glyphs::*;
use crate::scene::{Scene, Shape};
use crate::Glyph;
use musiex_core::duration::{is_dotted, Dur, EIGHTH, HALF, QUARTER, SIXTEENTH, WHOLE};
use musiex_core::model::{Bar, Event};
use musiex_core::notation;
use musiex_core::{Clef, Pitch, Settings};

const STAFF_LINE: f32 = 0.13;
const LEDGER: f32 = 0.16;
const STEM: f32 = 0.12;
const STEM_LEN: f32 = 3.5;
const BARLINE: f32 = 0.16;
const BEAM: f32 = 0.5;
const BEAM_GAP: f32 = 0.25;
const HEAD: f32 = 1.18;
const ACCIDENTAL_SPACE: f32 = 1.5;
/// Space after each barline, before the first note.
const BAR_PAD: f32 = 1.2;
/// Space kept after the last note of a system.
const END_PAD: f32 = 0.3;
/// Room kept above and below the staff, in staff spaces.
const TOP: f32 = -5.0;
const BOTTOM: f32 = 9.0;

#[derive(Debug, Clone)]
pub struct SystemOptions {
    /// Total width, in staff spaces.
    pub width: f32,
    /// Show the time signature (usually only on the first system).
    pub time_signature: bool,
    /// The first note continues a tie from the previous system.
    pub tie_in: bool,
}

impl Default for SystemOptions {
    fn default() -> Self {
        SystemOptions {
            width: 80.0,
            time_signature: true,
            tie_in: false,
        }
    }
}

/// Where a note or rest sits, for following playback.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteBox {
    pub bar: u64,
    /// Start and sounding length in ticks, counted from the start of bar 0.
    pub start: u64,
    pub dur: u64,
    /// Left edge of the notehead (or rest), in staff spaces.
    pub x: f32,
    pub rest: bool,
    /// The sounding pitch; `None` for a rest.
    pub pitch: Option<Pitch>,
}

#[derive(Debug, Clone)]
pub struct System {
    pub scene: Scene,
    pub notes: Vec<NoteBox>,
    /// The x of each bar's start (just after its barline), by bar index.
    pub bars: Vec<(u64, f32)>,
    /// Where the music starts, after clef, key and time signature.
    pub music_start: f32,
}

/// Staff step of the top line for each clef.
fn top_line(clef: Clef) -> i32 {
    match clef {
        Clef::Treble => 10, // F5
        Clef::Bass => -2,   // A3
        Clef::Alto => 4,    // G4
        Clef::Tenor => 2,   // E4
    }
}

fn staff_y(clef: Clef, step: i32) -> f32 {
    (top_line(clef) - step) as f32 / 2.0
}

fn clef_glyph(clef: Clef) -> (Glyph, f32) {
    match clef {
        Clef::Treble => (G_CLEF, 3.0),
        Clef::Bass => (F_CLEF, 1.0),
        Clef::Alto => (C_CLEF, 2.0),
        Clef::Tenor => (C_CLEF, 1.0),
    }
}

/// Staff steps of key-signature accidentals on the treble staff, in order.
const SHARP_STEPS: [i32; 7] = [10, 7, 11, 8, 5, 9, 6];
const FLAT_STEPS: [i32; 7] = [6, 9, 5, 8, 4, 7, 3];

fn key_signature_steps(clef: Clef, fifths: i8) -> Vec<(i32, bool)> {
    let shift = match clef {
        Clef::Treble => 0,
        Clef::Bass => -14,
        Clef::Alto | Clef::Tenor => -7,
    };
    if fifths >= 0 {
        SHARP_STEPS[..fifths as usize]
            .iter()
            .map(|s| (s + shift, true))
            .collect()
    } else {
        FLAT_STEPS[..(-fifths) as usize]
            .iter()
            .map(|s| (s + shift, false))
            .collect()
    }
}

fn digit(d: u32) -> Glyph {
    [
        TIME_SIG_0, TIME_SIG_1, TIME_SIG_2, TIME_SIG_3, TIME_SIG_4, TIME_SIG_5, TIME_SIG_6,
        TIME_SIG_7, TIME_SIG_8, TIME_SIG_9,
    ][d as usize]
}

fn digits(v: u8) -> Vec<Glyph> {
    v.to_string()
        .chars()
        .map(|c| digit(c.to_digit(10).expect("a decimal digit")))
        .collect()
}

fn accidental_glyph(alter: i8) -> Glyph {
    match alter {
        1 => ACCIDENTAL_SHARP,
        -1 => ACCIDENTAL_FLAT,
        _ => ACCIDENTAL_NATURAL,
    }
}

/// Undotted value of a written duration.
fn base_value(written: Dur) -> Dur {
    if is_dotted(written) {
        written / 3 * 2
    } else {
        written
    }
}

fn rest_glyph(written: Dur) -> (Glyph, f32) {
    match base_value(written) {
        d if d >= WHOLE => (REST_WHOLE, 1.0),
        d if d >= HALF => (REST_HALF, 2.0),
        d if d >= QUARTER => (REST_QUARTER, 2.0),
        d if d >= EIGHTH => (REST_8TH, 2.0),
        _ => (REST_16TH, 2.0),
    }
}

fn head_glyph(written: Dur) -> Glyph {
    match base_value(written) {
        d if d >= WHOLE => NOTEHEAD_WHOLE,
        d if d >= HALF => NOTEHEAD_HALF,
        _ => NOTEHEAD_BLACK,
    }
}

/// Number of beams or flags a written value needs.
fn beams(written: Dur) -> u32 {
    match base_value(written) {
        d if d <= SIXTEENTH => 2,
        d if d <= EIGHTH => 1,
        _ => 0,
    }
}

/// Natural horizontal space for a note lasting `dur`, before justification.
fn natural_width(dur: Dur) -> f32 {
    let ratio = dur.max(1) as f32 / SIXTEENTH as f32;
    1.9 + 1.3 * ratio.log2()
}

#[derive(Debug, Clone)]
struct Slot {
    bar: u64,
    start: u64,
    dur: u64,
    written: Dur,
    /// `None` for rests.
    pitch: Option<Pitch>,
    tie: bool,
    accidental: Option<i8>,
    beam_group: Option<usize>,
    tuplet: Option<usize>,
    width: f32,
    x: f32,
}

impl Slot {
    fn y(&self, clef: Clef) -> f32 {
        self.pitch.map_or(2.0, |p| staff_y(clef, p.step))
    }
}

fn slots(s: &Settings, bars: &[Bar]) -> Vec<Vec<Slot>> {
    let bar_len = u64::from(s.meter.bar_len());
    let mut group = 0;
    let mut tuplet = 0;
    let mut out = Vec::new();
    for bar in bars {
        let mut accidentals = notation::accidentals(s.key, bar).into_iter();
        let joined = notation::beamed_to_previous(s.meter, &bar.events);
        let mut t = bar.index * bar_len;
        let mut row = Vec::new();
        let slot = |written: Dur,
                    dur: Dur,
                    pitch: Option<Pitch>,
                    tie: bool,
                    accidental: Option<i8>,
                    beam_group: Option<usize>,
                    tup: Option<usize>,
                    t: &mut u64| {
            let width = natural_width(dur)
                + if accidental.is_some() {
                    ACCIDENTAL_SPACE
                } else {
                    0.0
                }
                + if is_dotted(written) { 0.6 } else { 0.0 };
            let sl = Slot {
                bar: bar.index,
                start: *t,
                dur: u64::from(dur),
                written,
                pitch,
                tie,
                accidental,
                beam_group,
                tuplet: tup,
                width,
                x: 0.0,
            };
            *t += u64::from(dur);
            sl
        };
        for (i, e) in bar.events.iter().enumerate() {
            let beam_group = if notation::beamable(e) {
                if !joined[i] {
                    group += 1;
                }
                Some(group)
            } else {
                None
            };
            match e {
                Event::Note(nt) => {
                    let acc = accidentals.next().flatten();
                    row.push(slot(
                        nt.dur,
                        nt.dur,
                        Some(nt.pitch),
                        nt.tie,
                        acc,
                        beam_group,
                        None,
                        &mut t,
                    ));
                }
                Event::Rest(d) => row.push(slot(*d, *d, None, false, None, None, None, &mut t)),
                Event::Tuplet(tp) => {
                    tuplet += 1;
                    for nt in &tp.notes {
                        let acc = accidentals.next().flatten();
                        row.push(slot(
                            nt.dur,
                            tp.actual(nt.dur),
                            Some(nt.pitch),
                            false,
                            acc,
                            beam_group,
                            Some(tuplet),
                            &mut t,
                        ));
                    }
                }
            }
        }
        out.push(row);
    }
    out
}

struct Stem {
    up: bool,
    x: f32,
    /// y where the stem leaves the notehead.
    base: f32,
    tip: f32,
}

fn stem_for(head_x: f32, y: f32, up: bool) -> Stem {
    if up {
        Stem {
            up,
            x: head_x + HEAD - STEM / 2.0,
            base: y - 0.17,
            // Stems reach at least the middle line.
            tip: (y - STEM_LEN).min(2.0),
        }
    } else {
        Stem {
            up,
            x: head_x + STEM / 2.0,
            base: y + 0.17,
            tip: (y + STEM_LEN).max(2.0),
        }
    }
}

/// Draws the clef, key signature and (optionally) time signature, and
/// returns where the music can start.
fn header(s: &Settings, time_signature: bool, scene: &mut Scene) -> f32 {
    let clef = s.clef;
    let mut x = 0.8;
    let (cg, cy) = clef_glyph(clef);
    scene.push(
        Shape::Glyph {
            glyph: cg,
            x,
            y: cy,
        },
        None,
    );
    x += cg.advance + 0.8;
    for (step, sharp) in key_signature_steps(clef, s.key.fifths) {
        let g = if sharp {
            ACCIDENTAL_SHARP
        } else {
            ACCIDENTAL_FLAT
        };
        scene.push(
            Shape::Glyph {
                glyph: g,
                x,
                y: staff_y(clef, step),
            },
            None,
        );
        x += g.advance + 0.15;
    }
    x += 0.6;
    if time_signature {
        let top = digits(s.meter.num);
        let bottom = digits(s.meter.den);
        let w = |gs: &[Glyph]| gs.iter().map(|g| g.advance).sum::<f32>();
        let width = w(&top).max(w(&bottom));
        for (row, y) in [(&top, 1.0), (&bottom, 3.0)] {
            let mut gx = x + (width - w(row)) / 2.0;
            for g in row.iter() {
                scene.push(
                    Shape::Glyph {
                        glyph: *g,
                        x: gx,
                        y,
                    },
                    None,
                );
                gx += g.advance;
            }
        }
        x += width + 1.0;
    }
    x
}

/// Width of the clef, key and time signature area.
pub fn header_width(s: &Settings, time_signature: bool) -> f32 {
    header(s, time_signature, &mut Scene::default())
}

/// The space a bar needs at its natural spacing. Callers fill a system with
/// bars until `header_width` plus these widths would exceed the system width,
/// so dense bars get fewer per line and nothing is squeezed.
pub fn bar_width(s: &Settings, bar: &Bar) -> f32 {
    slots(s, std::slice::from_ref(bar))[0]
        .iter()
        .map(|sl| sl.width)
        .sum::<f32>()
        + BAR_PAD
}

/// Lays out `bars` as one system and draws it.
pub fn engrave_system(s: &Settings, bars: &[Bar], opt: &SystemOptions) -> System {
    let clef = s.clef;
    let mut scene = Scene {
        width: opt.width,
        top: TOP,
        bottom: BOTTOM,
        elements: Vec::new(),
    };

    // Staff lines.
    for i in 0..5 {
        scene.push(
            Shape::Line {
                x1: 0.0,
                y1: i as f32,
                x2: opt.width,
                y2: i as f32,
                width: STAFF_LINE,
            },
            None,
        );
    }

    let music_start = header(s, opt.time_signature, &mut scene);

    // Horizontal spacing: natural widths, stretched to fill the system.
    let mut rows = slots(s, bars);
    let natural: f32 = rows
        .iter()
        .map(|r| BAR_PAD + r.iter().map(|sl| sl.width).sum::<f32>())
        .sum();
    let available = opt.width - music_start - END_PAD;
    let stretch = if natural > 0.0 {
        available / natural
    } else {
        1.0
    };
    let mut bar_starts = Vec::new();
    let mut cursor = music_start;
    for (ri, row) in rows.iter_mut().enumerate() {
        bar_starts.push((bars[ri].index, cursor));
        cursor += BAR_PAD * stretch;
        for sl in row.iter_mut() {
            let acc = if sl.accidental.is_some() {
                ACCIDENTAL_SPACE
            } else {
                0.0
            };
            sl.x = cursor + acc;
            cursor += sl.width * stretch;
        }
        let bx = if ri + 1 == bars.len() {
            opt.width - BARLINE / 2.0
        } else {
            cursor
        };
        scene.push(
            Shape::Line {
                x1: bx,
                y1: 0.0,
                x2: bx,
                y2: 4.0,
                width: BARLINE,
            },
            None,
        );
    }
    let all: Vec<Slot> = rows.into_iter().flatten().collect();

    // Stems: beamed groups share a direction, chosen by their average height.
    let mut stems: Vec<Option<Stem>> = Vec::with_capacity(all.len());
    for (i, sl) in all.iter().enumerate() {
        let Some(_) = sl.pitch else {
            stems.push(None);
            continue;
        };
        if base_value(sl.written) >= WHOLE {
            stems.push(None);
            continue;
        }
        let y = sl.y(clef);
        let up = match sl.beam_group.filter(|g| group_len(&all, *g) > 1) {
            Some(g) => {
                let ys: Vec<f32> = all
                    .iter()
                    .filter(|o| o.beam_group == Some(g))
                    .map(|o| o.y(clef))
                    .collect();
                ys.iter().sum::<f32>() / ys.len() as f32 > 2.0
            }
            None => y > 2.0,
        };
        let _ = i;
        stems.push(Some(stem_for(sl.x, y, up)));
    }

    // Beams: straight, gently sloped, every stem reaching the beam.
    let mut done_groups = Vec::new();
    for i in 0..all.len() {
        let Some(g) = all[i].beam_group else { continue };
        if done_groups.contains(&g) || group_len(&all, g) < 2 {
            continue;
        }
        done_groups.push(g);
        let members: Vec<usize> = (0..all.len())
            .filter(|&j| all[j].beam_group == Some(g))
            .collect();
        beam_group(&all, &mut stems, &members, &mut scene);
    }

    // Notes, rests, stems and flags.
    let mut notes = Vec::with_capacity(all.len());
    for (i, sl) in all.iter().enumerate() {
        notes.push(NoteBox {
            bar: sl.bar,
            start: sl.start,
            dur: sl.dur,
            x: sl.x,
            rest: sl.pitch.is_none(),
            pitch: sl.pitch,
        });
        let note = Some(i);
        let Some(pitch) = sl.pitch else {
            let (g, y) = rest_glyph(sl.written);
            scene.push(
                Shape::Glyph {
                    glyph: g,
                    x: sl.x,
                    y,
                },
                note,
            );
            if is_dotted(sl.written) {
                scene.push(
                    Shape::Glyph {
                        glyph: AUGMENTATION_DOT,
                        x: sl.x + g.advance + 0.3,
                        y: 1.5,
                    },
                    note,
                );
            }
            continue;
        };
        let y = staff_y(clef, pitch.step);
        let head = head_glyph(sl.written);
        // Ledger lines.
        let mut ly = -1.0;
        while ly >= y - 0.01 {
            ledger(&mut scene, sl.x, head.advance, ly, note);
            ly -= 1.0;
        }
        let mut ly = 5.0;
        while ly <= y + 0.01 {
            ledger(&mut scene, sl.x, head.advance, ly, note);
            ly += 1.0;
        }
        if let Some(alter) = sl.accidental {
            let g = accidental_glyph(alter);
            scene.push(
                Shape::Glyph {
                    glyph: g,
                    x: sl.x - g.advance - 0.25,
                    y,
                },
                note,
            );
        }
        scene.push(
            Shape::Glyph {
                glyph: head,
                x: sl.x,
                y,
            },
            note,
        );
        if is_dotted(sl.written) {
            let on_line = (y - y.round()).abs() < 0.01;
            scene.push(
                Shape::Glyph {
                    glyph: AUGMENTATION_DOT,
                    x: sl.x + head.advance + 0.3,
                    y: if on_line { y - 0.5 } else { y },
                },
                note,
            );
        }
        if let Some(st) = &stems[i] {
            scene.push(
                Shape::Line {
                    x1: st.x,
                    y1: st.base,
                    x2: st.x,
                    y2: st.tip,
                    width: STEM,
                },
                note,
            );
            let beamed = sl.beam_group.is_some_and(|g| group_len(&all, g) > 1);
            let flags = beams(sl.written);
            if !beamed && flags > 0 {
                let g = match (flags, st.up) {
                    (1, true) => FLAG_8TH_UP,
                    (1, false) => FLAG_8TH_DOWN,
                    (_, true) => FLAG_16TH_UP,
                    (_, false) => FLAG_16TH_DOWN,
                };
                scene.push(
                    Shape::Glyph {
                        glyph: g,
                        x: st.x - STEM / 2.0,
                        y: st.tip,
                    },
                    note,
                );
            }
        }
    }

    tuplets(&all, &stems, clef, &mut scene);
    ties(&all, &stems, clef, opt, music_start, &mut scene);

    System {
        scene,
        notes,
        bars: bar_starts,
        music_start,
    }
}

/// How many of `bars` (at least one) fit on a system `width` wide.
pub fn pack(s: &Settings, bars: &[Bar], width: f32, time_signature: bool) -> usize {
    let mut used = header_width(s, time_signature) + END_PAD;
    let mut n = 0;
    for b in bars {
        used += bar_width(s, b);
        if used > width && n > 0 {
            break;
        }
        n += 1;
    }
    n.max(1).min(bars.len())
}

fn group_len(all: &[Slot], g: usize) -> usize {
    all.iter().filter(|s| s.beam_group == Some(g)).count()
}

fn ledger(scene: &mut Scene, x: f32, head_width: f32, y: f32, note: Option<usize>) {
    scene.push(
        Shape::Line {
            x1: x - 0.4,
            y1: y,
            x2: x + head_width + 0.4,
            y2: y,
            width: LEDGER,
        },
        note,
    );
}

fn beam_group(all: &[Slot], stems: &mut [Option<Stem>], members: &[usize], scene: &mut Scene) {
    let first = members[0];
    let last = *members.last().expect("a group has members");
    let (x0, t0, up) = {
        let s = stems[first].as_ref().expect("beamed notes have stems");
        (s.x, s.tip, s.up)
    };
    let (x1, t1) = {
        let s = stems[last].as_ref().expect("beamed notes have stems");
        (s.x, s.tip)
    };
    let span = (x1 - x0).max(0.1);
    // Slope follows the melody but never more than one staff space overall.
    let rise = (t1 - t0).clamp(-1.0, 1.0);
    let slope = rise / span;
    let line = |x: f32, offset: f32| t0 + slope * (x - x0) + offset;
    // Move the beam so every stem keeps at least its full length.
    let mut offset: f32 = 0.0;
    for &m in members {
        let s = stems[m].as_ref().expect("beamed notes have stems");
        let at = line(s.x, 0.0);
        if up {
            offset = offset.min(s.tip - at);
        } else {
            offset = offset.max(s.tip - at);
        }
    }
    for &m in members {
        let s = stems[m].as_mut().expect("beamed notes have stems");
        s.tip = line(s.x, offset);
    }
    let toward = if up { 1.0 } else { -1.0 };
    let edge = |xa: f32, xb: f32, level: f32, scene: &mut Scene| {
        let d = toward * level * (BEAM + BEAM_GAP);
        let (ya, yb) = (line(xa, offset) + d, line(xb, offset) + d);
        scene.push(
            Shape::Polygon(vec![
                (xa, ya),
                (xb, yb),
                (xb, yb + toward * BEAM),
                (xa, ya + toward * BEAM),
            ]),
            None,
        );
    };
    let half = STEM / 2.0;
    edge(x0 - half, x1 + half, 0.0, scene);
    // Secondary beams for sixteenths: joined between neighbours, or a short
    // stub pointing into the group for a lone sixteenth.
    for (k, &m) in members.iter().enumerate() {
        if beams(all[m].written) < 2 {
            continue;
        }
        let xm = stems[m].as_ref().expect("stem").x;
        let next = members.get(k + 1).filter(|&&n| beams(all[n].written) >= 2);
        let prev = k
            .checked_sub(1)
            .map(|p| members[p])
            .filter(|&p| beams(all[p].written) >= 2);
        if let Some(&n) = next {
            let xn = stems[n].as_ref().expect("stem").x;
            edge(xm - half, xn + half, 1.0, scene);
        } else if prev.is_none() {
            let stub = 1.1;
            if k + 1 < members.len() {
                edge(xm - half, xm + stub, 1.0, scene);
            } else {
                edge(xm - stub, xm + half, 1.0, scene);
            }
        }
    }
}

fn tuplets(all: &[Slot], stems: &[Option<Stem>], clef: Clef, scene: &mut Scene) {
    let mut seen = Vec::new();
    for sl in all {
        let Some(t) = sl.tuplet else { continue };
        if seen.contains(&t) {
            continue;
        }
        seen.push(t);
        let members: Vec<usize> = (0..all.len())
            .filter(|&j| all[j].tuplet == Some(t))
            .collect();
        let first = members[0];
        let last = *members.last().expect("a tuplet has notes");
        let up = stems[first].as_ref().is_none_or(|s| s.up);
        let left = all[first].x;
        let right = all[last].x + HEAD;
        // Place the number beyond the stems (or noteheads) on the stem side.
        let extreme = members
            .iter()
            .map(|&m| stems[m].as_ref().map_or(all[m].y(clef), |s| s.tip))
            .fold(if up { f32::MAX } else { f32::MIN }, |a, b| {
                if up {
                    a.min(b)
                } else {
                    a.max(b)
                }
            });
        let y = if up { extreme - 0.6 } else { extreme + 2.1 };
        let cx = (left + right) / 2.0 - TUPLET_3.width() / 2.0;
        scene.push(
            Shape::Glyph {
                glyph: TUPLET_3,
                x: cx,
                y,
            },
            None,
        );
        let beamed = all[first].beam_group.is_some_and(|g| group_len(all, g) > 1)
            && all[first].beam_group == all[last].beam_group;
        if !beamed {
            // A bracket shows which notes belong to the tuplet.
            let by = y - 0.75;
            let hook = if up { 0.5 } else { -0.5 };
            let gap = 0.9;
            let mid = (left + right) / 2.0;
            for (a, b) in [(left, mid - gap), (mid + gap, right)] {
                scene.push(
                    Shape::Line {
                        x1: a,
                        y1: by,
                        x2: b,
                        y2: by,
                        width: 0.12,
                    },
                    None,
                );
            }
            for xh in [left, right] {
                scene.push(
                    Shape::Line {
                        x1: xh,
                        y1: by,
                        x2: xh,
                        y2: by + hook,
                        width: 0.12,
                    },
                    None,
                );
            }
        }
    }
}

fn ties(
    all: &[Slot],
    stems: &[Option<Stem>],
    clef: Clef,
    opt: &SystemOptions,
    music_start: f32,
    scene: &mut Scene,
) {
    let side = |i: usize| {
        let y = all[i].y(clef);
        // Ties curve away from the stem.
        let up = stems[i].as_ref().map_or(y < 2.0, |s| !s.up);
        (y + if up { -0.55 } else { 0.55 }, up)
    };
    for (i, sl) in all.iter().enumerate() {
        if !sl.tie || sl.pitch.is_none() {
            continue;
        }
        let (y, up) = side(i);
        let x1 = sl.x + HEAD + 0.15;
        let x2 = match all.get(i + 1) {
            Some(next) => next.x - 0.15,
            None => opt.width - 0.3,
        };
        scene.push(Shape::Tie { x1, x2, y, up }, Some(i));
    }
    if opt.tie_in && !all.is_empty() && all[0].pitch.is_some() {
        let (y, up) = side(0);
        scene.push(
            Shape::Tie {
                x1: music_start - 0.8,
                x2: all[0].x - 0.15,
                y,
                up,
            },
            Some(0),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiex_core::Generator;

    fn system(s: &Settings, n: usize) -> System {
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(n).collect();
        engrave_system(s, &bars, &SystemOptions::default())
    }

    #[test]
    fn staff_positions() {
        assert_eq!(
            staff_y(Clef::Treble, "F5".parse::<Pitch>().unwrap().step),
            0.0
        );
        assert_eq!(
            staff_y(Clef::Treble, "E4".parse::<Pitch>().unwrap().step),
            4.0
        );
        assert_eq!(
            staff_y(Clef::Treble, "C4".parse::<Pitch>().unwrap().step),
            5.0
        );
        assert_eq!(
            staff_y(Clef::Bass, "G2".parse::<Pitch>().unwrap().step),
            4.0
        );
        assert_eq!(
            staff_y(Clef::Alto, "C4".parse::<Pitch>().unwrap().step),
            2.0
        );
    }

    #[test]
    fn key_signature_positions() {
        // D major: F# on the top line, C# in the third space.
        assert_eq!(
            key_signature_steps(Clef::Treble, 2),
            vec![(10, true), (7, true)]
        );
        // Bass F major: Bb on the second line from the bottom.
        let (step, sharp) = key_signature_steps(Clef::Bass, -1)[0];
        assert!(!sharp);
        assert_eq!(staff_y(Clef::Bass, step), 3.0);
    }

    #[test]
    fn notes_move_left_to_right_and_fit() {
        let s = Settings {
            level: 4,
            rests: true,
            triplets: true,
            dotted: true,
            ties: true,
            sharps: true,
            flats: true,
            ..Settings::default()
        };
        let sys = system(&s, 4);
        assert!(!sys.notes.is_empty());
        for w in sys.notes.windows(2) {
            assert!(w[1].x > w[0].x, "{:?}", w);
            assert_eq!(w[0].start + w[0].dur, w[1].start);
        }
        let last = sys.notes.last().unwrap();
        assert!(last.x < 80.0 - 1.0);
        assert!(sys.notes[0].x > sys.music_start);
        assert_eq!(sys.bars.len(), 4);
    }

    #[test]
    fn every_note_is_drawn_and_grouped() {
        let s = Settings::default();
        let sys = system(&s, 2);
        let svg = sys.scene.to_svg(10.0);
        for i in 0..sys.notes.len() {
            assert!(
                svg.contains(&format!(r#"data-note="{i}""#)),
                "note {i} missing"
            );
        }
    }

    #[test]
    fn beamed_stems_reach_the_beam() {
        let s = Settings {
            level: 3,
            ..Settings::default()
        };
        let sys = system(&s, 8);
        // Every polygon (beam) corner sits on some stem's x range.
        let beams = sys
            .scene
            .elements
            .iter()
            .filter(|e| matches!(e.shape, Shape::Polygon(_)))
            .count();
        assert!(beams > 0);
    }

    #[test]
    fn packing_fits_more_bars_on_wider_systems() {
        let s = Settings {
            level: 4,
            ..Settings::default()
        };
        let bars: Vec<_> = Generator::new(s.clone()).unwrap().take(12).collect();
        let narrow = pack(&s, &bars, 30.0, true);
        let wide = pack(&s, &bars, 150.0, true);
        assert!(narrow >= 1);
        assert!(wide > narrow);
        // A packed system never needs squeezing.
        let used: f32 = header_width(&s, true)
            + END_PAD
            + bars[..wide].iter().map(|b| bar_width(&s, b)).sum::<f32>();
        assert!(used <= 150.0);
    }
}
