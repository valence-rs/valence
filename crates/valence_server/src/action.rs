use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::Deref;
use valence_protocol::packets::play::player_action_c2s::PlayerAction;
use valence_protocol::packets::play::{PlayerActionC2s, PlayerActionResponseS2c};
use valence_protocol::{BlockPos, Direction, VarInt, WritePacket};

use crate::client::{Client, FlushPacketsSet};
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct ActionPlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActionSet;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DiggingEvent>()
            .configure_set(PostUpdate, ActionSet.before(FlushPacketsSet))
            .add_systems(EventLoopPreUpdate, handle_player_action)
            .add_systems(PostUpdate, acknowledge_player_actions.in_set(ActionSet));
    }
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

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref)]
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
