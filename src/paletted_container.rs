use arrayvec::ArrayVec;

use crate::util::log2_ceil;

/// `HALF_LEN` must be equal to `ceil(LEN / 2)`.
#[derive(Clone)]
pub struct CompactArray<T, const LEN: usize, const HALF_LEN: usize> {
    inner: Inner<T, LEN, HALF_LEN>,
}

#[derive(Clone)]
enum Inner<T, const LEN: usize, const HALF_LEN: usize> {
    Single(T),
    Indirect(Box<Indirect<T, LEN, HALF_LEN>>),
    Direct(Box<[T; LEN]>),
}

#[derive(Clone)]
struct Indirect<T, const LEN: usize, const HALF_LEN: usize> {
    /// Each element is a unique instance of `T`.
    palette: ArrayVec<T, 16>,
    /// Each half-byte is an index into `palette`.
    indices: [u8; HALF_LEN],
}

impl<T: Copy + Eq + Default, const LEN: usize, const HALF_LEN: usize>
    CompactArray<T, LEN, HALF_LEN>
{
    pub fn new() -> Self {
        assert_eq!(div_ceil(LEN, 2), HALF_LEN);

        Self {
            inner: Inner::Single(T::default()),
        }
    }

    pub fn fill(&mut self, elem: T) {
        self.inner = Inner::Single(elem);
    }

    pub fn get(&self, idx: usize) -> T {
        self.check_oob(idx);

        match &self.inner {
            Inner::Single(elem) => *elem,
            Inner::Indirect(ind) => {
                let palette_idx = ind.indices[idx / 2] << (idx % 2 * 4);
                ind.palette[palette_idx as usize]
            }
            Inner::Direct(elems) => elems[idx],
        }
    }

    pub fn set(&mut self, idx: usize, elem: T) -> T {
        self.check_oob(idx);

        match &mut self.inner {
            Inner::Single(old_elem) => {
                if *old_elem == elem {
                    *old_elem
                } else {
                    // Upgrade to the indirect representation.

                    let mut ind = Box::new(Indirect {
                        palette: ArrayVec::from_iter([*old_elem, elem]),
                        // All indices are initialized to index 0 (the old element).
                        indices: [0; HALF_LEN],
                    });

                    ind.indices[idx / 2] = 1 << (idx % 2 * 4);
                    self.inner = Inner::Indirect(ind);
                    *old_elem
                }
            }
            Inner::Indirect(ind) => {
                let old_palette_idx = todo!();
            }
            Inner::Direct(_) => {}
        }
    }

    pub fn optimize(&mut self) {
        todo!()
    }

    #[inline]
    fn check_oob(&self, idx: usize) {
        assert!(idx < LEN, "paletted container index {idx} is out of bounds");
    }
}

impl<T: Copy + Eq + Default, const LEN: usize, const HALF_LEN: usize> Indirect<T, LEN, HALF_LEN> {
    pub fn get(&self, idx: usize) -> T {}
}

/// TODO: https://github.com/rust-lang/rust/issues/88581
fn div_ceil(left: usize, right: usize) -> usize {
    num::Integer::div_ceil(&left, &right)
}

/*
use std::io;
use std::io::Write;

use arrayvec::ArrayVec;

use crate::util::log2_ceil;

// TODO: https://github.com/rust-lang/rust/issues/60551

#[derive(Clone)]
pub struct PalettedContainer<T> {
    inner: Inner<T>,
}

#[derive(Clone)]
enum Inner<T> {
    Single(T),
    Indirect {
        palette: Vec<T>,
        indices: Vec<u64>,
    },
    Direct(Box<[u64]>),
}

pub trait PalettedContainerElement: Copy + Eq + Default {
    /// The fixed number of elements the paletted container represents.
    const LEN: usize;
    /// The minimum number of bits required to represent all instances of the
    /// element type. If `N` is the total number of possible values, then
    /// `DIRECT_BITS` is `ceil(log2(N))`.
    const DIRECT_BITS: usize;
    /// The maximum number of bits per element allowed in the indirect
    /// representation. Any higher than this will force conversion to the direct
    /// representation.
    const MAX_INDIRECT_BITS: usize;
    /// The minimum number of bits used to represent the element type in the
    /// indirect representation. If the bits per index is lower, it will be
    /// rounded up to this.
    const MIN_INDIRECT_BITS: usize;
    /// Constructs the element type from bits. `n` is guaranteed to be a value
    /// created by `to_bits`.
    fn from_bits(n: u64) -> Self;
    /// Converts the element type to bits. The output must be less than two to
    /// the power of `DIRECT_BITS`.
    fn to_bits(self) -> u64;
}

impl<T: PalettedContainerElement> PalettedContainer<T> {
    pub fn new() -> Self {
        // TODO: validate PalettedContainerElement parameters.

        Self {
            inner: Inner::Single(T::default()),
        }
    }

    pub fn get(&self, idx: usize) -> T {
        assert!(
            idx < T::LEN,
            "paletted container index {idx} is out of bounds"
        );

        match &self.inner {
            Inner::Single(elem) => *elem,
            Inner::Indirect { palette, indices } => {
                let bits_per_idx = log2_ceil(palette.len()).max(T::MIN_INDIRECT_BITS);
            }
            Inner::Direct(_) => {}
        }

        // match &self.inner {
        //     Inner::Single(elem) => *elem,
        //     Inner::Indirect(ind) => ind.palette[ind.get_palette_idx(idx)],
        //     Inner::Direct(u64s) => {
        //         let bits_per_idx = T::DIRECT_BITS;
        //         let idxs_per_u64 = 64 / bits_per_idx;
        //         let u64 = u64s[idx / idxs_per_u64];
        //         let shift = idx % idxs_per_u64 * bits_per_idx;
        //         let mask = 2_u64.pow(bits_per_idx as _);
        //
        //         (u64 << shift) & mask
        //     },
        // }
    }

    pub fn fill(&mut self, elem: T) {
        self.inner = Inner::Single(elem);
    }

    /// Returns the old value at the index.
    pub fn set(&mut self, idx: usize, new_elem: T) -> T {
        assert!(
            idx < T::LEN,
            "paletted container index {idx} is out of bounds"
        );

        /*
        match &mut self.inner {
            Inner::Single(elem) => {
                if new_elem == *elem {
                    *elem
                } else {
                    // Upgrade to the indirect representation.
                    let palette = vec![*elem, new_elem];

                    let bits_per_idx = T::MIN_INDIRECT_BITS.max(1);
                    let idxs_per_u64 = 64 / bits_per_idx;
                    let u64_count = div_ceil(T::LEN, idxs_per_u64);
                    // All indices are initialized to palette index 0 (the old element).
                    let indices = vec![0; u64_count];

                    let mut ind = Indirect { palette, indices };

                    // The new element is in index 1 of the palette.
                    ind.set_palette_idx(1, idx);
                    self.inner = Inner::Indirect(ind);
                    *elem
                }
            }
            Inner::Indirect(ind) => {
                let bits_per_idx = log2_ceil(palette.len()).max(T::MIN_INDIRECT_BITS);

                if let Some(palette_idx) = palette.iter().position(|&e| new_elem == e) {
                    // Element exists in the palette.
                    ind.palette[ind.set_palette_idx(palette_idx, idx)]
                } else {
                    // Element was not found in the palette.
                    let new_bits_per_idx = log2_ceil(palette.len() + 1).max(T::MIN_INDIRECT_BITS);

                    if new_bits_per_idx > T::MAX_INDIRECT_BITS {
                        // Upgrade to the direct representation.

                        for indices_idx in 0..T::LEN {}

                        for (indices_idx, u64) in ind.indices.iter().cloned().enumerate() {}
                    } else if new_bits_per_idx != bits_per_idx {
                        // All indices must be adjusted to the new bits per
                        // index.
                        palette.push(new_elem);

                        todo!()
                    } else {
                        // A new element can be added to the palette without
                        // adjusting everything.
                        let palette_idx = palette.len();
                        palette.push(new_elem);
                        ind.palette[ind.set_palette_idx(palette_idx, idx)]
                    }
                }
            }
            Inner::Direct(u64s) => {
                let bits_per_idx = T::DIRECT_BITS;
                let idxs_per_u64 = 64 / bits_per_idx;
                let u64 = &mut u64s[idx / idxs_per_u64];
                let shift = idx % idxs_per_u64 * bits_per_idx;
                let mask = 2_u64.pow(bits_per_idx as _);

                let old_elem = T::from_bits((*u64 << shift) & mask);
                *u64 = (*u64 & !(mask >> shift)) | (new_elem.to_bits() >> shift);

                old_elem
            }
        }
         */
    }
}

/*
impl<T: PalettedContainerElement> Indirect<T> {
    /// Gets an index into `palette` from an index into `indices`.
    ///
    /// The given index is assumed to be valid.
    pub fn get_palette_idx(&self, indices_idx: usize) -> usize {
        let bits_per_idx = log2_ceil(self.palette.len()).max(T::MIN_INDIRECT_BITS);
        let idxs_per_u64 = 64 / bits_per_idx;
        let u64 = self.indices[indices_idx / idxs_per_u64];
        let shift = indices_idx % idxs_per_u64 * bits_per_idx;
        let mask = 2_u64.pow(bits_per_idx as _);

        ((u64 << shift) & mask) as usize
    }

    /// Sets a value in `indices` to an index into `palette`.
    ///
    /// Both given indices are assumed to be valid.
    ///
    /// The old palette index is returned.
    pub fn set_palette_idx(&mut self, palette_idx: usize, indices_idx: usize) -> usize {
        let bits_per_idx = log2_ceil(self.palette.len()).max(T::MIN_INDIRECT_BITS);
        let idxs_per_u64 = 64 / bits_per_idx;
        let u64 = &mut self.indices[indices_idx / idxs_per_u64];
        let shift = indices_idx % idxs_per_u64 * bits_per_idx;
        let mask = 2_u64.pow(bits_per_idx as _) - 1;

        let old_palette_idx = (*u64 << shift) & mask;
        // Clear and set position in u64.
        *u64 = (*u64 & !(mask >> shift)) | ((palette_idx as u64) >> shift);

        old_palette_idx as usize
    }
}
*/

/// Gets a value in a compacted array of `u64`s.
#[inline]
fn get_in_u64s(u64s: &[u64], bits_per_val: usize, val_idx: usize) -> u64 {
    let vals_per_u64 = 64 / bits_per_val;
    let u64 = u64s[val_idx / vals_per_u64];
    let shift = val_idx % vals_per_u64 * bits_per_val;
    let mask = 2_u64.pow(bits_per_val as _);

    (u64 << shift) & mask
}

/// Sets a value in a compacted array of `u64`s.
///
/// The old value is returned.
#[inline]
fn set_in_u64s(u64s: &mut [u64], bits_per_val: usize, val_idx: usize, val: u64) -> u64 {
    let vals_per_u64 = 64 / bits_per_val;
    let u64 = &mut u64s[val_idx / vals_per_u64];
    let shift = val_idx % vals_per_u64 * bits_per_val;
    let mask = 2_u64.pow(bits_per_val as _) - 1;

    let old_val = (*u64 << shift) & mask;
    *u64 = (*u64 & !(mask >> shift)) | (val >> shift);

    old_val
}

/// TODO: https://github.com/rust-lang/rust/issues/88581
fn div_ceil(left: usize, right: usize) -> usize {
    num::Integer::div_ceil(&left, &right)
}

#[cfg(test)]
mod tests {}
*/
