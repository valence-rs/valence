use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Update};
use bevy_ecs::schedule::IntoSystemConfigs;
use compile::{compile_commands, CompiledCommandExecutionEvent};
use exec::{command_execution_packet, node_command_execution, CommandExecutionEvent};
use nodes::{send_nodes_to_clients, update_root_nodes};

pub mod command;
pub mod compile;
pub mod entity;
pub mod exec;
pub mod nodes;
pub mod parse;
pub mod pkt;
pub mod reader;
pub mod suggestions;
pub mod world;
pub mod boolean;
pub mod nums;

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                command_execution_packet,
                compile_commands.after(command_execution_packet),
            ),
        )
        .add_systems(Update, node_command_execution)
        .add_systems(
            PostUpdate,
            (
                update_root_nodes,
                send_nodes_to_clients.after(update_root_nodes),
            ),
        )
        .add_event::<CommandExecutionEvent>()
        .add_event::<CompiledCommandExecutionEvent>();
    }
}
