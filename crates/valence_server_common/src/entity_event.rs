use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};

pub trait AddEntityEvent {
    fn add_entity_event<E: Event>(&mut self);
}

impl AddEntityEvent for App {
    fn add_entity_event<E: Event>(&mut self) {
        self.add_systems(Last, clear_entity_events::<E>);
    }
}

fn clear_entity_events<E: Event>(mut events: Query<&mut EntityEvents<E>>) {
    events.iter_mut().for_each(|mut e| e.clear());
}

#[derive(Component, Clone, DerefMut, Deref)]
pub struct EntityEvents<E: Event>(pub Vec<E>);
