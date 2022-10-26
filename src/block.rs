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
    /// Returns a `PropValue` to use when setting a propety in a block state.
    ///
    /// # Examples
    /// Making a Torch hang on a wall:
    /// ```rust
    /// use valence::block::{BlockState, PropName};
    /// use valence::protocol::packets::c2s::play::BlockFace;
    ///
    /// let torch = BlockState::WALL_TORCH;
    /// let face = BlockFace::West;
    /// // Now the torch is hanging on a west facing wall.
    /// torch.set(PropName::Facing, face.to_block_facing());
    /// ```
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

    /// Retuns wether the block should hang on the wall, be on the celling or on
    /// the floor.
    ///
    /// # Examples
    /// Making a button be on the celling:
    /// ```rust
    /// use valence::block::{BlockState, PropName};
    /// use valence::protocol::packets::c2s::play::BlockFace;
    ///
    /// let button = BlockState::OAK_BUTTON;
    /// let face = BlockFace::Bottom;
    /// // Now the button is on the celling.
    /// button.set(PropName::Face, face.to_block_facing());
    /// ```
    /// Making a lever hang on a wall:
    /// ```rust
    /// use valence::block::{BlockState, PropName};
    /// use valence::protocol::packets::c2s::play::BlockFace;
    ///
    /// let lever = BlockState::LEVER;
    /// let face = BlockFace::West;
    /// // Now the lever is hanging on a wall.
    /// lever.set(PropName::Face, face.to_block_facing());
    /// ```
    /// Making a
    pub const fn to_block_face(self) -> PropValue {
        match self {
            BlockFace::Bottom => PropValue::Ceiling,
            BlockFace::Top => PropValue::Floor,
            _ => PropValue::Wall,
        }
    }

    /// Returns the axis that the block should be on.
    ///
    /// # Examples
    /// Setting an oak logs axis:
    /// ```rust
    /// use valence::block::{BlockState, PropName};
    /// use valence::protocol::packets::c2s::play::BlockFace;
    ///
    /// let log = BlockState::OAK_LOG;
    /// let face = BlockFace::East;
    /// // Now the log's axis is the x axis
    /// log.set(PropName::axis, face.to_block_axis());
    /// ```
    pub const fn to_block_axis(self) -> PropValue {
        match self {
            BlockFace::Bottom | BlockFace::Top => PropValue::Y,
            BlockFace::North | BlockFace::South => PropValue::Z,
            BlockFace::West | BlockFace::East => PropValue::X,
        }
    }

    /// Returns the opposite direction to the current
    ///
    /// # Examples
    /// Basic usage:
    /// ```rust
    /// use valence::protocol::packets::c2s::play::BlockFace;
    ///
    /// assert_eq(BlockFace::West.opposite(), BlockFace::East)
    /// ```
    pub const fn opposite(self) -> Self {
        match self {
            BlockFace::Bottom => BlockFace::Top,
            BlockFace::Top => BlockFace::Bottom,
            BlockFace::North => BlockFace::South,
            BlockFace::South => BlockFace::North,
            BlockFace::West => BlockFace::East,
            BlockFace::East => BlockFace::West,
        }
    }

    /// Returns the direction to the right side of the current
    ///
    /// `BlockFace::Bottom` and `BlockFace::Top` can't be rotated, so they just
    /// return them self again
    pub const fn rotate_right(self) -> Self {
        match self {
            BlockFace::Bottom => BlockFace::Bottom,
            BlockFace::Top => BlockFace::Top,
            BlockFace::North => BlockFace::East,
            BlockFace::South => BlockFace::West,
            BlockFace::West => BlockFace::North,
            BlockFace::East => BlockFace::South,
        }
    }

    /// Returns the direction to the left side of the current
    ///
    /// `BlockFace::Bottom` and `BlockFace::Top` can't be rotated, so they just
    /// return them self again
    pub const fn rotate_left(self) -> Self {
        match self {
            BlockFace::Bottom => BlockFace::Bottom,
            BlockFace::Top => BlockFace::Top,
            BlockFace::North => BlockFace::West,
            BlockFace::South => BlockFace::East,
            BlockFace::West => BlockFace::South,
            BlockFace::East => BlockFace::North,
        }
    }

    // Used for barrel, command block, dispenser, dropper, observer and piston
    // placement
    // For more info look at `net/minecraft/item/ItemPlacementContext.java:53`
    pub fn player_look_direction(pitch: f32, yaw: f32) -> Self {
        let pitch_pi = pitch * (std::f32::consts::PI / 180.0);
        let yaw_pi = -yaw * (std::f32::consts::PI / 180.0);
        let pitch_sin = f32::sin(pitch_pi);
        let pitch_cos = f32::cos(pitch_pi);
        let yaw_sin = f32::sin(yaw_pi);
        let yaw_cos = f32::cos(yaw_pi);
        let yaw_sin_greater = yaw_sin > 0.0;
        let pitch_sin_lesser = pitch_sin < 0.0;
        let yaw_cos_greater = yaw_cos > 0.0;
        let yaw_sin = if yaw_sin_greater { yaw_sin } else { -yaw_sin };
        let pitch_sin = if pitch_sin_lesser {
            -pitch_sin
        } else {
            pitch_sin
        };
        let yaw_cos = if yaw_cos_greater { yaw_cos } else { -yaw_cos };
        let yaw_sin_pitch_cos = yaw_sin * pitch_cos;
        let yaw_cos_pitch_cos = yaw_cos * pitch_cos;
        if yaw_sin > yaw_cos {
            if pitch_sin > yaw_sin_pitch_cos {
                return if pitch_sin_lesser {
                    BlockFace::Top
                } else {
                    BlockFace::Bottom
                };
            }
            return if yaw_sin_greater {
                BlockFace::East
            } else {
                BlockFace::West
            };
        }
        if pitch_sin > yaw_cos_pitch_cos {
            return if pitch_sin_lesser {
                BlockFace::Top
            } else {
                BlockFace::Bottom
            };
        }
        return if yaw_cos_greater {
            BlockFace::South
        } else {
            BlockFace::North
        };
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
