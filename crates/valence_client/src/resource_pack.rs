use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use packet::*;
use valence_core::protocol::encode::WritePacket;
use valence_core::text::Text;

use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};
use crate::Client;

pub(super) fn build(app: &mut App) {
    app.add_event::<ResourcePackStatusEvent>().add_system(
        handle_resource_pack_status
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ResourcePackStatusEvent {
    pub client: Entity,
    pub status: ResourcePackStatus,
}

pub use packet::ResourcePackStatusC2s as ResourcePackStatus;

impl Client {
    /// Requests that the client download and enable a resource pack.
    ///
    /// # Arguments
    /// * `url` - The URL of the resource pack file.
    /// * `hash` - The SHA-1 hash of the resource pack file. Any value other
    ///   than a 40-character hexadecimal string is ignored by the client.
    /// * `forced` - Whether a client should be kicked from the server upon
    ///   declining the pack (this is enforced client-side)
    /// * `prompt_message` - A message to be displayed with the resource pack
    ///   dialog.
    pub fn set_resource_pack(
        &mut self,
        url: &str,
        hash: &str,
        forced: bool,
        prompt_message: Option<Text>,
    ) {
        self.write_packet(&ResourcePackSendS2c {
            url,
            hash,
            forced,
            prompt_message: prompt_message.map(|t| t.into()),
        });
    }
}

fn handle_resource_pack_status(
    mut packets: EventReader<PacketEvent>,
    mut events: EventWriter<ResourcePackStatusEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<ResourcePackStatusC2s>() {
            events.send(ResourcePackStatusEvent {
                client: packet.client,
                status: pkt,
            })
        }
    }
}

pub mod packet {
    use std::borrow::Cow;

    use valence_core::protocol::{packet_id, Decode, Encode, Packet};
    use valence_core::text::Text;

    #[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::RESOURCE_PACK_SEND_S2C)]
    pub struct ResourcePackSendS2c<'a> {
        pub url: &'a str,
        pub hash: &'a str,
        pub forced: bool,
        pub prompt_message: Option<Cow<'a, Text>>,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::RESOURCE_PACK_STATUS_C2S)]
    pub enum ResourcePackStatusC2s {
        /// The client has accepted the server's resource pack.
        SuccessfullyLoaded,
        /// The client has declined the server's resource pack.
        Declined,
        /// The client has successfully loaded the server's resource pack.
        FailedDownload,
        /// The client has failed to download the server's resource pack.
        Accepted,
    }
}
