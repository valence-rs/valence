use glam::DVec3;

use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityDamageS2c {
    /// The ID of the entity taking damage
    pub entity_id: VarInt,
    /// The ID of the type of damage taken
    pub source_type_id: VarInt,
    /// The ID + 1 of the entity responsible for the damage, if present. If not
    /// present, the value is 0
    pub source_cause_id: VarInt,
    /// The ID + 1 of the entity that directly dealt the damage, if present. If
    /// not present, the value is 0. If this field is present:
    /// * and damage was dealt indirectly, such as by the use of a projectile,
    ///   this field will contain the ID of such projectile;
    /// * and damage was dealt dirctly, such as by manually attacking, this
    ///   field will contain the same value as Source Cause ID.
    pub source_direct_id: VarInt,
    /// The Notchian server sends the Source Position when the damage was dealt
    /// by the /damage command and a position was specified
    pub source_pos: Option<DVec3>,
}
