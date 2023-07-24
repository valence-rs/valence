use std::any::Any;
use std::marker::PhantomData;
use std::ptr::NonNull;

use bevy_ecs::archetype::Archetype;
use bevy_ecs::component::Tick;
use bevy_ecs::prelude::{Entity, EventReader};
use bevy_ecs::system::{In, Query, System, SystemMeta, SystemParam};
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_ecs::world::World;
use glam::DVec3;
use valence_client::message::SendMessage;
use valence_client::Client;
use valence_core::block_pos::BlockPos;
use valence_core::text::Text;

use crate::exec::CommandExecutionEvent;
use crate::nodes::EntityNode;
use crate::parse::ParseResultsRead;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RealCommandExecutor {
    Player(Entity),
    Console,
    Misc(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CommandExecutor {
    pub position: Option<DVec3>,
    pub base: CommandExecutorBase,
}

impl CommandExecutor {
    pub fn node_entity(&self, query: &Query<Option<&EntityNode>>) -> Option<Entity> {
        match self.base {
            CommandExecutorBase::Block { instance, .. } => Some(instance),
            CommandExecutorBase::Console => None,
            CommandExecutorBase::Entity { entity } => Some(entity),
        }
        .and_then(|v| {
            query
                .get(v)
                .expect("The given entity does not exist in the world of this query")
                .map(|v| v.0)
        })
    }
}

#[derive(SystemParam)]
pub struct CommandExecutorBridge<'w, 's> {
    client: Query<'w, 's, &'static mut Client>,
}

impl<'w, 's> CommandExecutorBridge<'w, 's> {
    pub fn send_message(&mut self, executor: RealCommandExecutor, text: Text) {
        match executor {
            RealCommandExecutor::Console => todo!(),
            RealCommandExecutor::Misc(id) => todo!(),
            RealCommandExecutor::Player(entity) => {
                self.client.get_mut(entity).unwrap().send_chat_message(text)
            }
        }
    }
}

impl From<CommandExecutorBase> for CommandExecutor {
    fn from(base: CommandExecutorBase) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum CommandExecutorBase {
    #[default]
    Console,
    Entity {
        entity: Entity,
    },
    Block {
        instance: Entity,
        pos: BlockPos,
    },
}

/// Usage of 'static lifetime is necessary because In argument of bevy can not
/// have any lifetimes. This will be handled correctly
pub type CommandArguments = (
    // lifetime is not 'static but the lifetime of outer function
    ParseResultsRead<'static>,
    RealCommandExecutor,
    // a mutable reference with lifetime of function
    NonNull<CommandExecutor>,
);
