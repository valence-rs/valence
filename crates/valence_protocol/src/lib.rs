use std::io::{Cursor, Write};
use std::mem::{self, MaybeUninit};

use anyhow::{anyhow, bail, ensure, Context};
use num_traits::{FromPrimitive, ToPrimitive};

use crate::var_int::VarIntReadError;

pub mod packets;
pub mod var_int;
mod id {
    include!(concat!(env!("OUT_DIR"), "/packet_id.rs"));
}

pub trait Packet {
    type Output<'a>;

    fn read_body<'a>(r: &mut impl McRead<'a>) -> anyhow::Result<Self::Output<'a>>;

    fn write_body(&self, w: &mut impl McWrite) -> anyhow::Result<()>;
}

pub trait PacketMeta<State: PacketState, Side: PacketSide> {
    const ID: i32;
}

pub trait PacketState {}

pub trait PacketSide {}

pub struct Handshaking;

impl PacketState for Handshaking {}

pub struct Status;

impl PacketState for Status {}

pub struct Login;

impl PacketState for Login {}

pub struct Configuration;

impl PacketState for Configuration {}

pub struct Serverbound;

impl PacketSide for Serverbound {}
pub struct Clientbound;

impl PacketSide for Clientbound {}

pub trait McWrite {
    /// Write a slice of bytes directly to the output without a length prefix.
    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()>;

    fn write_byte_slice(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let len: i32 = bytes
            .len()
            .try_into()
            .context("byte slice length exceeds `i32::MAX`")?;

        self.write_var_int(len)?;
        self.write_bytes(bytes)
    }

    /// Write a `str`. Errors if the string is longer than
    /// [`DEFAULT_MAX_STRING_LENGTH`] in UTF-16 chars.
    fn write_str(&mut self, str: &str) -> anyhow::Result<()> {
        self.write_str_bounded(str, DEFAULT_MAX_STRING_LENGTH)
    }

    fn write_str_bounded(&mut self, str: &str, max_char_count: u32) -> anyhow::Result<()> {
        let char_count = str.encode_utf16().count();

        ensure!(
            char_count <= max_char_count as usize,
            "utf-16 char count of string ({char_count}) exceeds maximum of {max_char_count}"
        );

        let len: i32 = str
            .len()
            .try_into()
            .context("string length not representable as `i32`")?;

        self.write_var_int(len)?;
        self.write_bytes(str.as_bytes())
    }

    fn write_enum<T: ToPrimitive>(&mut self, val: &T) -> anyhow::Result<()> {
        self.write_var_int(val.to_i32().context("enum not representable as `i32`")?)
    }

    /// Write a variable length integer.
    fn write_var_int(&mut self, int: i32) -> anyhow::Result<()> {
        var_int::write_var_int(int, |b| self.write_bytes(&[b]))
    }

    fn write_var_long(&mut self, int: i64) -> anyhow::Result<()> {
        var_int::write_var_long(int, |b| self.write_bytes(&[b]))
    }

    fn write_u8(&mut self, int: u8) -> anyhow::Result<()> {
        self.write_bytes(&[int])
    }

    fn write_i8(&mut self, int: i8) -> anyhow::Result<()> {
        self.write_bytes(&[int as u8])
    }

    fn write_u16(&mut self, int: u16) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_i16(&mut self, int: i16) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_u32(&mut self, int: u32) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_i32(&mut self, int: i32) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_u64(&mut self, int: u64) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_i64(&mut self, int: i64) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_u128(&mut self, int: u128) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_i128(&mut self, int: i128) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_f32(&mut self, int: f32) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }

    fn write_f64(&mut self, int: f64) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }
}

/// A source of contiguous bytes for Minecraft packets to read from. The
/// interface is similar to Minecraft's `PacketByteBuf` class.
pub trait McRead<'a> {
    fn read_bytes(&mut self, count: usize) -> anyhow::Result<&'a [u8]>;

    fn read_bytes_const<const N: usize>(&mut self) -> anyhow::Result<&'a [u8; N]> {
        Ok(self.read_bytes(N)?.try_into().expect("invalid slice len"))
    }

    /// Read an array of values from the input. The array is not length
    /// prefixed.
    fn read_array<T, const N: usize, F>(&mut self, mut f: F) -> anyhow::Result<[T; N]>
    where
        F: FnMut(&mut Self) -> anyhow::Result<T>,
    {
        // TODO: use std::array::try_from_fn when stabilized.

        struct Guard<T, const N: usize> {
            array: [MaybeUninit<T>; N],
            initialized: usize,
        }

        impl<T, const N: usize> Drop for Guard<T, N> {
            fn drop(&mut self) {
                unsafe { std::ptr::drop_in_place(self.array.as_mut_slice()) };
            }
        }

        // This is what `ArrayVec::new` does.
        let initial_array: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        let mut guard = Guard {
            array: initial_array,
            initialized: 0,
        };

        while guard.initialized < N {
            *unsafe { guard.array.get_unchecked_mut(guard.initialized) } =
                MaybeUninit::new(f(self)?);

            guard.initialized += 1;
        }

        let res: [T; N] = unsafe { std::mem::transmute_copy(&guard.array) };

        std::mem::forget(guard);

        Ok(res)
    }

    /// Read a varint-prefixed slice of bytes. There is no length limit.
    fn read_byte_slice(&mut self) -> anyhow::Result<&'a [u8]> {
        let len: u32 = self
            .read_var_int()?
            .try_into()
            .context("negative byte slice length")?;

        self.read_bytes(len as usize)
    }

    fn read_str(&mut self) -> anyhow::Result<&'a str> {
        self.read_str_bounded(DEFAULT_MAX_STRING_LENGTH)
    }

    fn read_str_bounded(&mut self, max_char_count: u32) -> anyhow::Result<&'a str> {
        let len: u32 = self
            .read_var_int()?
            .try_into()
            .context("negative string length")?;

        let max_bytes_per_char = 4;
        let max_byte_len = max_char_count * max_bytes_per_char;

        ensure!(
            len <= max_byte_len,
            "string byte length of {len} exceeds maximum of {max_byte_len}"
        );

        let str = std::str::from_utf8(self.read_bytes(len as usize)?)?;

        let utf16_char_count = str.encode_utf16().count();

        ensure!(
            utf16_char_count <= max_char_count as usize,
            "UTF-16 string length of {utf16_char_count} exceeds maximum of {max_char_count}"
        );

        Ok(str)
    }

    fn read_enum<T: FromPrimitive>(&mut self) -> anyhow::Result<T> {
        let discriminant = self.read_var_int()?;
        T::from_i32(discriminant).context("failed to decode enum")
    }

    fn read_option<T, F>(&mut self, f: F) -> anyhow::Result<Option<T>>
    where
        F: FnOnce(&mut Self) -> anyhow::Result<T>,
    {
        self.read_bool()?.then(|| f(self)).transpose()
    }

    fn read_collection<C, T, CF, F>(&mut self, cf: CF, mut f: F) -> anyhow::Result<C>
    where
        C: Extend<T>,
        CF: FnOnce(usize) -> C,
        F: FnMut(&mut Self) -> anyhow::Result<T>,
    {
        let len = self.read_var_int()?;
        let len: usize = len.try_into().context("invalid collection length")?;

        let mut collection = cf(cautious_capacity::<T>(len));

        for _ in 0..len {
            collection.extend([f(self)?]);
        }

        Ok(collection)
    }

    fn read_vec<T, F>(&mut self, f: F) -> anyhow::Result<Vec<T>>
    where
        F: FnMut(&mut Self) -> anyhow::Result<T>,
    {
        self.read_collection(Vec::with_capacity, f)
    }

    fn read_var_int(&mut self) -> anyhow::Result<i32> {
        match var_int::read_var_int(|| self.read_u8()) {
            Ok(int) => Ok(int),
            Err(VarIntReadError::ReadError(e)) => Err(e),
            Err(e @ VarIntReadError::TooLarge) => bail!(e),
        }
    }

    fn read_var_long(&mut self) -> anyhow::Result<i64> {
        match var_int::read_var_long(|| self.read_u8()) {
            Ok(int) => Ok(int),
            Err(VarIntReadError::ReadError(e)) => Err(e),
            Err(e @ VarIntReadError::TooLarge) => Err(anyhow!(e)),
        }
    }

    fn read_bool(&mut self) -> anyhow::Result<bool> {
        let byte = self.read_u8()?;
        ensure!(byte <= 1, "boolean byte is not zero or one (got {byte})");
        Ok(byte == 1)
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        let &[byte] = self.read_bytes_const()?;
        Ok(byte)
    }

    fn read_i8(&mut self) -> anyhow::Result<i8> {
        let &[byte] = self.read_bytes_const()?;
        Ok(byte as i8)
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        Ok(u16::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_i16(&mut self) -> anyhow::Result<i16> {
        Ok(i16::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_u32(&mut self) -> anyhow::Result<u32> {
        Ok(u32::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_i32(&mut self) -> anyhow::Result<i32> {
        Ok(i32::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_u64(&mut self) -> anyhow::Result<u64> {
        Ok(u64::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_i64(&mut self) -> anyhow::Result<i64> {
        Ok(i64::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_u128(&mut self) -> anyhow::Result<u128> {
        Ok(u128::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_i128(&mut self) -> anyhow::Result<i128> {
        Ok(i128::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_f32(&mut self) -> anyhow::Result<f32> {
        Ok(f32::from_be_bytes(*self.read_bytes_const()?))
    }

    fn read_f64(&mut self) -> anyhow::Result<f64> {
        Ok(f64::from_be_bytes(*self.read_bytes_const()?))
    }
}

pub const DEFAULT_MAX_STRING_LENGTH: u32 = 32767;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct McWriter<W>(pub W);

impl<W: Write> McWrite for McWriter<W> {
    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.0.write_all(bytes).map_err(|e| e.into())
    }
}

impl<W> From<W> for McWriter<W> {
    fn from(value: W) -> Self {
        Self(value)
    }
}

impl<'a> McRead<'a> for Cursor<&'a [u8]> {
    fn read_bytes(&mut self, count: usize) -> anyhow::Result<&'a [u8]> {
        let remaining_slice =
            &self.get_ref()[self.position().min(self.get_ref().len() as u64) as usize..];

        ensure!(
            remaining_slice.len() <= count,
            "attempt to read {count} bytes, but cursor has {} bytes remaining",
            remaining_slice.len()
        );

        self.set_position(self.position() + count as u64);

        Ok(&remaining_slice[..count])
    }
}

impl<'a> McRead<'a> for &'a [u8] {
    fn read_bytes(&mut self, count: usize) -> anyhow::Result<&'a [u8]> {
        ensure!(
            count <= self.len(),
            "attempt to read {count} bytes, but slice has length of {}",
            self.len()
        );

        let (l, r) = self.split_at(count);
        *self = r;
        Ok(l)
    }
}

/// Prevents preallocating too much memory in case we get a malicious or invalid
/// sequence length.
fn cautious_capacity<Element>(size_hint: usize) -> usize {
    const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

    if mem::size_of::<Element>() == 0 {
        0
    } else {
        size_hint.min(MAX_PREALLOC_BYTES / mem::size_of::<Element>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_array() {
        let mut buf: McWriter<Vec<u8>> = McWriter(vec![]);

        buf.write_str("abc").unwrap();
        buf.write_str("123").unwrap();

        let mut reader = buf.0.as_slice();

        let res: [String; 2] = reader
            .read_array(|r| r.read_str().map(String::from))
            .unwrap();

        let _: [&str; 0] = reader.read_array(|r| r.read_str()).unwrap();

        assert_eq!(&res, &["abc", "123"]);
    }

    #[test]
    #[should_panic = "expect the unexpected"]
    fn read_array_panic() {
        let mut buf: McWriter<Vec<u8>> = McWriter(vec![]);

        buf.write_str("abc").unwrap();
        buf.write_str("123").unwrap();

        let mut reader = buf.0.as_slice();

        let _: [String; 2] = reader
            .read_array(|r| {
                let s = r.read_str()?;
                assert_eq!(s, "abc", "expect the unexpected");
                Ok(s.into())
            })
            .unwrap();
    }
}
