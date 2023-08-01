use valence_registry::tags::Registry;

use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SYNCHRONIZE_TAGS_S2C)]
pub struct SynchronizeTagsS2c<'a> {
    pub registries: Cow<'a, [Registry]>,
}
