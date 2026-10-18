//! Transposing instruments: the part is written at one pitch and sounds at another.

use std::fmt;
use std::str::FromStr;

use crate::pitch::ParseError;

/// The instrument the part is written for. Exercises are always generated and
/// engraved at written pitch; only the sound is transposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Instrument {
    /// Sounds as written: piano, flute, violin, voice.
    #[default]
    Concert,
    /// Clarinet, trumpet, soprano sax: sounds a major second lower.
    Bb,
    /// Clarinet in A: sounds a minor third lower.
    A,
    /// Alto sax, alto clarinet: sounds a major sixth lower.
    Eb,
    /// French horn, English horn: sounds a perfect fifth lower.
    F,
    /// Tenor sax, bass clarinet: sounds a major ninth lower.
    TenorSax,
    /// Baritone sax: sounds an octave and a major sixth lower.
    BariSax,
    /// E-flat clarinet: sounds a minor third higher.
    EbClarinet,
    /// Guitar, double bass: sounds an octave lower.
    Guitar,
}

impl Instrument {
    pub const ALL: [Instrument; 9] = [
        Instrument::Concert,
        Instrument::Bb,
        Instrument::A,
        Instrument::Eb,
        Instrument::F,
        Instrument::TenorSax,
        Instrument::BariSax,
        Instrument::EbClarinet,
        Instrument::Guitar,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Instrument::Concert => "concert",
            Instrument::Bb => "bb",
            Instrument::A => "a",
            Instrument::Eb => "eb",
            Instrument::F => "f",
            Instrument::TenorSax => "tenor-sax",
            Instrument::BariSax => "bari-sax",
            Instrument::EbClarinet => "eb-clarinet",
            Instrument::Guitar => "guitar",
        }
    }

    /// Semitones from written to sounding pitch (negative sounds lower).
    pub fn transpose(self) -> i32 {
        match self {
            Instrument::Concert => 0,
            Instrument::Bb => -2,
            Instrument::A => -3,
            Instrument::Eb => -9,
            Instrument::F => -7,
            Instrument::TenorSax => -14,
            Instrument::BariSax => -21,
            Instrument::EbClarinet => 3,
            Instrument::Guitar => -12,
        }
    }
}

impl fmt::Display for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Instrument {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase().replace([' ', '_'], "-");
        Ok(match s.as_str() {
            "concert" | "c" | "piano" | "flute" | "violin" => Instrument::Concert,
            "bb" | "b-flat" | "clarinet" | "trumpet" | "soprano-sax" => Instrument::Bb,
            "a" | "a-clarinet" => Instrument::A,
            "eb" | "e-flat" | "alto-sax" => Instrument::Eb,
            "f" | "horn" | "french-horn" | "english-horn" => Instrument::F,
            "tenor-sax" | "bass-clarinet" => Instrument::TenorSax,
            "bari-sax" | "baritone-sax" => Instrument::BariSax,
            "eb-clarinet" => Instrument::EbClarinet,
            "guitar" | "double-bass" => Instrument::Guitar,
            _ => {
                return Err(ParseError(format!(
                    "invalid instrument '{s}', expected one of: {}",
                    Instrument::ALL.map(Instrument::name).join(", ")
                )));
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for i in Instrument::ALL {
            assert_eq!(i.name().parse::<Instrument>().unwrap(), i);
        }
        assert_eq!("Alto Sax".parse::<Instrument>().unwrap(), Instrument::Eb);
        assert!("kazoo".parse::<Instrument>().is_err());
    }
}
