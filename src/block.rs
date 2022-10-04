//! Blocks and related types.

#![allow(clippy::all, missing_docs)]

use std::fmt::{self, Display};
use std::io::Write;
use std::iter::FusedIterator;

use anyhow::Context;

pub use crate::block_pos::BlockPos;
use crate::item::ItemKind;
use crate::protocol::{Decode, Encode, VarInt};

include!(concat!(env!("OUT_DIR"), "/block.rs"));

impl fmt::Debug for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_block_state(*self, f)
    }
}

impl Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_block_state(*self, f)
    }
}

fn fmt_block_state(bs: BlockState, f: &mut fmt::Formatter) -> fmt::Result {
    let kind = bs.to_kind();

    write!(f, "{}", kind.to_str())?;

    let props = kind.props();

    if !props.is_empty() {
        let mut list = f.debug_list();
        for &p in kind.props() {
            struct KeyVal<'a>(&'a str, &'a str);

            impl<'a> fmt::Debug for KeyVal<'a> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "{}={}", self.0, self.1)
                }
            }

            list.entry(&KeyVal(p.to_str(), bs.get(p).unwrap().to_str()));
        }
        list.finish()
    } else {
        Ok(())
    }
}

impl Encode for BlockState {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(self.0 as i32).encode(w)
    }
}

impl Decode for BlockState {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
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
        for kind in BlockKind::ALL {
            let block = kind.to_state();

            for &prop in kind.props() {
                let new_block = block.set(prop, block.get(prop).unwrap());
                assert_eq!(new_block, block);
            }
        }
    }

    #[test]
    fn item_to_block_state() {
        let item = ItemKind::OakWood;

        assert_eq!(BlockKind::from_item_kind(item).unwrap(), BlockKind::OakWood)
    }

    #[test]
    fn block_state_to_item() {
        let block = BlockKind::SlimeBlock;

        let new_item = block.to_item_kind().unwrap();

        assert_eq!(new_item, ItemKind::SlimeBlock)
    }

    #[test]
    fn block_state_to_wall_state() {
        let block = BlockState::TORCH;
        let wall = block.wall_state().unwrap();

        assert_eq!(wall, BlockState::WALL_TORCH)
    }
}
