use crate::ident::Ident;
use crate::types::{GameMode, GlobalPos};
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct PlayerRespawnS2c<'a> {
    pub dimension_type_name: Ident<&'a str>,
    pub dimension_name: Ident<&'a str>,
    pub hashed_seed: u64,
    pub game_mode: GameMode,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub copy_metadata: bool,
    pub last_death_location: Option<GlobalPos<'a>>,
}
