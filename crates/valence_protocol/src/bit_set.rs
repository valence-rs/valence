use std::{io::Write, fmt};

use crate::{Encode, Decode};

// TODO: when better const exprs are available, compute BYTE_COUNT from BIT_COUNT.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct FixedBitSet<const BIT_COUNT: usize, const BYTE_COUNT: usize>(pub [u8; BYTE_COUNT]);

impl<const BIT_COUNT: usize, const BYTE_COUNT: usize> FixedBitSet<BIT_COUNT, BYTE_COUNT> {
    pub fn bit(&self, idx: usize) -> bool {
        check_counts(BIT_COUNT, BYTE_COUNT);
        assert!(idx < BIT_COUNT, "bit index of {idx} out of range for bitset with {BIT_COUNT} bits");

        self.0[idx / 8] >> idx % 8 & 1 == 1
    }

    pub fn set_bit(&mut self, idx: usize, val: bool) {
        check_counts(BIT_COUNT, BYTE_COUNT);
        assert!(idx < BIT_COUNT, "bit index of {idx} out of range for bitset with {BIT_COUNT} bits");

        let byte = &mut self.0[idx / 8];
        *byte = *byte | (val as u8) << idx % 8;
    }
}

impl<const BIT_COUNT: usize, const BYTE_COUNT: usize> Encode for FixedBitSet<BIT_COUNT, BYTE_COUNT> {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        check_counts(BIT_COUNT, BYTE_COUNT);
        self.0.encode(w)
    }
}

impl<const BIT_COUNT: usize, const BYTE_COUNT: usize> Decode<'_> for FixedBitSet<BIT_COUNT, BYTE_COUNT> {
    fn decode(r: &mut &'_ [u8]) -> anyhow::Result<Self> {
        check_counts(BIT_COUNT, BYTE_COUNT);
        Ok(Self(Decode::decode(r)?))
    }
}

const fn check_counts(bits: usize, bytes: usize) {
    assert!((bits + 7) / 8 == bytes)
}

impl<const BIT_COUNT: usize, const BYTE_COUNT: usize> fmt::Debug for FixedBitSet<BIT_COUNT, BYTE_COUNT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<const BIT_COUNT: usize, const BYTE_COUNT: usize> fmt::Display for FixedBitSet<BIT_COUNT, BYTE_COUNT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0b")?;

        for i in (0..BIT_COUNT).rev() {
            if self.bit(i) {
                write!(f, "1")?;
            } else {
                write!(f, "0")?;
            }
        }

        Ok(())
    }
}

/// 😔
macro_rules! impl_default {
    ($($N:literal)*) => {
        $(
            impl<const BIT_COUNT: usize> Default for FixedBitSet<BIT_COUNT, $N> {
                fn default() -> Self {
                    Self(Default::default())
                }
            }
        )*
    }
}

impl_default!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_bit_set_ops() {
        let mut bits = FixedBitSet::<20, 3>::default();
        
        assert!(!bits.bit(5));
        bits.set_bit(5, true);
        assert!(bits.bit(5));
        assert_eq!(bits.0, [0b00100000, 0, 0]);
    }

    #[test]
    #[should_panic]
    fn fixed_bit_set_out_of_range() {
        let mut bits = FixedBitSet::<20, 3>::default();

        bits.set_bit(20, true);
    }

    #[test]
    fn display_fixed_bit_set() {
        let mut bits = FixedBitSet::<20, 3>::default();
        bits.set_bit(5, true);

        assert_eq!(format!("{bits}"), "0b00000000000000100000");
    }
}

