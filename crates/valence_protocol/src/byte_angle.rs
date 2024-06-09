use std::f32::consts::TAU;
use std::fmt;
use std::io::Write;

use crate::{Decode, Encode};

/// Represents an angle in steps of 1/256 of a full turn.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteAngle(pub u8);

impl ByteAngle {
    pub fn from_degrees(f: f32) -> ByteAngle {
        ByteAngle((f.rem_euclid(360.0) / 360.0 * 256.0).round() as u8)
    }

    pub fn from_radians(f: f32) -> ByteAngle {
        ByteAngle((f.rem_euclid(TAU) / TAU * 256.0).round() as u8)
    }

    pub fn to_degrees(self) -> f32 {
        f32::from(self.0) / 256.0 * 360.0
    }

    pub fn to_radians(self) -> f32 {
        f32::from(self.0) / 256.0 * TAU
    }
}

impl fmt::Debug for ByteAngle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for ByteAngle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}°", self.to_degrees())
    }
}

impl Encode for ByteAngle {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.0.encode(w)
    }
}

impl Decode<'_> for ByteAngle {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        u8::decode(r).map(ByteAngle)
    }
}
