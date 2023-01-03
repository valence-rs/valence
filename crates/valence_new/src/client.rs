use std::collections::HashSet;
use std::net::IpAddr;

use bevy_ecs::prelude::*;
use tokio::sync::OwnedSemaphorePermit;
use uuid::Uuid;
use valence_protocol::Username;

use crate::server::{NewClientInfo, PlayPacketReceiver, PlayPacketSender};

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
            ip: info.ip
        }
    }
}

/// The system for updating clients.
///
/// This is responsible for ...
pub(crate) fn update_clients_system() {

}
