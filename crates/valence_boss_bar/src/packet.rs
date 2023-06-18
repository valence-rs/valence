use uuid::Uuid;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_core::text::Text;

use crate::components::{BossBarColor, BossBarDivision, BossBarFlags};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BOSS_BAR_S2C)]
pub struct BossBarS2c {
    pub id: Uuid,
    pub action: BossBarAction,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum BossBarAction {
    Add {
        title: Text,
        health: f32,
        color: BossBarColor,
        division: BossBarDivision,
        flags: BossBarFlags,
    },
    Remove,
    UpdateHealth(f32),
    UpdateTitle(Text),
    UpdateStyle(BossBarColor, BossBarDivision),
    UpdateFlags(BossBarFlags),
}
