//! Musiex core: generates endless sight-reading exercises.
//!
//! [`Generator`] yields [`Bar`]s forever from a set of [`Settings`]; the same
//! settings and seed always give the same music. [`abc`] writes them as ABC
//! notation and [`perform`] turns them into timed notes for MIDI or audio.

pub mod abc;
pub mod duration;
pub mod generator;
mod melody;
pub mod meter;
pub mod model;
pub mod perform;
pub mod pitch;
mod rhythm;
pub mod rng;
pub mod settings;

pub use generator::Generator;
pub use meter::Meter;
pub use model::{Bar, Event, Note, Tuplet};
pub use pitch::{Clef, Key, ParseError, Pitch};
pub use settings::Settings;
