use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use glam::Vec3;
use valence_core::block_pos::BlockPos;
use valence_core::direction::Direction;
use valence_core::hand::Hand;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};

use crate::action::ActionSequence;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<InteractBlockEvent>().add_system(
        handle_interact_block
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

#[derive(Copy, Clone, Debug)]
pub struct InteractBlockEvent {
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

fn handle_interact_block(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<&mut ActionSequence>,
    mut events: EventWriter<InteractBlockEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PlayerInteractBlockC2s>() {
            if let Ok(mut action_seq) = clients.get_mut(packet.client) {
                action_seq.update(pkt.sequence.0);
            }

            // TODO: check that the block interaction is valid.

            events.send(InteractBlockEvent {
                client: packet.client,
                hand: pkt.hand,
                position: pkt.position,
                face: pkt.face,
                cursor_pos: pkt.cursor_pos,
                head_inside_block: pkt.head_inside_block,
                sequence: pkt.sequence.0,
            });
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_INTERACT_BLOCK_C2S)]
pub struct PlayerInteractBlockC2s {
    pub hand: Hand,
    pub position: BlockPos,
    pub face: Direction,
    pub cursor_pos: Vec3,
    pub head_inside_block: bool,
    pub sequence: VarInt,
}
