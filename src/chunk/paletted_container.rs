use std::array;
use std::io::Write;

use arrayvec::ArrayVec;

use crate::chunk::{compact_u64s_len, encode_compact_u64s};
use crate::protocol::{Encode, VarInt};
use crate::util::log2_ceil;

/// `HALF_LEN` must be equal to `ceil(LEN / 2)`.
#[derive(Clone, Debug)]
pub enum PalettedContainer<T, const LEN: usize, const HALF_LEN: usize> {
    Single(T),
    Indirect(Box<Indirect<T, LEN, HALF_LEN>>),
    Direct(Box<[T; LEN]>),
}

#[derive(Clone, Debug)]
pub struct Indirect<T, const LEN: usize, const HALF_LEN: usize> {
    /// Each element is a unique instance of `T`.
    palette: ArrayVec<T, 16>,
    /// Each half-byte is an index into `palette`.
    indices: [u8; HALF_LEN],
}

impl<T: Copy + Eq + Default, const LEN: usize, const HALF_LEN: usize>
    PalettedContainer<T, LEN, HALF_LEN>
{
    pub fn new() -> Self {
        assert_eq!(num::Integer::div_ceil(&LEN, &2), HALF_LEN);
        assert_ne!(LEN, 0);

        Self::Single(T::default())
    }

    pub fn fill(&mut self, val: T) {
        *self = Self::Single(val)
    }

    pub fn get(&self, idx: usize) -> T {
        self.check_oob(idx);

        match self {
            Self::Single(elem) => *elem,
            Self::Indirect(ind) => ind.get(idx),
            Self::Direct(elems) => elems[idx],
        }
    }

    pub fn set(&mut self, idx: usize, val: T) -> T {
        self.check_oob(idx);

        match self {
            Self::Single(old_val) => {
                if *old_val == val {
                    *old_val
                } else {
                    // Upgrade to indirect.
                    let old = *old_val;
                    let mut ind = Box::new(Indirect {
                        palette: ArrayVec::from_iter([old, val]),
                        // All indices are initialized to index 0 (the old element).
                        indices: [0; HALF_LEN],
                    });

                    ind.indices[idx / 2] = 1 << (idx % 2 * 4);
                    *self = Self::Indirect(ind);
                    old
                }
            }
            Self::Indirect(ind) => {
                if let Some(old) = ind.set(idx, val) {
                    old
                } else {
                    // Upgrade to direct.
                    *self = Self::Direct(Box::new(array::from_fn(|i| ind.get(i))));
                    self.set(idx, val)
                }
            }
            Self::Direct(vals) => {
                let old = vals[idx];
                vals[idx] = val;
                old
            }
        }
    }

    pub fn optimize(&mut self) {
        match self {
            Self::Single(_) => {}
            Self::Indirect(ind) => {
                let mut new_ind = Indirect {
                    palette: ArrayVec::new(),
                    indices: [0; HALF_LEN],
                };

                for i in 0..LEN {
                    new_ind.set(i, ind.get(i));
                }

                if new_ind.palette.len() == 1 {
                    *self = Self::Single(new_ind.palette[0]);
                } else {
                    **ind = new_ind;
                }
            }
            Self::Direct(dir) => {
                let mut ind = Indirect {
                    palette: ArrayVec::new(),
                    indices: [0; HALF_LEN],
                };

                for (i, val) in dir.iter().cloned().enumerate() {
                    if ind.set(i, val).is_none() {
                        return;
                    }
                }

                *self = if ind.palette.len() == 1 {
                    Self::Single(ind.palette[0])
                } else {
                    Self::Indirect(Box::new(ind))
                };
            }
        }
    }

    #[inline]
    fn check_oob(&self, idx: usize) {
        assert!(
            idx < LEN,
            "index {idx} is out of bounds in paletted container of length {LEN}"
        );
    }

    /// Encodes the paletted container in the format that Minecraft expects.
    ///
    /// - **`writer`**: The [`Write`] instance to write the paletted container
    ///   to.
    /// - **`to_bits`**: A function to convert the element type to bits. The
    ///   output must be less than two to the power of `direct_bits`.
    /// - **`min_indirect_bits`**: The minimum number of bits used to represent
    ///   the element type in the indirect representation. If the bits per index
    ///   is lower, it will be rounded up to this.
    /// - **`max_indirect_bits`**: The maximum number of bits per element
    ///   allowed in the indirect representation. Any higher than this will
    ///   force conversion to the direct representation while encoding.
    /// - **`direct_bits`**: The minimum number of bits required to represent
    ///   all instances of the element type. If `N` is the total number of
    ///   possible values, then `DIRECT_BITS` is `ceil(log2(N))`.
    pub fn encode_mc_format<W, F>(
        &self,
        mut writer: W,
        mut to_bits: F,
        min_indirect_bits: usize,
        max_indirect_bits: usize,
        direct_bits: usize,
    ) -> anyhow::Result<()>
    where
        W: Write,
        F: FnMut(T) -> u64,
    {
        debug_assert!(min_indirect_bits <= 4);
        debug_assert!(min_indirect_bits <= max_indirect_bits);
        debug_assert!(max_indirect_bits <= 64);
        debug_assert!(direct_bits <= 64);

        match self {
            Self::Single(val) => {
                // Bits per entry
                0_u8.encode(&mut writer)?;

                // Palette
                VarInt(to_bits(*val) as i32).encode(&mut writer)?;

                // Number of longs
                VarInt(0).encode(&mut writer)?;
            }
            Self::Indirect(ind) => {
                let bits_per_entry = min_indirect_bits.max(log2_ceil(ind.palette.len()));

                // Encode as direct if necessary.
                if bits_per_entry > max_indirect_bits {
                    // Bits per entry
                    (direct_bits as u8).encode(&mut writer)?;

                    // Number of longs in data array.
                    VarInt(compact_u64s_len(LEN, direct_bits) as _).encode(&mut writer)?;
                    // Data array
                    encode_compact_u64s(
                        &mut writer,
                        (0..LEN).map(|i| to_bits(ind.get(i))),
                        direct_bits,
                    )?;
                } else {
                    // Bits per entry
                    (bits_per_entry as u8).encode(&mut writer)?;

                    // Palette len
                    VarInt(ind.palette.len() as i32).encode(&mut writer)?;
                    // Palette
                    for val in &ind.palette {
                        VarInt(to_bits(*val) as i32).encode(&mut writer)?;
                    }

                    // Number of longs in data array.
                    VarInt(compact_u64s_len(LEN, bits_per_entry) as _).encode(&mut writer)?;
                    // Data array
                    encode_compact_u64s(
                        &mut writer,
                        ind.indices
                            .iter()
                            .cloned()
                            .flat_map(|byte| [byte & 0b1111, byte >> 4])
                            .map(u64::from)
                            .take(LEN),
                        bits_per_entry,
                    )?;
                }
            }
            Self::Direct(dir) => {
                // Bits per entry
                (direct_bits as u8).encode(&mut writer)?;

                // Number of longs in data array.
                VarInt(compact_u64s_len(LEN, direct_bits) as _).encode(&mut writer)?;
                // Data array
                encode_compact_u64s(&mut writer, dir.iter().cloned().map(to_bits), direct_bits)?;
            }
        }

        Ok(())
    }
}

impl<T: Copy + Eq + Default, const LEN: usize, const HALF_LEN: usize> Default
    for PalettedContainer<T, LEN, HALF_LEN>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Eq + Default, const LEN: usize, const HALF_LEN: usize> Indirect<T, LEN, HALF_LEN> {
    pub fn get(&self, idx: usize) -> T {
        let palette_idx = self.indices[idx / 2] >> (idx % 2 * 4) & 0b1111;
        self.palette[palette_idx as usize]
    }

    pub fn set(&mut self, idx: usize, val: T) -> Option<T> {
        let palette_idx = if let Some(i) = self.palette.iter().position(|v| *v == val) {
            i
        } else {
            self.palette.try_push(val).ok()?;
            self.palette.len() - 1
        };

        let old_val = self.get(idx);
        let u8 = &mut self.indices[idx / 2];
        let shift = idx % 2 * 4;
        *u8 = (*u8 & !(0b1111 << shift)) | ((palette_idx as u8) << shift);
        Some(old_val)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    fn check<T: Copy + Eq + Default, const LEN: usize, const HALF_LEN: usize>(
        p: &PalettedContainer<T, LEN, HALF_LEN>,
        s: &[T],
    ) -> bool {
        assert_eq!(s.len(), LEN);
        (0..LEN).all(|i| p.get(i) == s[i])
    }

    #[test]
    fn random_assignments() {
        const LEN: usize = 100;
        let range = 0..64;

        let mut rng = rand::thread_rng();

        for _ in 0..20 {
            let mut p = PalettedContainer::<u32, LEN, { LEN / 2 }>::new();

            let init = rng.gen_range(range.clone());

            p.fill(init);
            let mut a = [init; LEN];

            assert!(check(&p, &a));

            let mut rng = rand::thread_rng();

            for _ in 0..LEN * 10 {
                let idx = rng.gen_range(0..LEN);
                let val = rng.gen_range(range.clone());

                assert_eq!(p.get(idx), p.set(idx, val));
                assert_eq!(val, p.get(idx));
                a[idx] = val;

                p.optimize();

                assert!(check(&p, &a));
            }
        }
    }
}
