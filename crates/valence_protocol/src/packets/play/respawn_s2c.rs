use std::borrow::Cow;

use bitfield_struct::bitfield;
use valence_ident::Ident;

use crate::game_mode::OptGameMode;
use crate::{Decode, Encode, GameMode, GlobalPos, Packet, VarInt};

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct RespawnS2c<'a> {
    pub dimension_type: VarInt,
    pub dimension_name: Ident<Cow<'a, str>>,
    pub hashed_seed: u64,
    pub game_mode: GameMode,
    pub previous_game_mode: OptGameMode,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death_location: Option<GlobalPos<'a>>,
    pub portal_cooldown: VarInt,
    pub data_kept: DataKeptFlags,
}
#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct DataKeptFlags {
    pub keep_attributes: bool,
    pub keep_metadata: bool,
    #[bits(6)]
    _pad: u8,
}
