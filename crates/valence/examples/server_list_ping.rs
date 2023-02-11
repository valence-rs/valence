use std::net::SocketAddr;

use valence::prelude::*;

pub fn main() {
    App::new()
        .add_plugin(ServerPlugin::new(MyCallbacks).with_connection_mode(ConnectionMode::Offline))
        .run();
}

struct MyCallbacks;

#[async_trait]
impl AsyncCallbacks for MyCallbacks {
    async fn server_list_ping(
        &self,
        _shared: &SharedServer,
        remote_addr: SocketAddr,
        _protocol_version: i32,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: 42,
            max_players: 420,
            player_sample: vec![PlayerSampleEntry {
                name: "foobar".into(),
                id: Uuid::from_u128(12345),
            }],
            description: "Your IP address is ".into_text()
                + remote_addr.to_string().color(Color::GOLD),
            favicon_png: include_bytes!("../../../assets/logo-64x64.png"),
        }
    }

    async fn login(&self, _shared: &SharedServer, _info: &NewClientInfo) -> Result<(), Text> {
        Err("You are not meant to join this example".color(Color::RED))
    }
}
