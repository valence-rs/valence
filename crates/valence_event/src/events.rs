use bevy_app::{App, First, Plugin, PluginGroup, PluginGroupBuilder};
use bevy_ecs::prelude::Event;
use bevy_ecs::schedule::{IntoSystemSetConfigs, SystemSet};
use valence_server::EventLoopPreUpdate;

use self::inventory::InventoryEventPlugin;
use crate::state_event::{EventsWithState, State, States};

pub mod inventory;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum EventDispacherSets {
    MainEvents,
    Checks,
    UserEvents,
}

pub struct EventDispacherPlugin;

impl Plugin for EventDispacherPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            EventLoopPreUpdate,
            (
                EventDispacherSets::MainEvents,
                EventDispacherSets::Checks,
                EventDispacherSets::UserEvents,
            )
                .chain(),
        )
        
        .init_resource::<States>();

    }
}

pub struct EventPlugins;

impl PluginGroup for EventPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(InventoryEventPlugin)
    }
}

trait AddEventWithStateExt {
    fn add_event_with_state<E: Event>(&mut self) -> &mut Self;
}

impl AddEventWithStateExt for App {
    fn add_event_with_state<E: Event>(&mut self) -> &mut Self {
        if !self.world.contains_resource::<EventsWithState<E>>() {
            self.init_resource::<EventsWithState<E>>()
                .add_systems(First, EventsWithState::<E>::update_system);
        }
        // if !self.world.contains_resource::<States>() {
        //     self.init_resource::<States>();
        // }
        self
    }
}
