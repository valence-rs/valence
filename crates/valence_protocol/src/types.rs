//! Miscellaneous type definitions used in packets.

use std::borrow::Cow;
use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct PublicKeyData<'a> {
    pub timestamp: u64,
    pub public_key: &'a [u8],
    pub signature: &'a [u8],
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Encode, Decode)]
pub enum Hand {
    #[default]
    Main,
    Off,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Serialize, Deserialize)]
pub struct Property<S = String> {
    pub name: S,
    pub value: S,
    pub signature: Option<S>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum SoundCategory {
    Master,
    Music,
    Record,
    Weather,
    Block,
    Hostile,
    Neutral,
    Player,
    Ambient,
    Voice,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Encode, Decode)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct GlobalPos<'a> {
    pub dimension_name: Ident<Cow<'a, str>>,
    pub position: BlockPos,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum WindowType {
    Generic9x1,
    Generic9x2,
    Generic9x3,
    Generic9x4,
    Generic9x5,
    Generic9x6,
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    Crafting,
    Enchantment,
    Furnace,
    Grindstone,
    Hopper,
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    Cartography,
    Stonecutter,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Direction {
    /// -Y
    Down,
    /// +Y
    Up,
    /// -Z
    North,
    /// +Z
    South,
    /// -X
    West,
    /// +X
    East,
}

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
