use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Update};
use bevy_ecs::schedule::IntoSystemConfigs;
use compile::{compile_commands, CompiledCommandExecutionEvent};
use exec::{command_execution_packet, node_command_execution, CommandExecutionEvent};
use nodes::{send_nodes_to_clients, update_root_nodes, NodeGraphInWorld};
use suggestions::{
    send_calculated_suggestions, suggestions_request_packet, suggestions_spawn_tasks,
    SuggestionsAnswerEvent, SuggestionsQueue, SuggestionsRequestEvent, SuggestionsTokioRuntime,
};

pub mod boolean;
pub mod builder;
pub mod command;
pub mod compile;
pub mod exec;
pub mod nodes;
pub mod nums;
pub mod parse;
pub mod pkt;
pub mod reader;
pub mod suggestions;

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                command_execution_packet,
                compile_commands.after(command_execution_packet),
                suggestions_request_packet,
            ),
        )
        .add_systems(Update, (node_command_execution, suggestions_spawn_tasks))
        .add_systems(
            PostUpdate,
            (
                update_root_nodes,
                send_nodes_to_clients.after(update_root_nodes),
                send_calculated_suggestions,
            ),
        )
        .init_resource::<NodeGraphInWorld>()
        .init_resource::<SuggestionsTokioRuntime>()
        .init_resource::<SuggestionsQueue>()
        .add_event::<CommandExecutionEvent>()
        .add_event::<CompiledCommandExecutionEvent>()
        .add_event::<SuggestionsRequestEvent>()
        .add_event::<SuggestionsAnswerEvent>();
    }
}
