use std::f32::consts::TAU;
use std::io::Write;

use crate::{Decode, Encode, Result};

/// Represents an angle in steps of 1/256 of a full turn.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ByteAngle(pub u8);

impl ByteAngle {
    pub fn from_degrees(f: f32) -> ByteAngle {
        ByteAngle((f.rem_euclid(360.0) / 360.0 * 256.0).round() as u8)
    }

    pub fn from_radians(f: f32) -> ByteAngle {
        ByteAngle((f.rem_euclid(TAU) / TAU * 256.0).round() as u8)
    }

    pub fn to_degrees(self) -> f32 {
        self.0 as f32 / 256.0 * 360.0
    }

    pub fn to_radians(self) -> f32 {
        self.0 as f32 / 256.0 * TAU
    }
}

impl Encode for ByteAngle {
    fn encode(&self, w: impl Write) -> Result<()> {
        self.0.encode(w)
    }
}

impl Decode<'_> for ByteAngle {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        u8::decode(r).map(ByteAngle)
    }
}
