use std::collections::HashSet;
use std::fmt;
use std::net::IpAddr;

use bevy_ecs::prelude::*;
use tokio::sync::OwnedSemaphorePermit;
use tracing::warn;
use uuid::Uuid;
use valence_protocol::packets::s2c::play::DisconnectPlay;
use valence_protocol::{EncodePacket, Username};

use crate::server::{NewClientInfo, PlayPacketReceiver, PlayPacketSender, Server};

pub mod event;

#[derive(Component)]
pub struct Client {
    /// Is `None` when the client is disconnected.
    send: Option<PlayPacketSender>,
    recv: PlayPacketReceiver,
    /// Ensures that we don't allow more connections to the server until the
    /// client is dropped.
    _permit: OwnedSemaphorePermit,
    username: Username<String>,
    uuid: Uuid,
    ip: IpAddr,
    /*
    /// To make sure we're not loading already loaded chunks, or unloading
    /// unloaded chunks.
    #[cfg(debug_assertions)]
    loaded_chunks: HashSet<ChunkPos>,
    */
}

impl Client {
    pub(crate) fn new(
        send: PlayPacketSender,
        recv: PlayPacketReceiver,
        permit: OwnedSemaphorePermit,
        info: NewClientInfo,
    ) -> Self {
        Self {
            send: Some(send),
            recv,
            _permit: permit,
            username: info.username,
            uuid: info.uuid,
            ip: info.ip,
        }
    }

    /// Attempts to write a play packet into this client's packet buffer. The
    /// packet will be sent at the end of the tick.
    ///
    /// If encoding the packet fails, the client is disconnected. Has no
    /// effect if the client is already disconnected.
    pub fn write_packet<P>(&mut self, pkt: &P)
    where
        P: EncodePacket + fmt::Debug + ?Sized,
    {
        if let Some(send) = &mut self.send {
            if let Err(e) = send.append_packet(pkt) {
                warn!(
                    username = %self.username,
                    uuid = %self.uuid,
                    ip = %self.ip,
                    "failed to write packet: {e:#}"
                );
                self.send = None;
            }
        }
    }

    /// Writes arbitrary bytes to this client's packet buffer. Don't use this
    /// function unless you know what you're doing. Consider using
    /// [`write_packet`] instead.
    ///
    /// [`write_packet`]: Self::write_packet
    pub fn write_packet_bytes(&mut self, bytes: &[u8]) {
        if let Some(send) = &mut self.send {
            send.append_bytes(bytes);
        }
    }

    /// Gets the username of this client.
    pub fn username(&self) -> Username<&str> {
        self.username.as_str_username()
    }

    /// Gets the UUID of this client.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Gets the IP address of this client.
    pub fn ip(&self) -> IpAddr {
        self.ip
    }

    /// Gets whether or not the client is connected to the server.
    ///
    /// A disconnected client component will never become reconnected. It is
    /// your responsibility to despawn disconnected client entities, since
    /// they will not be automatically despawned by Valence.
    pub fn is_disconnected(&self) -> bool {
        self.send.is_none()
    }
}

/// The system for updating clients.
pub(crate) fn update_clients(
    mut clients: Query<(&mut Client, ChangeTrackers<Client>)>,
    server: Res<Server>,
) {
    // TODO: what batch size to use?
    clients.par_for_each_mut(1, |(mut client, change)| {
        if let Some(mut send) = client.send.take() {
            match update_client(&mut client, change, &server) {
                Ok(()) => client.send = Some(send),
                Err(e) => {
                    let _ = send.append_packet(&DisconnectPlay { reason: "".into() });
                    warn!(
                        username = %client.username,
                        uuid = %client.uuid,
                        ip = %client.ip,
                        "error updating client: {e:#}"
                    );
                }
            }
        }
    });
}

fn update_client(
    client: &mut Client,
    change: ChangeTrackers<Client>,
    server: &Server,
) -> anyhow::Result<()> {
    // Send the login (play) packet and other initial packets. We defer this until
    // now so that the user can set the client's initial location, game
    // mode, etc.
    if change.is_added() {


        /*
        // TODO: enable all the features?
        send.append_packet(&FeatureFlags {
            features: vec![Ident::new("vanilla").unwrap()],
        })?;
        */

        // TODO: write player list init packets.
    }

    todo!()
}
