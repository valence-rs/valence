use uuid::Uuid;
use valence_core::{protocol::{Encode, Decode, Packet, packet_id}, text::Text};

use crate::components::{BossBarColor, BossBarDivision, BossBarFlags, BossBarBundle};

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

impl BossBarBundle {

    pub fn generate_add_packet(&self) -> BossBarS2c {
        BossBarS2c {
            id: self.id.0,
            action: BossBarAction::Add {
                title: self.title.clone().0,
                health: self.health.0,
                color: self.style.color,
                division: self.style.division,
                flags: self.flags,
            },
        }
    }

    pub fn generate_remove_packet(&self) -> BossBarS2c {
        BossBarS2c {
            id: self.id.0,
            action: BossBarAction::Remove,
        }
    }

}