#![allow(clippy::all, missing_docs)]

use std::fmt;
use std::io::{Read, Write};

use anyhow::Context;

use crate::protocol::{Decode, Encode, VarInt};

include!(concat!(env!("OUT_DIR"), "/block.rs"));

impl fmt::Debug for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let typ = self.to_type();

        write!(f, "{}", typ.to_str())?;

        let props = typ.props();

        if !props.is_empty() {
            let mut list = f.debug_list();
            for &p in typ.props() {
                struct KeyVal<'a>(&'a str, &'a str);

                impl<'a> fmt::Debug for KeyVal<'a> {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{}={}", self.0, self.1)
                    }
                }

                list.entry(&KeyVal(p.to_str(), self.get(p).unwrap().to_str()));
            }
            list.finish()
        } else {
            Ok(())
        }
    }
}

impl Encode for BlockState {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(self.0 as i32).encode(w)
    }
}

impl Decode for BlockState {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?.0;
        let errmsg = "invalid block state ID";

        BlockState::from_raw(id.try_into().context(errmsg)?).context(errmsg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_consistency() {
        for typ in BlockType::ALL {
            let block = typ.to_state();

            for &prop in typ.props() {
                let new_block = block.set(prop, block.get(prop).unwrap());
                assert_eq!(new_block, block);
            }
        }
    }
}
