use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bitfield_struct::bitfield;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_entity::player::{self, PlayerModelParts};

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};
use crate::ViewDistance;

pub(super) fn build(app: &mut App) {
    app.add_systems(EventLoopPreUpdate, handle_client_settings);
}

/// Component containing client-controlled settings about a client.
#[derive(Component, Default, Debug)]
pub struct ClientSettings {
    pub locale: Box<str>,
    pub chat_mode: ChatMode,
    pub chat_colors: bool,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

fn handle_client_settings(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(
        &mut ViewDistance,
        &mut ClientSettings,
        &mut PlayerModelParts,
        &mut player::MainArm,
    )>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<ClientSettingsC2s>() {
            if let Ok((mut view_dist, mut settings, mut model_parts, mut main_arm)) =
                clients.get_mut(packet.client)
            {
                view_dist.set_if_neq(ViewDistance::new(pkt.view_distance));

                settings.locale = pkt.locale.into();
                settings.chat_mode = pkt.chat_mode;
                settings.chat_colors = pkt.chat_colors;
                settings.enable_text_filtering = pkt.enable_text_filtering;
                settings.allow_server_listings = pkt.allow_server_listings;

                model_parts.set_if_neq(PlayerModelParts(u8::from(pkt.displayed_skin_parts) as i8));
                main_arm.set_if_neq(player::MainArm(pkt.main_arm as i8));
            }
        }
    }
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLIENT_SETTINGS_C2S)]
pub struct ClientSettingsC2s<'a> {
    pub locale: &'a str,
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    pub chat_colors: bool,
    pub displayed_skin_parts: DisplayedSkinParts,
    pub main_arm: MainArm,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct DisplayedSkinParts {
    pub cape: bool,
    pub jacket: bool,
    pub left_sleeve: bool,
    pub right_sleeve: bool,
    pub left_pants_leg: bool,
    pub right_pants_leg: bool,
    pub hat: bool,
    _pad: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Encode, Decode)]
pub enum ChatMode {
    Enabled,
    CommandsOnly,
    #[default]
    Hidden,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Encode, Decode)]
pub enum MainArm {
    Left,
    #[default]
    Right,
}

impl From<MainArm> for player::MainArm {
    fn from(value: MainArm) -> Self {
        Self(value as i8)
    }
}
