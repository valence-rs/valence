use std::borrow::Cow;
use std::collections::BTreeSet;

use valence_ident::Ident;

use crate::game_mode::OptGameMode;
use crate::{Decode, Encode, GameMode, GlobalPos, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct LoginS2c<'a> {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub dimension_names: Cow<'a, BTreeSet<Ident<Cow<'a, str>>>>,
    pub max_players: VarInt,
    pub view_distance: VarInt,
    pub simulation_distance: VarInt,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    pub dimension_type: VarInt, // from the dimention type registry
    pub dimension_name: Ident<Cow<'a, str>>,
    pub hashed_seed: i64,
    pub game_mode: GameMode,
    pub previous_game_mode: OptGameMode,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death_location: Option<GlobalPos<'a>>,
    pub portal_cooldown: VarInt,
    pub sea_level: VarInt,
    pub enforeces_secure_chat: bool,
}
