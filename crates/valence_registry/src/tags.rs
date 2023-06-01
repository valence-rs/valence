use std::borrow::Cow;

use bevy_ecs::prelude::*;
use serde::Deserialize;
use valence_core::ident::Ident;
use valence_core::protocol::encode::{PacketWriter, WritePacket};
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_core::Server;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SYNCHRONIZE_TAGS_S2C)]
pub struct SynchronizeTagsS2c<'a> {
    pub registries: Cow<'a, [Registry<'a>]>,
}

#[derive(Debug, Resource)]
pub struct TagsRegistry<'a> {
    pub registries: Vec<Registry<'a>>,
    cached_packet: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Registry<'a> {
    pub registry: Ident<Cow<'a, str>>,
    pub tags: Vec<TagEntry<'a>>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Encode, Decode)]
pub struct TagEntry<'a> {
    pub name: Ident<Cow<'a, str>>,
    pub entries: Vec<u64>,
}

impl<'a> TagsRegistry<'a> {
    pub(crate) fn build_synchronize_tags(&'a self) -> SynchronizeTagsS2c<'a> {
        SynchronizeTagsS2c {
            registries: Cow::Borrowed(&self.registries),
        }
    }

    pub fn sync_tags_packet(&self) -> &Vec<u8> {
        &self.cached_packet
    }
}

pub(crate) fn init_tags_registry(mut tags: ResMut<TagsRegistry<'static>>) {
    let registries =
        serde_json::from_str::<Vec<Registry>>(include_str!("../../../extracted/tags.json"))
            .expect("tags.json is invalid");
    tags.registries = registries.into();
}

pub(crate) fn cache_tags_packet(server: Res<Server>, tags: ResMut<TagsRegistry<'static>>) {
    if tags.is_changed() {
        let tags = tags.into_inner();
        let packet = tags.build_synchronize_tags();
        let mut bytes = vec![];
        let mut scratch = vec![];
        let mut writer =
            PacketWriter::new(&mut bytes, server.compression_threshold(), &mut scratch);
        writer.write_packet(&packet);
        tags.cached_packet = bytes;
    }
}

impl Default for TagsRegistry<'_> {
    fn default() -> Self {
        Self {
            registries: Default::default(),
            cached_packet: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPlugin;

    #[test]
    fn smoke_test() {
        let mut app = bevy_app::App::new();
        app.add_plugin(RegistryPlugin);
        app.insert_resource(Server::default());
        app.update();

        let tags_registry = app.world.get_resource::<TagsRegistry>().unwrap();
        let packet = tags_registry.build_synchronize_tags();
        assert!(!packet.registries.is_empty());
        assert!(!tags_registry.cached_packet.is_empty());
    }
}
