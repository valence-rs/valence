//! Blocks and related types.

#![allow(clippy::all, missing_docs)]

use std::fmt::{self, Display};
use std::io::Write;
use std::iter::FusedIterator;

use anyhow::Context;

pub use crate::block_pos::BlockPos;
use crate::item::ItemKind;
use crate::protocol::packets::c2s::play::BlockFace;
use crate::protocol::{Decode, Encode, VarInt};

include!(concat!(env!("OUT_DIR"), "/block.rs"));

impl BlockFace {
    pub const fn to_block_facing(self) -> PropValue {
        match self {
            BlockFace::Bottom => PropValue::Down,
            BlockFace::Top => PropValue::Up,
            BlockFace::North => PropValue::North,
            BlockFace::South => PropValue::South,
            BlockFace::West => PropValue::West,
            BlockFace::East => PropValue::East,
        }
    }

    pub const fn to_block_axis(self) -> PropValue {
        match self {
            BlockFace::Bottom | BlockFace::Top => PropValue::Y,
            BlockFace::North | BlockFace::South => PropValue::Z,
            BlockFace::West | BlockFace::East => PropValue::X,
        }
    }
}

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

    fn encoded_len(&self) -> usize {
        VarInt(self.0 as i32).encoded_len()
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
    fn standing_to_wall() {
        assert_eq!(
            BlockState::TORCH.to_wall_variant().unwrap(),
            BlockState::WALL_TORCH
        );
        assert_eq!(
            BlockState::SPRUCE_SIGN.to_wall_variant().unwrap(),
            BlockState::SPRUCE_WALL_SIGN
        );
        assert_eq!(
            BlockState::PURPLE_BANNER.to_wall_variant().unwrap(),
            BlockState::PURPLE_WALL_BANNER
        );

        assert_eq!(BlockState::NETHER_PORTAL.to_wall_variant(), None);
    }
}
