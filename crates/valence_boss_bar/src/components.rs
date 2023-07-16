use std::collections::BTreeSet;

use bevy_ecs::prelude::{Bundle, Component, Entity};
use bitfield_struct::bitfield;
use valence_core::protocol::{Decode, Encode};
use valence_core::text::Text;
use valence_core::uuid::UniqueId;

/// The bundle of components that make up a boss bar.
#[derive(Bundle, Default)]
pub struct BossBarBundle {
    pub id: UniqueId,
    pub title: BossBarTitle,
    pub health: BossBarHealth,
    pub style: BossBarStyle,
    pub flags: BossBarFlags,
    pub viewers: BossBarViewers,
}

/// The title of a boss bar.
#[derive(Component, Clone, Default)]
pub struct BossBarTitle(pub Text);

/// The health of a boss bar.
#[derive(Component, Default)]
pub struct BossBarHealth(pub f32);

/// The style of a boss bar. This includes the color and division of the boss
/// bar.
#[derive(Component, Default)]
pub struct BossBarStyle {
    pub color: BossBarColor,
    pub division: BossBarDivision,
}

/// The color of a boss bar.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Encode, Decode)]
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
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Encode, Decode)]
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
#[derive(Component, PartialEq, Eq, Default, Encode, Decode)]
pub struct BossBarFlags {
    pub darken_sky: bool,
    pub dragon_bar: bool,
    pub create_fog: bool,
    #[bits(5)]
    _pad: u8,
}

/// The viewers of a boss bar.
#[derive(Component, Default)]
pub struct BossBarViewers {
    /// The current viewers of the boss bar. It is the list that should be
    /// updated.
    pub viewers: BTreeSet<Entity>,
    /// The viewers of the last tick in order to determine which viewers have
    /// been added and removed.
    pub(crate) old_viewers: BTreeSet<Entity>,
}
