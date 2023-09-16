use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_protocol::packets::play::game_state_change_s2c::GameEventKind;
use valence_protocol::packets::play::GameStateChangeS2c;
pub use valence_protocol::GameMode;
use valence_protocol::WritePacket;

use crate::client::FlushPacketsSet;
use crate::Client;

pub struct UpdateGameModePlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateGameModeSet;

impl Plugin for UpdateGameModePlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(PostUpdate, UpdateGameModeSet.before(FlushPacketsSet))
            .add_systems(PostUpdate, update_game_mode.in_set(UpdateGameModeSet));
    }
}

pub(crate) fn update_game_mode(mut clients: Query<(&mut Client, &GameMode), Changed<GameMode>>) {
    for (mut client, game_mode) in &mut clients {
        if client.is_added() {
            // Game join packet includes the initial game mode.
            continue;
        }

        client.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ChangeGameMode,
            value: *game_mode as i32 as f32,
        });
    }
}
