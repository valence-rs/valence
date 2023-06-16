use bevy_ecs::prelude::Component;
use valence_client::Client;
use valence_core::{protocol::{Decode, Encode}, text::Text, uuid::UniqueId};
use bitfield_struct::bitfield;

pub mod packet;

pub struct BossBar {
    pub id: UniqueId,
    pub title: BossBarTitle,
    pub health: BossBarHealth,
    pub style: BossBarStyle,
    pub flags: BossBarFlags,
    pub viewers: BossBarViewers,
}

#[derive(Component)]
pub struct BossBarTitle(pub Text);

#[derive(Component)]
pub struct BossBarHealth(pub f32);

#[derive(Component)]
pub struct BossBarStyle {
    pub color: BossBarColor,
    pub division: BossBarDivision,
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum BossBarDivision {
    NoDivision,
    SixNotches,
    TenNotches,
    TwelveNotches,
    TwentyNotches,
}

#[bitfield(u8)]
#[derive(Component, PartialEq, Eq, Encode, Decode)]
pub struct BossBarFlags {
    pub darken_sky: bool,
    pub dragon_bar: bool,
    pub create_fog: bool,
    #[bits(5)]
    _pad: u8,
}

#[derive(Component)]
pub struct BossBarViewers(pub Vec<Client>);