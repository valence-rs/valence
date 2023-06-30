use bevy_app::{App, Plugin, PreUpdate, PostUpdate};
use bevy_ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfig, SystemSet};
use command::{check_command_not_found, command_event, CommandExecutionEvent, CommandStorage};
use node::{update_client_nodes, update_nodes};
use parse::CompletedSuggestionEvent;

pub mod boolean;
pub mod command;
pub mod node;
pub mod packet;
pub mod parse;
pub mod reader;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CommandEventSet;

pub struct CommandAPIPlugin;

impl Plugin for CommandAPIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandStorage>()
            .add_event::<CommandExecutionEvent>()
            .add_event::<CompletedSuggestionEvent>()
            .configure_set(PreUpdate, CommandEventSet)
            .add_systems(
                PreUpdate,
                (
                    command_event.in_set(CommandEventSet),
                    check_command_not_found.after(command_event),
                ),
            )
            .add_systems(PostUpdate, (update_nodes, update_client_nodes));
    }
}
