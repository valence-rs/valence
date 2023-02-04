use std::io::Write;

use crate::{Decode, Encode, VarInt};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MessageSignature<'a> {
    pub message_id: i32,
    pub signature: Option<&'a [u8; 256]>,
}

impl<'a> Encode for MessageSignature<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        VarInt(self.message_id + 1).encode(&mut w)?;

        match self.signature {
            None => {}
            Some(signature) => signature.encode(&mut w)?,
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for MessageSignature<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let message_id = VarInt::decode(r)?.0 - 1;

        let signature = if message_id == -1 {
            Some(<&[u8; 256]>::decode(r)?)
        } else {
            None
        };

        Ok(Self {
            message_id,
            signature,
        })
    }
}
