use std::io::Write;

use super::var_int::VarInt;
use super::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MessageSignature<'a> {
    ByIndex(i32),
    BySignature(&'a [u8; 256]),
}

impl<'a> Encode for MessageSignature<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            MessageSignature::ByIndex(index) => VarInt(index + 1).encode(&mut w)?,
            MessageSignature::BySignature(signature) => {
                VarInt(0).encode(&mut w)?;
                signature.encode(&mut w)?;
            }
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for MessageSignature<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let index = VarInt::decode(r)?.0.saturating_sub(1);

        if index == -1 {
            Ok(MessageSignature::BySignature(<&[u8; 256]>::decode(r)?))
        } else {
            Ok(MessageSignature::ByIndex(index))
        }
    }
}
