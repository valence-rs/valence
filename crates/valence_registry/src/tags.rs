use std::borrow::Cow;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_protocol::encode::{PacketWriter, WritePacket};
pub use valence_protocol::packets::play::synchronize_tags_s2c::Registry;
use valence_protocol::packets::play::SynchronizeTagsS2c;
use valence_server_common::Server;

use crate::RegistrySet;

pub(super) fn build(app: &mut App) {
    app.init_resource::<TagsRegistry>()
        .add_systems(PreStartup, init_tags_registry)
        .add_systems(PostUpdate, cache_tags_packet.in_set(RegistrySet));
}

#[derive(Debug, Resource, Default)]
pub struct TagsRegistry {
    pub registries: Vec<Registry>,
    cached_packet: Vec<u8>,
}

impl TagsRegistry {
    fn build_synchronize_tags(&self) -> SynchronizeTagsS2c {
        SynchronizeTagsS2c {
            registries: Cow::Borrowed(&self.registries),
        }
    }

    pub fn sync_tags_packet(&self) -> &Vec<u8> {
        &self.cached_packet
    }
}

fn init_tags_registry(mut tags: ResMut<TagsRegistry>) {
    let registries = serde_json::from_str::<Vec<Registry>>(include_str!("../extracted/tags.json"))
        .expect("tags.json is invalid");
    tags.registries = registries;
}

pub(crate) fn cache_tags_packet(server: Res<Server>, tags: ResMut<TagsRegistry>) {
    if tags.is_changed() {
        let tags = tags.into_inner();
        let packet = tags.build_synchronize_tags();
        let mut bytes = vec![];
        let mut writer = PacketWriter::new(&mut bytes, server.compression_threshold());
        writer.write_packet(&packet);
        tags.cached_packet = bytes;
    }
}

#[cfg(test)]
mod tests {
    /* TODO: move this to src/tests/
    #[test]
    fn smoke_test() {
        let mut app = bevy_app::App::new();
        app.add_plugins(RegistryPlugin);
        // app.insert_resource(Server::default());
        app.update();

        let tags_registry = app.world.get_resource::<TagsRegistry>().unwrap();
        let packet = tags_registry.build_synchronize_tags();
        assert!(!packet.registries.is_empty());
        assert!(!tags_registry.cached_packet.is_empty());
    }
    */
}
