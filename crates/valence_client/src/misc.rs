use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use glam::Vec3;
use valence_core::block_pos::BlockPos;
use valence_core::direction::Direction;
use valence_core::hand::Hand;
use valence_core::packet::c2s::play::{
    ChatMessageC2s, ClientStatusC2s, HandSwingC2s, PlayerInteractBlockC2s, PlayerInteractItemC2s,
    ResourcePackStatusC2s,
};

use super::action::ActionSequence;
use valence_entity::{EntityAnimation, EntityAnimations};
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<HandSwing>()
        .add_event::<InteractBlock>()
        .add_event::<ChatMessage>()
        .add_event::<Respawn>()
        .add_event::<RequestStats>()
        .add_event::<ResourcePackStatusChange>()
        .add_system(
            handle_misc_packets
                .in_schedule(EventLoopSchedule)
                .in_base_set(EventLoopSet::PreUpdate),
        );
}

#[derive(Copy, Clone, Debug)]
pub struct HandSwing {
    pub client: Entity,
    pub hand: Hand,
}

#[derive(Copy, Clone, Debug)]
pub struct InteractBlock {
    pub client: Entity,
    /// The hand that was used
    pub hand: Hand,
    /// The location of the block that was interacted with
    pub position: BlockPos,
    /// The face of the block that was clicked
    pub face: Direction,
    /// The position inside of the block that was clicked on
    pub cursor_pos: Vec3,
    /// Whether or not the player's head is inside a block
    pub head_inside_block: bool,
    /// Sequence number for synchronization
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub client: Entity,
    pub message: Box<str>,
    pub timestamp: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct Respawn {
    pub client: Entity,
}

#[derive(Copy, Clone, Debug)]
pub struct RequestStats {
    pub client: Entity,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourcePackStatus {
    /// The client has accepted the server's resource pack.
    Accepted,
    /// The client has declined the server's resource pack.
    Declined,
    /// The client has successfully loaded the server's resource pack.
    Loaded,
    /// The client has failed to download the server's resource pack.
    FailedDownload,
}

#[derive(Clone, Debug)]
pub struct ResourcePackStatusChange {
    pub client: Entity,
    pub status: ResourcePackStatus,
}

#[allow(clippy::too_many_arguments)]
fn handle_misc_packets(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(&mut ActionSequence, &mut EntityAnimations)>,
    mut hand_swing_events: EventWriter<HandSwing>,
    mut interact_block_events: EventWriter<InteractBlock>,
    mut chat_message_events: EventWriter<ChatMessage>,
    mut respawn_events: EventWriter<Respawn>,
    mut request_stats_events: EventWriter<RequestStats>,
    mut resource_pack_status_change_events: EventWriter<ResourcePackStatusChange>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<HandSwingC2s>() {
            if let Ok((_, mut animations)) = clients.get_mut(packet.client) {
                animations.trigger(match pkt.hand {
                    Hand::Main => EntityAnimation::SwingMainHand,
                    Hand::Off => EntityAnimation::SwingOffHand,
                });
            }

            hand_swing_events.send(HandSwing {
                client: packet.client,
                hand: pkt.hand,
            });
        } else if let Some(pkt) = packet.decode::<PlayerInteractBlockC2s>() {
            if let Ok((mut action_seq, _)) = clients.get_mut(packet.client) {
                action_seq.update(pkt.sequence.0);
            }

            interact_block_events.send(InteractBlock {
                client: packet.client,
                hand: pkt.hand,
                position: pkt.position,
                face: pkt.face,
                cursor_pos: pkt.cursor_pos.into(),
                head_inside_block: pkt.head_inside_block,
                sequence: pkt.sequence.0,
            });
        } else if let Some(pkt) = packet.decode::<PlayerInteractItemC2s>() {
            if let Ok((mut action_seq, _)) = clients.get_mut(packet.client) {
                action_seq.update(pkt.sequence.0);
            }

            // TODO
        } else if let Some(pkt) = packet.decode::<ChatMessageC2s>() {
            chat_message_events.send(ChatMessage {
                client: packet.client,
                message: pkt.message.into(),
                timestamp: pkt.timestamp,
            });
        } else if let Some(pkt) = packet.decode::<ClientStatusC2s>() {
            match pkt {
                ClientStatusC2s::PerformRespawn => respawn_events.send(Respawn {
                    client: packet.client,
                }),
                ClientStatusC2s::RequestStats => request_stats_events.send(RequestStats {
                    client: packet.client,
                }),
            }
        } else if let Some(pkt) = packet.decode::<ResourcePackStatusC2s>() {
            resource_pack_status_change_events.send(ResourcePackStatusChange {
                client: packet.client,
                status: match pkt {
                    ResourcePackStatusC2s::Accepted => ResourcePackStatus::Accepted,
                    ResourcePackStatusC2s::Declined => ResourcePackStatus::Declined,
                    ResourcePackStatusC2s::SuccessfullyLoaded => ResourcePackStatus::Loaded,
                    ResourcePackStatusC2s::FailedDownload => ResourcePackStatus::FailedDownload,
                },
            });
        }
    }
}
