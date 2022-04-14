// TODO: rename to ByteAngle?

use std::f64::consts::TAU;
use std::io::{Read, Write};

use crate::protocol::{Decode, Encode};

/// Represents an angle in steps of 1/256 of a full turn.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ByteAngle(pub u8);

impl ByteAngle {
    pub fn from_radians_f64(f: f64) -> ByteAngle {
        ByteAngle((f.rem_euclid(TAU) / TAU * 256.0).round() as u8)
    }

    pub fn to_radians_f64(self) -> f64 {
        self.0 as f64 / 256.0 * TAU
    }
}

impl Encode for ByteAngle {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.0.encode(w)
    }
}

impl Decode for ByteAngle {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        Ok(ByteAngle(u8::decode(r)?))
    }
}
