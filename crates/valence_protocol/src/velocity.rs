use std::fmt;

use derive_more::{From, Into};

use crate::{Decode, Encode};

/// Quantized entity velocity.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, From, Into)]
pub struct Velocity(pub [i16; 3]);

impl Velocity {
    /// From meters/second.
    pub fn from_ms_f32(ms: [f32; 3]) -> Self {
        Self(ms.map(|v| (8000.0 / 20.0 * v) as i16))
    }

    /// From meters/second.
    pub fn from_ms_f64(ms: [f64; 3]) -> Self {
        Self(ms.map(|v| (8000.0 / 20.0 * v) as i16))
    }

    /// To meters/second.
    pub fn to_ms_f32(self) -> [f32; 3] {
        self.0.map(|v| v as f32 / (8000.0 / 20.0))
    }

    /// To meters/second.
    pub fn to_ms_f64(self) -> [f64; 3] {
        self.0.map(|v| v as f64 / (8000.0 / 20.0))
    }
}

impl fmt::Debug for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [x, y, z] = self.to_ms_f32();
        write!(f, "⟨{x},{y},{z}⟩ m/s")
    }
}

#[cfg(test)]
#[test]
fn velocity_from_ms() {
    let ms = -3.3575;

    let val_1 = Velocity::from_ms_f32([(); 3].map(|_| -3.3575)).0[0];
    let val_2 = Velocity::from_ms_f64([(); 3].map(|_| -3.3575)).0[0];

    assert_eq!(val_1, val_2);
    assert_eq!(val_1, -1343);
}
