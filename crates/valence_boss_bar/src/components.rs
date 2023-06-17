use bevy_ecs::prelude::{Component, Entity};
use valence_core::{uuid::UniqueId, text::Text, protocol::{Encode, Decode}};
use bitfield_struct::bitfield;

#[derive(Component)]
pub struct BossBar {
    pub id: UniqueId,
    pub title: BossBarTitle,
    pub health: BossBarHealth,
    pub style: BossBarStyle,
    pub flags: BossBarFlags,
    pub viewers: BossBarViewers,
}

impl BossBar {

    pub fn new(title: Text, color: BossBarColor, division: BossBarDivision, flags: BossBarFlags) -> BossBar {
        BossBar {
            id: UniqueId::default(),
            title: BossBarTitle(title),
            health: BossBarHealth(1.0),
            style: BossBarStyle {
                color,
                division,
            },
            flags,
            viewers: BossBarViewers(Vec::new()),
        }
    }

    pub fn update_health(&mut self, health: f32) {
        self.health.0 = health;
    }

    pub fn update_title(&mut self, title: Text) {
        self.title.0 = title;
    }

    pub fn update_style(&mut self, color: BossBarColor, division: BossBarDivision) {
        self.style.color = color;
        self.style.division = division;
    }

    pub fn update_flags(&mut self, flags: BossBarFlags) {
        self.flags = flags;
    }

    pub fn add_client(&mut self, entity: Entity) {
        self.viewers.0.push(entity);
    }

    pub fn remove_client(&mut self, entity: Entity) {
        self.viewers.0.retain(|e| *e != entity);
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
pub struct BossBarViewers(pub Vec<Entity>);