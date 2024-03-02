use std::io::Write;

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
    /// Write a slice of bytes directly to the output without any length prefix.
    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()>;

    /// Write a variable length integer.
    fn write_var_int(&mut self, int: i32) -> anyhow::Result<()> {
        var_int::write_var_int(int, |b| self.write_bytes(&[b]))
    }

    fn write_var_long(&mut self, int: i64) -> anyhow::Result<()> {
        var_int::write_var_long(int, |b| self.write_bytes(&[b]))
    }

    /// Write a `str`. Errors if the string is longer than 32767 in utf-16
    /// chars.
    fn write_str(&mut self, str: &str) -> anyhow::Result<()> {
        self.write_str_bounded(str, 32767)
    }

    fn write_str_bounded(&mut self, str: &str, max_len: u32) -> anyhow::Result<()> {
        let char_count = str.encode_utf16().count();

        ensure!(
            char_count <= max_len as usize,
            "utf-16 char count of string ({char_count}) exceeds maximum of {max_len}"
        );

        let len: i32 = str
            .len()
            .try_into()
            .context("string length not representable as `i32`")?;

        self.write_var_int(len)?;
        self.write_bytes(str.as_bytes())
    }

    fn write_enum<T: ToPrimitive>(&mut self, t: &T) -> anyhow::Result<()> {
        self.write_var_int(t.to_i32().context("enum not representable as `i32`")?)
    }

    fn write_i8(&mut self, int: i8) -> anyhow::Result<()> {
        self.write_bytes(&[int as u8])
    }

    fn write_i64(&mut self, int: i64) -> anyhow::Result<()> {
        self.write_bytes(&int.to_be_bytes())
    }
}

pub trait McRead<'a> {
    fn read_bytes(&mut self, count: usize) -> anyhow::Result<&'a [u8]>;

    fn read_bytes_const<const N: usize>(&mut self) -> anyhow::Result<&'a [u8; N]> {
        Ok(self.read_bytes(N)?.try_into().expect("invalid slice len"))
    }

    fn read_str(&mut self) -> anyhow::Result<&'a str> {
        self.read_str_bounded(32767)
    }

    fn read_str_bounded(&mut self, max_len: u32) -> anyhow::Result<&'a str> {
        let len = self.read_var_int()?;
        let len: u32 = len.try_into().context("negative string length")?;

        let max_bytes_per_char = 2;
        let max_byte_len = max_len * max_bytes_per_char;

        ensure!(
            len <= max_byte_len,
            "string byte length of {len} exceeds maximum of {max_byte_len}"
        );

        let str = std::str::from_utf8(self.read_bytes(len as usize)?)?;

        let utf16_len = str.encode_utf16().count();

        ensure!(
            utf16_len <= max_len as usize,
            "utf-16 string length of {utf16_len} exceeds maximum of {max_len}"
        );

        Ok(str)
    }

    fn read_enum<T: FromPrimitive>(&mut self) -> anyhow::Result<T> {
        let discriminant = self.read_var_int()?;
        T::from_i32(discriminant).context("failed to decode enum")
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

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        let &[byte] = self.read_bytes_const::<1>()?;
        Ok(byte)
    }

    fn read_i8(&mut self) -> anyhow::Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    fn read_u64(&mut self) -> anyhow::Result<u64> {
        Ok(u64::from_be_bytes(*self.read_bytes_const::<8>()?))
    }

    fn read_i64(&mut self) -> anyhow::Result<i64> {
        Ok(self.read_u64()? as i64)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct McWriter<W>(pub W);

impl<W: Write> McWrite for McWriter<W> {
    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.0.write_all(bytes).map_err(|e| e.into())
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
