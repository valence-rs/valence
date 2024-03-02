use thiserror::Error;

#[inline]
pub fn read_var_int<F, E>(mut read_byte: F) -> Result<i32, VarIntReadError<E>>
where
    F: FnMut() -> Result<u8, E>,
{
    let mut val = 0;
    for i in 0..VAR_INT_MAX_LEN {
        let byte = read_byte()?;

        val |= (byte as i32 & 0x7F) << (i * 7);

        if byte & 0x80 == 0 {
            return Ok(val);
        }
    }

    Err(VarIntReadError::TooLarge)
}

#[inline]
pub fn write_var_int<F, E>(int: i32, mut write_byte: F) -> Result<(), E>
where
    F: FnMut(u8) -> Result<(), E>,
{
    let mut int = int as u32;

    loop {
        if int & 0xFFFFFF80 == 0 {
            write_byte(int as u8)?;
            return Ok(());
        }

        write_byte(int as u8 | 0x80)?;

        int >>= 7;
    }
}

#[inline]
pub fn read_var_long<F, E>(mut read_byte: F) -> Result<i64, VarIntReadError<E>>
where
    F: FnMut() -> Result<u8, E>,
{
    let mut val = 0;
    for i in 0..VAR_LONG_MAX_LEN {
        let byte = read_byte()?;

        val |= (byte as i64 & 0x7F) << (i * 7);

        if byte & 0x80 == 0 {
            return Ok(val);
        }
    }

    Err(VarIntReadError::TooLarge)
}

#[inline]
pub fn write_var_long<F, E>(int: i64, mut write_byte: F) -> Result<(), E>
where
    F: FnMut(u8) -> Result<(), E>,
{
    let mut int = int as u64;

    loop {
        if int & 0xFFFFFFFFFFFFFF80 == 0 {
            write_byte(int as u8)?;
            return Ok(());
        }

        write_byte(int as u8 | 0x80)?;

        int >>= 7;
    }
}

/// Returns the number of bytes the varint will occupy once written.
#[inline]
pub fn var_int_len(int: i32) -> usize {
    match int {
        0 => 1,
        n => (31 - n.leading_zeros() as usize) / 7 + 1,
    }
}

pub const VAR_INT_MAX_LEN: usize = 5;
pub const VAR_LONG_MAX_LEN: usize = 10;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Error)]
pub enum VarIntReadError<E> {
    #[error(transparent)]
    ReadError(#[from] E),
    #[error("var int is too large")]
    TooLarge,
}
