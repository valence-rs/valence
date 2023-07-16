#![allow(clippy::type_complexity)]

use std::net::SocketAddr;

use rand::Rng;
use valence::network::{
    async_trait, BroadcastToLan, CleanupFn, ConnectionMode, PlayerSampleEntry, ServerListPing,
};
use valence::prelude::*;
use valence_core::MINECRAFT_VERSION;
use valence_network::HandshakeData;

pub fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            callbacks: MyCallbacks.into(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .run();
}

struct MyCallbacks;

#[async_trait]
impl NetworkCallbacks for MyCallbacks {
    async fn server_list_ping(
        &self,
        _shared: &SharedNetworkState,
        remote_addr: SocketAddr,
        handshake_data: &HandshakeData,
    ) -> ServerListPing {
        let max_players = 420;

        ServerListPing::Respond {
            online_players: rand::thread_rng().gen_range(0..=max_players),
            max_players,
            player_sample: vec![PlayerSampleEntry {
                name: "foobar".into(),
                id: Uuid::from_u128(12345),
            }],
            description: "Your IP address is ".into_text()
                + remote_addr.to_string().color(Color::DARK_GRAY),
            favicon_png: include_bytes!("../assets/logo-64x64.png"),
            version_name: ("Valence ".color(Color::GOLD) + MINECRAFT_VERSION.color(Color::RED))
                .to_legacy_lossy(),
            protocol: handshake_data.protocol_version,
        }
    }

    async fn broadcast_to_lan(&self, _shared: &SharedNetworkState) -> BroadcastToLan {
        BroadcastToLan::Enabled("Hello Valence!".into())
    }

    async fn login(
        &self,
        _shared: &SharedNetworkState,
        _info: &NewClientInfo,
    ) -> Result<CleanupFn, Text> {
        Err("You are not meant to join this example".color(Color::RED))
    }
}
