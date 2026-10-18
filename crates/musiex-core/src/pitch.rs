//! Pitches, keys and clefs.

use std::fmt;
use std::str::FromStr;

/// Semitones above C for each letter, C D E F G A B.
const LETTER_SEMITONES: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const LETTER_NAMES: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
/// Position of each letter on the circle of fifths, relative to C.
const LETTER_FIFTHS: [i32; 7] = [0, 2, 4, -1, 1, 3, 5];
/// Letters in the order sharps are added to a key signature: F C G D A E B.
const SHARP_ORDER: [usize; 7] = [3, 0, 4, 1, 5, 2, 6];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// A written pitch: a staff position plus an alteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pitch {
    /// Diatonic step; 0 is middle C (C4), 1 is D4, -1 is B3.
    pub step: i32,
    /// -1 flat, 0 natural, +1 sharp.
    pub alter: i8,
}

impl Pitch {
    pub const MIDDLE_C: Pitch = Pitch { step: 0, alter: 0 };

    pub fn new(step: i32, alter: i8) -> Self {
        Pitch { step, alter }
    }

    /// 0 = C ... 6 = B.
    pub fn letter(self) -> usize {
        self.step.rem_euclid(7) as usize
    }

    /// Scientific octave number: middle C is in octave 4.
    pub fn octave(self) -> i32 {
        4 + self.step.div_euclid(7)
    }

    pub fn letter_name(self) -> char {
        LETTER_NAMES[self.letter()]
    }

    /// MIDI key number; middle C is 60.
    pub fn midi(self) -> i32 {
        12 * (self.octave() + 1) + LETTER_SEMITONES[self.letter()] + i32::from(self.alter)
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let accidental = match self.alter {
            -1 => "b",
            1 => "#",
            _ => "",
        };
        write!(f, "{}{}{}", self.letter_name(), accidental, self.octave())
    }
}

/// Parses names like `C4`, `F#3`, `Bb5`.
impl FromStr for Pitch {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || {
            ParseError(format!(
                "invalid pitch '{s}', expected something like C4, F#3 or Bb5"
            ))
        };
        let mut chars = s.trim().chars();
        let letter = chars.next().ok_or_else(err)?.to_ascii_uppercase();
        let letter = LETTER_NAMES
            .iter()
            .position(|&c| c == letter)
            .ok_or_else(err)? as i32;
        let rest: String = chars.collect();
        let (alter, octave) = if let Some(o) = rest.strip_prefix('#') {
            (1, o)
        } else if let Some(o) = rest.strip_prefix('b') {
            (-1, o)
        } else {
            (0, rest.as_str())
        };
        let octave: i32 = octave.parse().map_err(|_| err())?;
        if !(0..=9).contains(&octave) {
            return Err(err());
        }
        Ok(Pitch::new((octave - 4) * 7 + letter, alter))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Major,
    Minor,
}

/// A key, stored by its signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    tonic_letter: usize,
    tonic_alter: i8,
    pub mode: Mode,
    /// Number of sharps (positive) or flats (negative) in the signature.
    pub fifths: i8,
}

impl Key {
    pub const C_MAJOR: Key = Key {
        tonic_letter: 0,
        tonic_alter: 0,
        mode: Mode::Major,
        fifths: 0,
    };

    /// The alteration the key signature applies to a letter (0 = C ... 6 = B).
    pub fn alter_for(self, letter: usize) -> i8 {
        let pos = SHARP_ORDER
            .iter()
            .position(|&l| l == letter)
            .expect("letter is 0..7") as i8;
        if self.fifths > 0 && pos < self.fifths {
            1
        } else if self.fifths < 0 && pos >= 7 + self.fifths {
            -1
        } else {
            0
        }
    }

    /// The natural pitch of this key at a given step: what is written with no accidental.
    pub fn diatonic(self, step: i32) -> Pitch {
        let p = Pitch::new(step, 0);
        Pitch::new(step, self.alter_for(p.letter()))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let accidental = match self.tonic_alter {
            -1 => "b",
            1 => "#",
            _ => "",
        };
        let mode = match self.mode {
            Mode::Major => "",
            Mode::Minor => "m",
        };
        write!(
            f,
            "{}{}{}",
            LETTER_NAMES[self.tonic_letter], accidental, mode
        )
    }
}

/// Parses names like `C`, `F#`, `Bb`, `Am`, `C#m`, `Ebm`.
impl FromStr for Key {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = |why: &str| ParseError(format!("invalid key '{s}': {why}"));
        let mut chars = s.trim().chars();
        let letter = chars
            .next()
            .ok_or_else(|| err("empty"))?
            .to_ascii_uppercase();
        let tonic_letter = LETTER_NAMES
            .iter()
            .position(|&c| c == letter)
            .ok_or_else(|| err("expected a letter A-G"))?;
        let rest: String = chars.collect();
        let (tonic_alter, rest) = if let Some(r) = rest.strip_prefix('#') {
            (1, r)
        } else if let Some(r) = rest.strip_prefix('b') {
            (-1, r)
        } else {
            (0, rest.as_str())
        };
        let mode = match rest {
            "" | "maj" => Mode::Major,
            "m" | "min" => Mode::Minor,
            _ => {
                return Err(err(
                    "expected an optional #/b and an optional 'm' for minor",
                ))
            }
        };
        let mut fifths = LETTER_FIFTHS[tonic_letter] + 7 * i32::from(tonic_alter);
        if mode == Mode::Minor {
            fifths -= 3;
        }
        if !(-7..=7).contains(&fifths) {
            return Err(err("needs more than seven sharps or flats"));
        }
        Ok(Key {
            tonic_letter,
            tonic_alter,
            mode,
            fifths: fifths as i8,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clef {
    Treble,
    Bass,
    Alto,
    Tenor,
}

impl Clef {
    pub fn name(self) -> &'static str {
        match self {
            Clef::Treble => "treble",
            Clef::Bass => "bass",
            Clef::Alto => "alto",
            Clef::Tenor => "tenor",
        }
    }

    /// A comfortable reading range for this clef: about a ledger line either side of the staff.
    pub fn default_range(self) -> (Pitch, Pitch) {
        let p = |s: &str| s.parse::<Pitch>().expect("valid built-in pitch");
        match self {
            Clef::Treble => (p("C4"), p("A5")),
            Clef::Bass => (p("E2"), p("C4")),
            Clef::Alto => (p("D3"), p("B4")),
            Clef::Tenor => (p("B2"), p("G4")),
        }
    }
}

impl FromStr for Clef {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "treble" | "g" => Ok(Clef::Treble),
            "bass" | "f" => Ok(Clef::Bass),
            "alto" | "c" => Ok(Clef::Alto),
            "tenor" => Ok(Clef::Tenor),
            _ => Err(ParseError(format!(
                "invalid clef '{s}', expected treble, bass, alto or tenor"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pitches() {
        assert_eq!("C4".parse::<Pitch>().unwrap(), Pitch::MIDDLE_C);
        let fs3: Pitch = "F#3".parse().unwrap();
        assert_eq!(fs3.midi(), 54);
        assert_eq!(fs3.to_string(), "F#3");
        let bb5: Pitch = "Bb5".parse().unwrap();
        assert_eq!(bb5.midi(), 82);
        assert_eq!("A0".parse::<Pitch>().unwrap().midi(), 21);
        assert!("H4".parse::<Pitch>().is_err());
        assert!("C".parse::<Pitch>().is_err());
    }

    #[test]
    fn negative_steps() {
        let b3 = Pitch::new(-1, 0);
        assert_eq!(b3.letter_name(), 'B');
        assert_eq!(b3.octave(), 3);
        assert_eq!(b3.midi(), 59);
    }

    #[test]
    fn key_signatures() {
        let fifths = |s: &str| s.parse::<Key>().unwrap().fifths;
        assert_eq!(fifths("C"), 0);
        assert_eq!(fifths("G"), 1);
        assert_eq!(fifths("F"), -1);
        assert_eq!(fifths("Bb"), -2);
        assert_eq!(fifths("F#"), 6);
        assert_eq!(fifths("C#"), 7);
        assert_eq!(fifths("Am"), 0);
        assert_eq!(fifths("Em"), 1);
        assert_eq!(fifths("Dm"), -1);
        assert_eq!(fifths("Ebm"), -6);
        assert!("Fb".parse::<Key>().is_err());
        assert!("Xm".parse::<Key>().is_err());
    }

    #[test]
    fn key_alterations() {
        let d: Key = "D".parse().unwrap();
        // F# and C# in D major.
        assert_eq!(d.alter_for(3), 1);
        assert_eq!(d.alter_for(0), 1);
        assert_eq!(d.alter_for(4), 0);
        let bb: Key = "Bb".parse().unwrap();
        // Bb and Eb.
        assert_eq!(bb.alter_for(6), -1);
        assert_eq!(bb.alter_for(2), -1);
        assert_eq!(bb.alter_for(5), 0);
        assert_eq!(bb.to_string(), "Bb");
        assert_eq!("c#m".parse::<Key>().unwrap().to_string(), "C#m");
    }
}
