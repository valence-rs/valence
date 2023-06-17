use bevy_ecs::prelude::{Component, Entity, Bundle};
use valence_core::{uuid::UniqueId, text::Text, protocol::{Encode, Decode}};
use bitfield_struct::bitfield;

#[derive(Bundle)]
pub struct BossBarBundle {
    pub id: UniqueId,
    pub title: BossBarTitle,
    pub health: BossBarHealth,
    pub style: BossBarStyle,
    pub flags: BossBarFlags,
    pub viewers: BossBarViewers,
}

impl BossBarBundle {

    pub fn new(title: Text, color: BossBarColor, division: BossBarDivision, flags: BossBarFlags) -> BossBarBundle {
        BossBarBundle {
            id: UniqueId::default(),
            title: BossBarTitle(title),
            health: BossBarHealth(1.0),
            style: BossBarStyle {
                color,
                division,
            },
            flags,
            viewers: BossBarViewers::new(),
        }
    }

}

#[derive(Component, Clone)]
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
pub struct BossBarViewers {
    pub current_viewers: Vec<Entity>,
    pub last_viewers: Vec<Entity>,
}

impl BossBarViewers {

    pub fn new() -> BossBarViewers {
        BossBarViewers {
            current_viewers: Vec::new(),
            last_viewers: Vec::new(),
        }
    }

}