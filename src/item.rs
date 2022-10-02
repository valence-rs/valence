//! Items

use anyhow::Context;

use crate::protocol::{Decode, Encode, VarInt};

include!(concat!(env!("OUT_DIR"), "/item.rs"));

impl Encode for ItemKind {
    fn encode(&self, w: &mut impl std::io::Write) -> anyhow::Result<()> {
        VarInt(self.to_raw() as i32).encode(w)
    }
}

impl Decode for ItemKind {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?.0;
        let errmsg = "invalid item ID";

        ItemKind::from_raw(id.try_into().context(errmsg)?).context(errmsg)
    }
}
