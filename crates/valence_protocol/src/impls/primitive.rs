use std::io::Write;
use std::slice;

use anyhow::ensure;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::{Decode, Encode};

impl Encode for bool {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_u8(*self as u8)?)
    }

    fn encode_slice(slice: &[bool], mut w: impl Write) -> anyhow::Result<()> {
        // SAFETY: Bools have the same layout as u8.
        let bytes = unsafe { slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len()) };
        // Bools are guaranteed to have the correct bit pattern.
        Ok(w.write_all(bytes)?)
    }
}

impl Decode<'_> for bool {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let n = r.read_u8()?;
        ensure!(n <= 1, "decoded boolean byte is not 0 or 1 (got {n})");
        Ok(n == 1)
    }
}

impl Encode for u8 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_u8(*self)?)
    }

    fn encode_slice(slice: &[u8], mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_all(slice)?)
    }
}

impl Decode<'_> for u8 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_u8()?)
    }
}

impl Encode for i8 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_i8(*self)?)
    }

    fn encode_slice(slice: &[i8], mut w: impl Write) -> anyhow::Result<()> {
        // SAFETY: i8 has the same layout as u8.
        let bytes = unsafe { slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len()) };
        Ok(w.write_all(bytes)?)
    }
}

impl Decode<'_> for i8 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_i8()?)
    }
}

impl Encode for u16 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_u16::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u16 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_u16::<BigEndian>()?)
    }
}

impl Encode for i16 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_i16::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for i16 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_i16::<BigEndian>()?)
    }
}

impl Encode for u32 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_u32::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u32 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_u32::<BigEndian>()?)
    }
}

impl Encode for i32 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_i32::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for i32 {
    fn decode(r: &mut &'_ [u8]) -> anyhow::Result<Self> {
        Ok(r.read_i32::<BigEndian>()?)
    }
}

impl Encode for u64 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_u64::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u64 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_u64::<BigEndian>()?)
    }
}

impl Encode for i64 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_i64::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for i64 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_i64::<BigEndian>()?)
    }
}

impl Encode for u128 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_u128::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u128 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(r.read_u128::<BigEndian>()?)
    }
}

impl Encode for i128 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_i128::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for i128 {
    fn decode(r: &mut &'_ [u8]) -> anyhow::Result<Self> {
        Ok(r.read_i128::<BigEndian>()?)
    }
}

impl Encode for f32 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        ensure!(
            self.is_finite(),
            "attempt to encode non-finite f32 ({})",
            self
        );
        Ok(w.write_f32::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for f32 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let f = r.read_f32::<BigEndian>()?;
        ensure!(f.is_finite(), "attempt to decode non-finite f32 ({f})");
        Ok(f)
    }
}

impl Encode for f64 {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        ensure!(
            self.is_finite(),
            "attempt to encode non-finite f64 ({})",
            self
        );
        Ok(w.write_f64::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for f64 {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let f = r.read_f64::<BigEndian>()?;
        ensure!(f.is_finite(), "attempt to decode non-finite f64 ({f})");
        Ok(f)
    }
}
