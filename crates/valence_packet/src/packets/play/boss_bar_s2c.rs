use bevy_ecs::prelude::Component;

use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BOSS_BAR_S2C)]
pub struct BossBarS2c<'a> {
    pub id: Uuid,
    pub action: BossBarAction<'a>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum BossBarAction<'a> {
    Add {
        title: Cow<'a, Text>,
        health: f32,
        color: BossBarColor,
        division: BossBarDivision,
        flags: BossBarFlags,
    },
    Remove,
    UpdateHealth(f32),
    UpdateTitle(Cow<'a, Text>),
    UpdateStyle(BossBarColor, BossBarDivision),
    UpdateFlags(BossBarFlags),
}

/// The color of a boss bar.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Default)]
pub enum BossBarColor {
    #[default]
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

/// The division of a boss bar.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Default)]
pub enum BossBarDivision {
    #[default]
    NoDivision,
    SixNotches,
    TenNotches,
    TwelveNotches,
    TwentyNotches,
}

/// The flags of a boss bar (darken sky, dragon bar, create fog).
#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode, Component, Default)]
pub struct BossBarFlags {
    pub darken_sky: bool,
    pub dragon_bar: bool,
    pub create_fog: bool,
    #[bits(5)]
    _pad: u8,
}

impl ToPacketAction for BossBarFlags {
    fn to_packet_action(&self) -> BossBarAction {
        BossBarAction::UpdateFlags(*self)
    }
}

/// Trait for converting a component to a boss bar action.
pub trait ToPacketAction {
    fn to_packet_action(&self) -> BossBarAction;
}
