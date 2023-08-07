use std::io::Write;

use anyhow::bail;
use byteorder::ReadBytesExt;

use crate::{Decode, Encode};

/// An `i64` encoded with variable length.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(transparent)]
pub struct VarLong(pub i64);

impl VarLong {
    /// The maximum number of bytes a `VarLong` can occupy when read from and
    /// written to the Minecraft protocol.
    pub const MAX_SIZE: usize = 10;

    /// Returns the exact number of bytes this varlong will write when
    /// [`Encode::encode`] is called, assuming no error occurs.
    pub fn written_size(self) -> usize {
        match self.0 {
            0 => 1,
            n => (63 - n.leading_zeros() as usize) / 7 + 1,
        }
    }
}

impl Encode for VarLong {
    // Adapted from VarInt-Simd encode
    // https://github.com/as-com/varint-simd/blob/0f468783da8e181929b01b9c6e9f741c1fe09825/src/encode/mod.rs#L71
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        not(target_os = "macos")
    ))]
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        // Break the number into 7-bit parts and spread them out into a vector
        let mut res = [0u64; 2];
        {
            let x = self.0 as u64;

            res[0] = unsafe { _pdep_u64(x, 0x7f7f7f7f7f7f7f7f) };
            res[1] = unsafe { _pdep_u64(x >> 56, 0x000000000000017f) };
        }
        let stage1: __m128i = unsafe { std::mem::transmute(res) };

        // Create a mask for where there exist values
        // This signed comparison works because all MSBs should be cleared at this point
        // Also handle the special case when num == 0
        let minimum =
            unsafe { _mm_set_epi8(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xffu8 as i8) };
        let exists = unsafe { _mm_or_si128(_mm_cmpgt_epi8(stage1, _mm_setzero_si128()), minimum) };
        let bits = unsafe { _mm_movemask_epi8(exists) };

        // Count the number of bytes used
        let bytes_needed = 32 - bits.leading_zeros() as u8; // lzcnt on supported CPUs

        // Fill that many bytes into a vector
        let ascend = unsafe { _mm_setr_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15) };
        let mask = unsafe { _mm_cmplt_epi8(ascend, _mm_set1_epi8(bytes_needed as i8)) };

        // Shift it down 1 byte so the last MSB is the only one set, and make sure only
        // the MSB is set
        let shift = unsafe { _mm_bsrli_si128(mask, 1) };
        let msbmask = unsafe { _mm_and_si128(shift, _mm_set1_epi8(128u8 as i8)) };

        // Merge the MSB bits into the vector
        let merged = unsafe { _mm_or_si128(stage1, msbmask) };
        let bytes = unsafe { std::mem::transmute::<__m128i, [u8; 16]>(merged) };

        w.write_all(unsafe { bytes.get_unchecked(..bytes_needed as usize) })?;

        Ok(())
    }

    #[cfg(any(
        not(any(target_arch = "x86", target_arch = "x86_64")),
        target_os = "macos"
    ))]
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        use byteorder::WriteBytesExt;

        let mut val = self.0 as u64;
        loop {
            if val & 0b1111111111111111111111111111111111111111111111111111111110000000 == 0 {
                w.write_u8(val as u8)?;
                return Ok(());
            }
            w.write_u8(val as u8 & 0b01111111 | 0b10000000)?;
            val >>= 7;
        }
    }
}

impl Decode<'_> for VarLong {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = r.read_u8()?;
            val |= (byte as i64 & 0b01111111) << (i * 7);
            if byte & 0b10000000 == 0 {
                return Ok(VarLong(val));
            }
        }
        bail!("VarInt is too large")
    }
}

impl From<i64> for VarLong {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use super::*;

    #[test]
    fn encode_decode() {
        let mut rng = thread_rng();
        let mut buf = vec![];

        for n in (0..1_000_000)
            .map(|_| rng.gen())
            .chain([0, i64::MIN, i64::MAX])
        {
            VarLong(n).encode(&mut buf).unwrap();

            let mut slice = buf.as_slice();
            assert!(slice.len() <= VarLong::MAX_SIZE);

            assert_eq!(n, VarLong::decode(&mut slice).unwrap().0);
            assert!(slice.is_empty());
            buf.clear();
        }
    }
}
