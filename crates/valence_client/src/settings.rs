use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
pub use packet::ChatMode;
pub use valence_entity::player::{MainArm, PlayerModelParts};

use self::packet::ClientSettingsC2s;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};
use crate::ViewDistance;

pub(super) fn build(app: &mut App) {
    app.add_system(
        handle_client_settings
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
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
        &mut MainArm,
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
                main_arm.set_if_neq(MainArm(pkt.main_arm as i8));
            }
        }
    }
}

pub mod packet {
    use valence_core::protocol::{packet_id, Decode, Encode, Packet};

    use crate::packet::DisplayedSkinParts;

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
}
