//! Durations measured in integer ticks.
//!
//! A whole note is 3840 ticks, which divides evenly by every value the
//! generator produces (down to sixteenth-note triplets), so no rational
//! arithmetic is needed.

pub type Dur = u32;

pub const WHOLE: Dur = 3840;
pub const HALF: Dur = WHOLE / 2;
pub const QUARTER: Dur = WHOLE / 4;
pub const EIGHTH: Dur = WHOLE / 8;
pub const SIXTEENTH: Dur = WHOLE / 16;

/// Ticks per quarter note, for MIDI and playback.
pub const PPQ: u32 = QUARTER;

/// A plain note value: whole, half, quarter... (or a power-of-two multiple of a whole).
pub fn is_binary(d: Dur) -> bool {
    if d == 0 {
        return false;
    }
    if d >= WHOLE {
        d % WHOLE == 0 && (d / WHOLE).is_power_of_two()
    } else {
        WHOLE % d == 0 && (WHOLE / d).is_power_of_two()
    }
}

/// A dotted note value: one and a half times a plain value.
pub fn is_dotted(d: Dur) -> bool {
    d % 3 == 0 && is_binary(d / 3 * 2)
}

/// Whether a single note head can show this duration (no ties needed).
pub fn is_writable(d: Dur) -> bool {
    d <= WHOLE + HALF && (is_binary(d) || is_dotted(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_values() {
        assert!(is_binary(QUARTER));
        assert!(is_binary(WHOLE));
        assert!(is_binary(2 * WHOLE));
        assert!(!is_binary(3 * QUARTER));
        assert!(is_dotted(3 * QUARTER));
        assert!(is_dotted(3 * EIGHTH));
        assert!(!is_dotted(QUARTER));
        assert!(is_writable(3 * QUARTER));
        assert!(!is_writable(5 * QUARTER));
        assert!(!is_writable(9 * EIGHTH));
    }
}
