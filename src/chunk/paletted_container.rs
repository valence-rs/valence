use std::array;
use std::io::Write;

use arrayvec::ArrayVec;

use crate::chunk::{compact_u64s_len, encode_compact_u64s};
use crate::protocol::{Encode, VarInt};
use crate::util::log2_ceil;

// TODO: https://github.com/rust-lang/rust/issues/60551

/// `HALF_LEN` must be equal to `ceil(LEN / 2)`.
#[derive(Clone, Debug)]
pub struct PalettedContainer<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize> {
    inner: Inner<T, LEN, HALF_LEN>,
}

pub trait PalettedContainerElement: Copy + Eq + Default {
    /// The minimum number of bits required to represent all instances of the
    /// element type. If `N` is the total number of possible values, then
    /// `DIRECT_BITS` is `ceil(log2(N))`.
    const DIRECT_BITS: usize;
    /// The maximum number of bits per element allowed in the indirect
    /// representation while encoding. Any higher than this will force
    /// conversion to the direct representation while encoding.
    const MAX_INDIRECT_BITS: usize;
    /// The minimum number of bits used to represent the element type in the
    /// indirect representation while encoding. If the bits per index is lower,
    /// it will be rounded up to this.
    const MIN_INDIRECT_BITS: usize;
    /// Converts the element type to bits. The output must be less than two to
    /// the power of `DIRECT_BITS`.
    fn to_bits(self) -> u64;
}

#[derive(Clone, Debug)]
enum Inner<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize> {
    Single(T),
    Indirect(Box<Indirect<T, LEN, HALF_LEN>>),
    Direct(Box<[T; LEN]>),
}

#[derive(Clone, Debug)]
struct Indirect<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize> {
    /// Each element is a unique instance of `T`.
    palette: ArrayVec<T, 16>,
    /// Each half-byte is an index into `palette`.
    indices: [u8; HALF_LEN],
}

impl<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize>
    PalettedContainer<T, LEN, HALF_LEN>
{
    pub fn new() -> Self {
        assert_eq!(num::Integer::div_ceil(&LEN, &2), HALF_LEN);
        assert_ne!(LEN, 0);

        Self {
            inner: Inner::Single(T::default()),
        }
    }

    pub fn fill(&mut self, val: T) {
        self.inner = Inner::Single(val);
    }

    pub fn get(&self, idx: usize) -> T {
        self.check_oob(idx);

        match &self.inner {
            Inner::Single(elem) => *elem,
            Inner::Indirect(ind) => ind.get(idx),
            Inner::Direct(elems) => elems[idx],
        }
    }

    pub fn single(&self) -> Option<T> {
        match &self.inner {
            Inner::Single(val) => Some(*val),
            Inner::Indirect(_) => None,
            Inner::Direct(_) => None,
        }
    }

    pub fn set(&mut self, idx: usize, val: T) -> T {
        self.check_oob(idx);

        match &mut self.inner {
            Inner::Single(old_val) => {
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
                    self.inner = Inner::Indirect(ind);
                    old
                }
            }
            Inner::Indirect(ind) => {
                if let Some(old) = ind.set(idx, val) {
                    old
                } else {
                    // Upgrade to direct.
                    self.inner = Inner::Direct(Box::new(array::from_fn(|i| ind.get(i))));
                    self.set(idx, val)
                }
            }
            Inner::Direct(vals) => {
                let old = vals[idx];
                vals[idx] = val;
                old
            }
        }
    }

    pub fn optimize(&mut self) {
        match &mut self.inner {
            Inner::Single(_) => {}
            Inner::Indirect(ind) => {
                let mut new_ind = Indirect {
                    palette: ArrayVec::new(),
                    indices: [0; HALF_LEN],
                };

                for i in 0..LEN {
                    new_ind.set(i, ind.get(i));
                }

                if new_ind.palette.len() == 1 {
                    self.inner = Inner::Single(new_ind.palette[0]);
                } else {
                    **ind = new_ind;
                }
            }
            Inner::Direct(dir) => {
                let mut ind = Indirect {
                    palette: ArrayVec::new(),
                    indices: [0; HALF_LEN],
                };

                for (i, val) in dir.iter().cloned().enumerate() {
                    if ind.set(i, val).is_none() {
                        return;
                    }
                }

                if ind.palette.len() == 1 {
                    self.inner = Inner::Single(ind.palette[0]);
                } else {
                    self.inner = Inner::Indirect(Box::new(ind));
                }
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
}

impl<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize> Default
    for PalettedContainer<T, LEN, HALF_LEN>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize>
    Indirect<T, LEN, HALF_LEN>
{
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

/// Encodes the paletted container in the format that Minecraft expects.
impl<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize> Encode
    for PalettedContainer<T, LEN, HALF_LEN>
{
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        assert!(T::DIRECT_BITS <= u8::MAX as _);
        assert!(T::MAX_INDIRECT_BITS <= u8::MAX as _);
        assert!(T::MIN_INDIRECT_BITS <= T::MAX_INDIRECT_BITS);
        assert!(T::MIN_INDIRECT_BITS <= 4);

        match &self.inner {
            Inner::Single(val) => {
                // Bits per entry
                0_u8.encode(w)?;

                // Palette
                VarInt(val.to_bits() as i32).encode(w)?;

                // Number of longs
                VarInt(0).encode(w)?;
            }
            Inner::Indirect(ind) => {
                let bits_per_entry = T::MIN_INDIRECT_BITS.max(log2_ceil(ind.palette.len()));

                // TODO: if bits_per_entry > MAX_INDIRECT_BITS, encode as direct.
                debug_assert!(bits_per_entry <= T::MAX_INDIRECT_BITS);

                // Bits per entry
                (bits_per_entry as u8).encode(w)?;

                // Palette len
                VarInt(ind.palette.len() as i32).encode(w)?;
                // Palette
                for val in &ind.palette {
                    VarInt(val.to_bits() as i32).encode(w)?;
                }

                // Number of longs in data array.
                VarInt(compact_u64s_len(LEN, bits_per_entry) as _).encode(w)?;
                // Data array
                encode_compact_u64s(
                    w,
                    ind.indices
                        .iter()
                        .cloned()
                        .flat_map(|byte| [byte & 0b1111, byte >> 4])
                        .map(u64::from)
                        .take(LEN),
                    bits_per_entry,
                )?;
            }
            Inner::Direct(dir) => {
                // Bits per entry
                (T::DIRECT_BITS as u8).encode(w)?;

                // Number of longs in data array.
                VarInt(compact_u64s_len(LEN, T::DIRECT_BITS) as _).encode(w)?;
                // Data array
                encode_compact_u64s(w, dir.iter().map(|v| v.to_bits()), T::DIRECT_BITS)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    fn check<T: PalettedContainerElement, const LEN: usize, const HALF_LEN: usize>(
        p: &PalettedContainer<T, LEN, HALF_LEN>,
        s: &[T],
    ) -> bool {
        assert_eq!(s.len(), LEN);
        (0..LEN).all(|i| p.get(i) == s[i])
    }

    impl PalettedContainerElement for u32 {
        const DIRECT_BITS: usize = 0;
        const MAX_INDIRECT_BITS: usize = 0;
        const MIN_INDIRECT_BITS: usize = 0;

        fn to_bits(self) -> u64 {
            self.into()
        }
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
