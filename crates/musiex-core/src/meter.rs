//! Time signatures and how bars divide into beats.

use crate::duration::{Dur, WHOLE};
use crate::pitch::ParseError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meter {
    pub num: u8,
    pub den: u8,
}

impl Meter {
    pub fn new(num: u8, den: u8) -> Result<Self, ParseError> {
        if !matches!(den, 2 | 4 | 8 | 16) {
            return Err(ParseError(format!(
                "invalid meter {num}/{den}: the lower number must be 2, 4, 8 or 16"
            )));
        }
        if !(1..=16).contains(&num) {
            return Err(ParseError(format!(
                "invalid meter {num}/{den}: the upper number must be 1 to 16"
            )));
        }
        Ok(Meter { num, den })
    }

    pub fn bar_len(self) -> Dur {
        WHOLE / Dur::from(self.den) * Dur::from(self.num)
    }

    /// 6/8, 9/8, 12/8...: beats are dotted and divide in three.
    pub fn is_compound(self) -> bool {
        self.den >= 8 && self.num % 3 == 0 && self.num > 3
    }

    /// The felt beat: a dotted value in compound meters, the lower number otherwise.
    pub fn beat(self) -> Dur {
        let unit = WHOLE / Dur::from(self.den);
        if self.is_compound() {
            3 * unit
        } else {
            unit
        }
    }

    pub fn beats_per_bar(self) -> u32 {
        self.bar_len() / self.beat()
    }

    /// How a bar splits at the top level, in beats: 5/4 is 3+2, 7/8 is 3+4.
    /// Meters with 1, 2, 3, 4, 8 or 16 beats are one group.
    pub fn groups(self) -> Vec<u32> {
        let n = self.beats_per_bar();
        if matches!(n, 1 | 2 | 3 | 4 | 8 | 16) {
            return vec![n];
        }
        let mut groups = Vec::new();
        let mut rest = n;
        while rest > 4 {
            groups.push(3);
            rest -= 3;
        }
        groups.push(rest);
        groups
    }
}

impl fmt::Display for Meter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl FromStr for Meter {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (num, den) = s.split_once('/').ok_or_else(|| {
            ParseError(format!("invalid meter '{s}', expected something like 3/4"))
        })?;
        let parse = |x: &str| {
            x.trim()
                .parse::<u8>()
                .map_err(|_| ParseError(format!("invalid meter '{s}'")))
        };
        Meter::new(parse(num)?, parse(den)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::{EIGHTH, QUARTER};

    #[test]
    fn simple_and_compound() {
        let m: Meter = "4/4".parse().unwrap();
        assert_eq!(m.bar_len(), WHOLE);
        assert_eq!(m.beat(), QUARTER);
        assert_eq!(m.groups(), vec![4]);

        let m: Meter = "6/8".parse().unwrap();
        assert!(m.is_compound());
        assert_eq!(m.beat(), 3 * EIGHTH);
        assert_eq!(m.groups(), vec![2]);

        let m: Meter = "3/8".parse().unwrap();
        assert!(!m.is_compound());
        assert_eq!(m.groups(), vec![3]);

        assert_eq!("5/4".parse::<Meter>().unwrap().groups(), vec![3, 2]);
        assert_eq!("7/8".parse::<Meter>().unwrap().groups(), vec![3, 4]);
        assert_eq!("12/8".parse::<Meter>().unwrap().groups(), vec![4]);
        assert!("4/3".parse::<Meter>().is_err());
        assert!("0/4".parse::<Meter>().is_err());
    }
}
