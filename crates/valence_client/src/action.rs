use valence_core::block_pos::BlockPos;
use valence_core::direction::Direction;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};

use super::*;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};
use valence_packet::client::{PlayerAction, PlayerActionC2s};

pub(super) fn build(app: &mut App) {
    app.add_event::<DiggingEvent>()
        .add_systems(EventLoopPreUpdate, handle_player_action)
        .add_systems(
            PostUpdate,
            acknowledge_player_actions.in_set(UpdateClientsSet),
        );
}

#[derive(Event, Copy, Clone, Debug)]
pub struct DiggingEvent {
    pub client: Entity,
    pub position: BlockPos,
    pub direction: Direction,
    pub state: DiggingState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DiggingState {
    Start,
    Abort,
    Stop,
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct ActionSequence(i32);

impl ActionSequence {
    pub fn update(&mut self, val: i32) {
        self.0 = self.0.max(val);
    }

    pub fn get(&self) -> i32 {
        self.0
    }
}

fn handle_player_action(
    mut clients: Query<&mut ActionSequence>,
    mut packets: EventReader<PacketEvent>,
    mut digging_events: EventWriter<DiggingEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            if let Ok(mut seq) = clients.get_mut(packet.client) {
                seq.update(pkt.sequence.0);
            }

            // TODO: check that digging is happening within configurable distance to client.
            // TODO: check that blocks are being broken at the appropriate speeds.

            match pkt.action {
                PlayerAction::StartDestroyBlock => digging_events.send(DiggingEvent {
                    client: packet.client,
                    position: pkt.position,
                    direction: pkt.direction,
                    state: DiggingState::Start,
                }),
                PlayerAction::AbortDestroyBlock => digging_events.send(DiggingEvent {
                    client: packet.client,
                    position: pkt.position,
                    direction: pkt.direction,
                    state: DiggingState::Abort,
                }),
                PlayerAction::StopDestroyBlock => digging_events.send(DiggingEvent {
                    client: packet.client,
                    position: pkt.position,
                    direction: pkt.direction,
                    state: DiggingState::Stop,
                }),
                PlayerAction::DropAllItems => {}
                PlayerAction::DropItem => {}
                PlayerAction::ReleaseUseItem => {}
                PlayerAction::SwapItemWithOffhand => {}
            }
        }
    }
}

fn acknowledge_player_actions(
    mut clients: Query<(&mut Client, &mut ActionSequence), Changed<ActionSequence>>,
) {
    for (mut client, mut action_seq) in &mut clients {
        if action_seq.0 != 0 {
            client.write_packet(&PlayerActionResponseS2c {
                sequence: VarInt(action_seq.0),
            });

            action_seq.0 = 0;
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_ACTION_RESPONSE_S2C)]
pub struct PlayerActionResponseS2c {
    pub sequence: VarInt,
}
