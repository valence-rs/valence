pub use valence_protocol::packet::c2s::play::client_settings::ChatMode;
// use valence_protocol::packet::c2s::play::client_settings::MainArm;
use valence_protocol::packet::c2s::play::ClientSettingsC2s;

use super::*;
pub use crate::entity::player::{MainArm, PlayerModelParts};
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_system(
        handle_client_settings
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

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
