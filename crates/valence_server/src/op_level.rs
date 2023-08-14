use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_protocol::packets::play::EntityStatusS2c;
use valence_protocol::WritePacket;

use crate::client::{Client, UpdateClientsSet};

pub struct OpLevelPlugin;

impl Plugin for OpLevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_op_level.in_set(UpdateClientsSet));
    }
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct OpLevel(u8);

impl OpLevel {
    pub fn get(&self) -> u8 {
        self.0
    }

    /// Sets the op level. Value is clamped to `0..=3`.
    pub fn set(&mut self, lvl: u8) {
        self.0 = lvl.min(3);
    }
}

fn update_op_level(mut clients: Query<(&mut Client, &OpLevel), Changed<OpLevel>>) {
    for (mut client, lvl) in &mut clients.iter_mut() {
        client.write_packet(&EntityStatusS2c {
            entity_id: 0,
            entity_status: 24 + lvl.0,
        });
    }
}
