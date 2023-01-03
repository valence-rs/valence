pub mod event;

use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct Client {

}

impl Client {
    pub(crate) fn new() -> Self {
        Self {

        }
    }
}