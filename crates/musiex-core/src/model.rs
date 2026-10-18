//! The generated music: bars made of notes, rests and tuplets.

use crate::duration::Dur;
use crate::pitch::Pitch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub pitch: Pitch,
    /// Written duration. Inside a tuplet the sounding duration is shorter.
    pub dur: Dur,
    /// Tied to the next note, which then has the same pitch.
    pub tie: bool,
}

/// `p` notes in the time of `q`, e.g. a triplet is 3 in the time of 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tuplet {
    pub p: u8,
    pub q: u8,
    pub notes: Vec<Note>,
}

impl Tuplet {
    /// Sounding duration of one written value inside this tuplet.
    pub fn actual(&self, written: Dur) -> Dur {
        written * Dur::from(self.q) / Dur::from(self.p)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Note(Note),
    Rest(Dur),
    Tuplet(Tuplet),
}

impl Event {
    /// Sounding duration of the whole event.
    pub fn dur(&self) -> Dur {
        match self {
            Event::Note(n) => n.dur,
            Event::Rest(d) => *d,
            Event::Tuplet(t) => t.notes.iter().map(|n| t.actual(n.dur)).sum(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bar {
    /// Position in the exercise, starting at 0.
    pub index: u64,
    pub events: Vec<Event>,
}

impl Bar {
    pub fn dur(&self) -> Dur {
        self.events.iter().map(Event::dur).sum()
    }

    /// All notes in order, including those inside tuplets.
    pub fn notes(&self) -> impl Iterator<Item = &Note> {
        self.events.iter().flat_map(|e| match e {
            Event::Note(n) => std::slice::from_ref(n).iter(),
            Event::Tuplet(t) => t.notes.iter(),
            Event::Rest(_) => [].iter(),
        })
    }
}
