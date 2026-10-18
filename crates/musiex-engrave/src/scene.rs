//! Drawing primitives in staff spaces (y down; the top staff line is y = 0,
//! the bottom one y = 4) and their SVG form.

use crate::Glyph;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
    },
    Glyph {
        glyph: Glyph,
        x: f32,
        y: f32,
    },
    /// A filled polygon, for beams and tuplet brackets.
    Polygon(Vec<(f32, f32)>),
    /// A tie from `x1` to `x2` at height `y`, curving up or down.
    Tie {
        x1: f32,
        x2: f32,
        y: f32,
        up: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub shape: Shape,
    /// The note (index into `System::notes`) this element belongs to, so it
    /// can be highlighted while playing.
    pub note: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    pub width: f32,
    /// Vertical extent: the top and bottom of the drawing, in staff spaces.
    pub top: f32,
    pub bottom: f32,
    pub elements: Vec<Element>,
}

fn n(v: f32) -> String {
    // Two decimals are plenty at any screen size and keep the SVG small.
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

impl Scene {
    pub fn push(&mut self, shape: Shape, note: Option<usize>) {
        self.elements.push(Element { shape, note });
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    /// The scene as an SVG document, `px_per_space` pixels per staff space.
    /// Everything is drawn in `currentColor`; each note's elements are
    /// grouped as `<g class="note" data-note="N">` for styling and highlighting.
    pub fn to_svg(&self, px_per_space: f32) -> String {
        let mut out = String::new();
        let _ = write!(
            out,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 {} {} {}" width="{}" height="{}" fill="currentColor" stroke="currentColor">"#,
            n(self.top),
            n(self.width),
            n(self.height()),
            n(self.width * px_per_space),
            n(self.height() * px_per_space),
        );
        let mut current: Option<usize> = None;
        for e in &self.elements {
            if e.note != current {
                if current.is_some() {
                    out.push_str("</g>");
                }
                if let Some(i) = e.note {
                    let _ = write!(out, r#"<g class="note" data-note="{i}">"#);
                }
                current = e.note;
            }
            shape_svg(&e.shape, &mut out);
        }
        if current.is_some() {
            out.push_str("</g>");
        }
        out.push_str("</svg>");
        out
    }
}

fn shape_svg(shape: &Shape, out: &mut String) {
    match shape {
        Shape::Line {
            x1,
            y1,
            x2,
            y2,
            width,
        } => {
            let _ = write!(
                out,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke-width="{}"/>"#,
                n(*x1),
                n(*y1),
                n(*x2),
                n(*y2),
                n(*width)
            );
        }
        Shape::Glyph { glyph, x, y } => {
            let _ = write!(
                out,
                r#"<path transform="translate({} {})" stroke="none" d="{}"/>"#,
                n(*x),
                n(*y),
                glyph.path
            );
        }
        Shape::Polygon(points) => {
            out.push_str(r#"<polygon stroke="none" points=""#);
            for (i, (x, y)) in points.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                let _ = write!(out, "{},{}", n(*x), n(*y));
            }
            out.push_str(r#""/>"#);
        }
        Shape::Tie { x1, x2, y, up } => {
            let len = x2 - x1;
            let h = (0.5 + 0.05 * len).min(1.1);
            let sign = if *up { -1.0 } else { 1.0 };
            let inner = h - 0.16;
            let a = len * 0.25;
            let _ = write!(
                out,
                r#"<path stroke="none" d="M{} {}C{} {} {} {} {} {}C{} {} {} {} {} {}Z"/>"#,
                n(*x1),
                n(*y),
                n(x1 + a),
                n(y + sign * h),
                n(x2 - a),
                n(y + sign * h),
                n(*x2),
                n(*y),
                n(x2 - a),
                n(y + sign * inner),
                n(x1 + a),
                n(y + sign * inner),
                n(*x1),
                n(*y),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_compact() {
        assert_eq!(n(1.0), "1");
        assert_eq!(n(1.25), "1.25");
        assert_eq!(n(-0.001), "0");
        assert_eq!(n(0.105), "0.1");
    }

    #[test]
    fn groups_note_elements() {
        let mut s = Scene {
            width: 10.0,
            top: -2.0,
            bottom: 6.0,
            ..Scene::default()
        };
        let line = |x| Shape::Line {
            x1: x,
            y1: 0.0,
            x2: x,
            y2: 4.0,
            width: 0.1,
        };
        s.push(line(0.0), None);
        s.push(line(1.0), Some(0));
        s.push(line(2.0), Some(0));
        s.push(line(3.0), Some(1));
        let svg = s.to_svg(10.0);
        assert!(svg.starts_with(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -2 10 8" width="100" height="80""#
        ));
        assert_eq!(svg.matches("<g ").count(), 2);
        assert_eq!(svg.matches("</g>").count(), 2);
        assert!(svg.contains(r#"<g class="note" data-note="1">"#));
    }
}
