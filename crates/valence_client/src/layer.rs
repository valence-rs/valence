use std::collections::HashSet;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::FlushPacketsSet;

pub(super) fn build(app: &mut App) {
    app.add_system(
        update_old_client_layer_set.in_set(FlushPacketsSet),
    );
}

/// A component that represents a layers that an client is on.
#[derive(Component, Default, Debug)]
pub struct ClientLayerSet(pub HashSet<u8>, HashSet<u8>);

impl ClientLayerSet {
    pub fn new(layers: Vec<u8>) -> Self {
        Self(layers.into_iter().collect(), HashSet::new())
    }

    pub fn set(&mut self, layer: u8, visibility: bool) -> bool {
        if visibility {
            self.0.insert(layer)
        } else {
            self.0.remove(&layer)
        }
    }

    pub fn get(&self, layer: u8) -> bool {
        self.0.contains(&layer)
    }

    pub fn toggle(&mut self, layer: u8) -> bool {
        if self.0.contains(&layer) {
            self.0.remove(&layer)
        } else {
            self.0.insert(layer)
        }
    }

    pub fn update(&mut self) {
        self.1 = self.0.clone();
    }

    pub fn get_added(&self) -> impl Iterator<Item = &u8> {
        self.0.difference(&self.1)
    }

    pub fn get_removed(&self) -> impl Iterator<Item = &u8> {
        self.1.difference(&self.0)
    }
}

fn update_old_client_layer_set(mut client_layer_set: Query<&mut ClientLayerSet>) {
    for mut client_layer_set in client_layer_set.iter_mut() {
        client_layer_set.update();
    }
}