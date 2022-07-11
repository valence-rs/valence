use std::io::{Read, Write};

use crate::protocol_inner::{Decode, Encode};

/// Represents an angle in steps of 1/256 of a full turn.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ByteAngle(pub u8);

impl ByteAngle {
    pub fn from_degrees(f: f32) -> ByteAngle {
        ByteAngle((f.rem_euclid(360.0) / 360.0 * 256.0).round() as u8)
    }

    pub fn to_degrees(self) -> f32 {
        self.0 as f32 / 256.0 * 360.0
    }
}

impl Encode for ByteAngle {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.0.encode(w)
    }
}

impl Decode for ByteAngle {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        u8::decode(r).map(ByteAngle)
    }
}
