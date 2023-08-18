use std::borrow::Cow;

use bevy_ecs::prelude::{Bundle, Component};
use valence_entity::EntityLayerId;
use valence_server::protocol::packets::play::boss_bar_s2c::{
    BossBarAction, BossBarColor, BossBarDivision, BossBarFlags, ToPacketAction,
};
use valence_server::{Text, UniqueId};

/// The bundle of components that make up a boss bar.
#[derive(Bundle, Default)]
pub struct BossBarBundle {
    pub id: UniqueId,
    pub title: BossBarTitle,
    pub health: BossBarHealth,
    pub style: BossBarStyle,
    pub flags: BossBarFlags,
    pub layer: EntityLayerId,
}

/// The title of a boss bar.
#[derive(Component, Clone, Default)]
pub struct BossBarTitle(pub Text);

impl ToPacketAction for BossBarTitle {
    fn to_packet_action(&self) -> BossBarAction {
        BossBarAction::UpdateTitle(Cow::Borrowed(&self.0))
    }
}

/// The health of a boss bar.
#[derive(Component, Default)]
pub struct BossBarHealth(pub f32);

impl ToPacketAction for BossBarHealth {
    fn to_packet_action(&self) -> BossBarAction {
        BossBarAction::UpdateHealth(self.0)
    }
}

/// The style of a boss bar. This includes the color and division of the boss
/// bar.
#[derive(Component, Default)]
pub struct BossBarStyle {
    pub color: BossBarColor,
    pub division: BossBarDivision,
}

impl ToPacketAction for BossBarStyle {
    fn to_packet_action(&self) -> BossBarAction {
        BossBarAction::UpdateStyle(self.color, self.division)
    }
}

impl ToPacketAction for BossBarFlags {
    fn to_packet_action(&self) -> BossBarAction {
        BossBarAction::UpdateFlags(*self)
    }
}

/// Trait for converting a component to a boss bar action.
trait ToPacketAction {
    fn to_packet_action(&self) -> BossBarAction;
}
