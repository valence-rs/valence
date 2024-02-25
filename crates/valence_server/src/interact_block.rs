use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_math::Vec3;
use valence_protocol::packets::play::PlayerInteractBlockC2s;
use valence_protocol::{BlockPos, Direction, Hand};

use crate::action::ActionSequence;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct InteractBlockPlugin;

impl Plugin for InteractBlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InteractBlockEvent>()
            .add_systems(EventLoopPreUpdate, handle_interact_block);
    }
}

#[derive(Event, Copy, Clone, Debug)]
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
    mut interact_block_events: EventWriter<InteractBlockEvent>,
) {
    for packet in packets.read() {
        let Some(pkt) = packet.decode::<PlayerInteractBlockC2s>() else {
            continue;
        };

        if let Ok(mut action_seq) = clients.get_mut(packet.client) {
            action_seq.update(pkt.sequence.0);
        }

        // TODO: check that the block interaction is valid.

        interact_block_events.send(InteractBlockEvent {
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
