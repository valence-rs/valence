use std::borrow::Cow;

use bevy_ecs::prelude::Component;
use bitfield_struct::bitfield;
use uuid::Uuid;
use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct BossEventS2c<'a> {
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
#[derive(PartialEq, Eq, Encode, Decode, Component)]
pub struct BossBarFlags {
    pub darken_sky: bool,
    pub dragon_bar: bool,
    pub create_fog: bool,
    #[bits(5)]
    _pad: u8,
}
