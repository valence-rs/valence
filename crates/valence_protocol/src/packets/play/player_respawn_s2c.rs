use super::*;

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct PlayerRespawnS2c<'a> {
    pub dimension_type_name: Ident<Cow<'a, str>>,
    pub dimension_name: Ident<Cow<'a, str>>,
    pub hashed_seed: u64,
    pub game_mode: GameMode,
    pub previous_game_mode: OptGameMode,
    pub is_debug: bool,
    pub is_flat: bool,
    pub copy_metadata: bool,
    pub last_death_location: Option<GlobalPos<'a>>,
    pub portal_cooldown: VarInt,
}
