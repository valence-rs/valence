use std::io::Write;

use anyhow::Context;
use uuid::Uuid;
use valence_generated::block::{BlockEntityKind, BlockKind, BlockState};
use valence_generated::item::ItemKind;
use valence_ident::{Ident, IdentError};
use valence_nbt::Compound;

use crate::{Decode, Encode, VarInt};

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            Some(t) => {
                true.encode(&mut w)?;
                t.encode(w)
            }
            None => false.encode(w),
        }
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Option<T> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(match bool::decode(r)? {
            true => Some(T::decode(r)?),
            false => None,
        })
    }
}

impl Encode for Uuid {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.as_u128().encode(w)
    }
}

impl<'a> Decode<'a> for Uuid {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        u128::decode(r).map(Uuid::from_u128)
    }
}

impl Encode for Compound {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        Ok(valence_nbt::to_binary(self, w, "")?)
    }
}

impl Decode<'_> for Compound {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        // Check for null compound.
        if r.first() == Some(&0) {
            *r = &r[1..];
            return Ok(Compound::new());
        }

        // TODO: consider if we need to bound the input slice or add some other
        // mitigation to prevent excessive memory usage on hostile input.
        Ok(valence_nbt::from_binary(r)?.0)
    }
}

impl<S: Encode> Encode for Ident<S> {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a, S> Decode<'a> for Ident<S>
where
    S: Decode<'a>,
    Ident<S>: TryFrom<S, Error = IdentError>,
{
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Ident::try_from(S::decode(r)?)?)
    }
}

impl Encode for BlockState {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        VarInt(self.to_raw() as i32).encode(w)
    }
}

impl Decode<'_> for BlockState {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?.0;
        let errmsg = "invalid block state ID";

        BlockState::from_raw(id.try_into().context(errmsg)?).context(errmsg)
    }
}

impl Encode for BlockKind {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        VarInt(self.to_raw() as i32).encode(w)
    }
}

impl Decode<'_> for BlockKind {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?.0;
        let errmsg = "invalid block kind ID";

        BlockKind::from_raw(id.try_into().context(errmsg)?).context(errmsg)
    }
}

impl Encode for BlockEntityKind {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        VarInt(self.id() as i32).encode(w)
    }
}

impl<'a> Decode<'a> for BlockEntityKind {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?;
        Self::from_id(id.0 as u32).with_context(|| format!("id {}", id.0))
    }
}

impl Encode for ItemKind {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        VarInt(self.to_raw() as i32).encode(w)
    }
}

impl Decode<'_> for ItemKind {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?.0;
        let errmsg = "invalid item ID";

        ItemKind::from_raw(id.try_into().context(errmsg)?).context(errmsg)
    }
}
