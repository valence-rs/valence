use valence_protocol::block_pos::BlockPos;
use valence_protocol::packet::c2s::play::player_action::Action;
use valence_protocol::packet::c2s::play::PlayerActionC2s;
use valence_protocol::types::Direction;

use super::*;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

#[derive(Copy, Clone, Debug)]
pub struct Digging {
    pub client: Entity,
    pub position: BlockPos,
    pub direction: Direction,
    pub digging_state: DiggingState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DiggingState {
    Start,
    Abort,
    Stop,
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct ActionSequence(i32);

pub(super) fn build(app: &mut App) {
    app.add_event::<Digging>()
        .add_system(
            handle_player_action
                .in_schedule(EventLoopSchedule)
                .in_base_set(EventLoopSet::PreUpdate),
        )
        .add_system(
            acknowledge_player_actions
                .in_base_set(CoreSet::PostUpdate)
                .before(FlushPacketsSet),
        );
}

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
    mut digging_events: EventWriter<Digging>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            if let Ok(mut seq) = clients.get_mut(packet.client) {
                seq.update(pkt.sequence.0);
            }

            // TODO: check that digging is happening within configurable distance to client.
            // TODO: check that blocks are being broken at the appropriate speeds.

            match pkt.action {
                Action::StartDestroyBlock => digging_events.send(Digging {
                    client: packet.client,
                    position: pkt.position,
                    direction: pkt.direction,
                    digging_state: DiggingState::Start,
                }),
                Action::AbortDestroyBlock => digging_events.send(Digging {
                    client: packet.client,
                    position: pkt.position,
                    direction: pkt.direction,
                    digging_state: DiggingState::Abort,
                }),
                Action::StopDestroyBlock => digging_events.send(Digging {
                    client: packet.client,
                    position: pkt.position,
                    direction: pkt.direction,
                    digging_state: DiggingState::Stop,
                }),
                Action::DropAllItems => {}
                Action::DropItem => {}
                Action::ReleaseUseItem => todo!(), // TODO: release use item.
                Action::SwapItemWithOffhand => {}
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
