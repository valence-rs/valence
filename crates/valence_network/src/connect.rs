//! Handles new connections to the server and the log-in process.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{anyhow, bail, ensure, Context};
use base64::prelude::*;
use hmac::digest::Update;
use hmac::{Hmac, Mac};
use num_bigint::BigInt;
use reqwest::StatusCode;
use rsa::PaddingScheme;
use serde::Deserialize;
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, trace, warn};
use uuid::Uuid;
use valence_client::is_valid_username;
use valence_core::packet::c2s::handshake::handshake::NextState;
use valence_core::packet::c2s::handshake::HandshakeC2s;
use valence_core::packet::c2s::login::{LoginHelloC2s, LoginKeyC2s, LoginQueryResponseC2s};
use valence_core::packet::c2s::status::{QueryPingC2s, QueryRequestC2s};
use valence_core::packet::decode::PacketDecoder;
use valence_core::packet::encode::PacketEncoder;
use valence_core::packet::raw::RawBytes;
use valence_core::packet::s2c::login::{
    LoginCompressionS2c, LoginDisconnectS2c, LoginHelloS2c, LoginQueryRequestS2c, LoginSuccessS2c,
};
use valence_core::packet::s2c::status::{QueryPongS2c, QueryResponseS2c};
use valence_core::packet::var_int::VarInt;
use valence_core::packet::Decode;
use valence_core::property::Property;
use valence_core::text::Text;
use valence_core::{ident, translation_key, MINECRAFT_VERSION, PROTOCOL_VERSION};

use crate::packet_io::PacketIo;
use crate::{CleanupOnDrop, ConnectionMode, NewClientInfo, ServerListPing, SharedNetworkState};

/// Accepts new connections to the server as they occur.
pub(super) async fn do_accept_loop(shared: SharedNetworkState) {
    let listener = match TcpListener::bind(shared.0.address).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("failed to start TCP listener: {e}");
            return;
        }
    };

    loop {
        match shared.0.connection_sema.clone().acquire_owned().await {
            Ok(permit) => match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let shared = shared.clone();

                    tokio::spawn(async move {
                        handle_connection(shared, stream, remote_addr).await;
                        drop(permit);
                    });
                }
                Err(e) => {
                    error!("failed to accept incoming connection: {e}");
                }
            },
            // Closed semaphore indicates server shutdown.
            Err(_) => return,
        }
    }
}

async fn handle_connection(shared: SharedNetworkState, stream: TcpStream, remote_addr: SocketAddr) {
    trace!("handling connection");

    if let Err(e) = stream.set_nodelay(true) {
        error!("failed to set TCP_NODELAY: {e}");
    }

    let conn = PacketIo::new(
        stream,
        PacketEncoder::new(),
        PacketDecoder::new(),
        Duration::from_secs(5),
    );

    // TODO: peek stream for 0xFE legacy ping

    if let Err(e) = handle_handshake(shared, conn, remote_addr).await {
        // EOF can happen if the client disconnects while joining, which isn't
        // very erroneous.
        if let Some(e) = e.downcast_ref::<io::Error>() {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                return;
            }
        }
        warn!("connection ended with error: {e:#}");
    }
}

struct HandshakeData {
    protocol_version: i32,
    server_address: String,
    next_state: NextState,
}

async fn handle_handshake(
    shared: SharedNetworkState,
    mut io: PacketIo,
    remote_addr: SocketAddr,
) -> anyhow::Result<()> {
    let handshake = io.recv_packet::<HandshakeC2s>().await?;

    let handshake = HandshakeData {
        protocol_version: handshake.protocol_version.0,
        server_address: handshake.server_address.to_owned(),
        next_state: handshake.next_state,
    };

    ensure!(
        matches!(&shared.0.connection_mode, ConnectionMode::BungeeCord)
            || handshake.server_address.chars().count() <= 255,
        "handshake server address is too long"
    );

    match handshake.next_state {
        NextState::Status => handle_status(shared, io, remote_addr, handshake)
            .await
            .context("error handling status"),
        NextState::Login => {
            match handle_login(&shared, &mut io, remote_addr, handshake)
                .await
                .context("error handling login")?
            {
                Some((info, cleanup)) => {
                    let client = io.into_client_args(
                        info,
                        shared.0.incoming_byte_limit,
                        shared.0.outgoing_byte_limit,
                        cleanup,
                    );

                    let _ = shared.0.new_clients_send.send_async(client).await;

                    Ok(())
                }
                None => Ok(()),
            }
        }
    }
}

async fn handle_status(
    shared: SharedNetworkState,
    mut io: PacketIo,
    remote_addr: SocketAddr,
    handshake: HandshakeData,
) -> anyhow::Result<()> {
    io.recv_packet::<QueryRequestC2s>().await?;

    match shared
        .0
        .callbacks
        .inner
        .server_list_ping(&shared, remote_addr, handshake.protocol_version)
        .await
    {
        ServerListPing::Respond {
            online_players,
            max_players,
            player_sample,
            description,
            favicon_png,
        } => {
            let mut json = json!({
                "version": {
                    "name": MINECRAFT_VERSION,
                    "protocol": PROTOCOL_VERSION
                },
                "players": {
                    "online": online_players,
                    "max": max_players,
                    "sample": player_sample,
                },
                "description": description,
            });

            if !favicon_png.is_empty() {
                let mut buf = "data:image/png;base64,".to_owned();
                BASE64_STANDARD.encode_string(favicon_png, &mut buf);
                json["favicon"] = Value::String(buf);
            }

            io.send_packet(&QueryResponseS2c {
                json: &json.to_string(),
            })
            .await?;
        }
        ServerListPing::Ignore => return Ok(()),
    }

    let QueryPingC2s { payload } = io.recv_packet().await?;

    io.send_packet(&QueryPongS2c { payload }).await?;

    Ok(())
}

/// Handle the login process and return the new client's data if successful.
async fn handle_login(
    shared: &SharedNetworkState,
    conn: &mut PacketIo,
    remote_addr: SocketAddr,
    handshake: HandshakeData,
) -> anyhow::Result<Option<(NewClientInfo, CleanupOnDrop)>> {
    if handshake.protocol_version != PROTOCOL_VERSION {
        // TODO: send translated disconnect msg.
        return Ok(None);
    }

    let LoginHelloC2s {
        username,
        profile_id: _, // TODO
    } = conn.recv_packet().await?;

    ensure!(is_valid_username(username), "invalid username");

    let username = username.to_owned();

    let info = match shared.connection_mode() {
        ConnectionMode::Online { .. } => login_online(shared, conn, remote_addr, username).await?,
        ConnectionMode::Offline => login_offline(remote_addr, username)?,
        ConnectionMode::BungeeCord => login_bungeecord(&handshake.server_address, username)?,
        ConnectionMode::Velocity { secret } => login_velocity(conn, username, secret).await?,
    };

    if let Some(threshold) = shared.0.compression_threshold {
        conn.send_packet(&LoginCompressionS2c {
            threshold: VarInt(threshold as i32),
        })
        .await?;

        conn.set_compression(Some(threshold));
    }

    let cleanup = match shared.0.callbacks.inner.login(shared, &info).await {
        Ok(f) => CleanupOnDrop(Some(f)),
        Err(reason) => {
            info!("disconnect at login: \"{reason}\"");
            conn.send_packet(&LoginDisconnectS2c {
                reason: reason.into(),
            })
            .await?;
            return Ok(None);
        }
    };

    conn.send_packet(&LoginSuccessS2c {
        uuid: info.uuid,
        username: &info.username,
        properties: Default::default(),
    })
    .await?;

    Ok(Some((info, cleanup)))
}

/// Login procedure for online mode.
async fn login_online(
    shared: &SharedNetworkState,
    conn: &mut PacketIo,
    remote_addr: SocketAddr,
    username: String,
) -> anyhow::Result<NewClientInfo> {
    let my_verify_token: [u8; 16] = rand::random();

    conn.send_packet(&LoginHelloS2c {
        server_id: "", // Always empty
        public_key: &shared.0.public_key_der,
        verify_token: &my_verify_token,
    })
    .await?;

    let LoginKeyC2s {
        shared_secret,
        verify_token: encrypted_verify_token,
    } = conn.recv_packet().await?;

    let shared_secret = shared
        .0
        .rsa_key
        .decrypt(PaddingScheme::PKCS1v15Encrypt, shared_secret)
        .context("failed to decrypt shared secret")?;

    let verify_token = shared
        .0
        .rsa_key
        .decrypt(PaddingScheme::PKCS1v15Encrypt, encrypted_verify_token)
        .context("failed to decrypt verify token")?;

    ensure!(
        my_verify_token.as_slice() == verify_token,
        "verify tokens do not match"
    );

    let crypt_key: [u8; 16] = shared_secret
        .as_slice()
        .try_into()
        .context("shared secret has the wrong length")?;

    conn.enable_encryption(&crypt_key)?;

    let hash = Sha1::new()
        .chain(&shared_secret)
        .chain(&shared.0.public_key_der)
        .finalize();

    let url = shared
        .0
        .callbacks
        .inner
        .session_server(
            shared,
            username.as_str(),
            &auth_digest(&hash),
            &remote_addr.ip(),
        )
        .await;

    let resp = shared.0.http_client.get(url).send().await?;

    match resp.status() {
        StatusCode::OK => {}
        StatusCode::NO_CONTENT => {
            let reason = Text::translate(
                translation_key::MULTIPLAYER_DISCONNECT_UNVERIFIED_USERNAME,
                [],
            );
            conn.send_packet(&LoginDisconnectS2c {
                reason: reason.into(),
            })
            .await?;
            bail!("session server could not verify username");
        }
        status => {
            bail!("session server GET request failed (status code {status})");
        }
    }

    #[derive(Debug, Deserialize)]
    struct GameProfile {
        id: Uuid,
        name: String,
        properties: Vec<Property>,
    }

    let profile: GameProfile = resp.json().await.context("parsing game profile")?;

    ensure!(
        is_valid_username(&profile.name),
        "invalid game profile username"
    );

    ensure!(profile.name == username, "usernames do not match");

    Ok(NewClientInfo {
        uuid: profile.id,
        username,
        ip: remote_addr.ip(),
        properties: profile.properties.into(),
    })
}

fn auth_digest(bytes: &[u8]) -> String {
    BigInt::from_signed_bytes_be(bytes).to_str_radix(16)
}

/// Login procedure for offline mode.
fn login_offline(remote_addr: SocketAddr, username: String) -> anyhow::Result<NewClientInfo> {
    Ok(NewClientInfo {
        // Derive the client's UUID from a hash of their username.
        uuid: Uuid::from_slice(&Sha256::digest(username.as_str())[..16])?,
        username,
        properties: vec![].into(),
        ip: remote_addr.ip(),
    })
}

/// Login procedure for BungeeCord.
fn login_bungeecord(server_address: &str, username: String) -> anyhow::Result<NewClientInfo> {
    // Get data from server_address field of the handshake
    let [_, client_ip, uuid, properties]: [&str; 4] = server_address
        .split('\0')
        .take(4)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow!("malformed BungeeCord server address data"))?;

    // Read properties and get textures
    let properties: Vec<Property> =
        serde_json::from_str(properties).context("failed to parse BungeeCord player properties")?;

    Ok(NewClientInfo {
        uuid: uuid.parse()?,
        username,
        properties: properties.into(),
        ip: client_ip.parse()?,
    })
}

/// Login procedure for Velocity.
async fn login_velocity(
    io: &mut PacketIo,
    username: String,
    velocity_secret: &str,
) -> anyhow::Result<NewClientInfo> {
    const VELOCITY_MIN_SUPPORTED_VERSION: u8 = 1;
    const VELOCITY_MODERN_FORWARDING_WITH_KEY_V2: i32 = 3;

    let message_id: i32 = 0; // TODO: make this random?

    // Send Player Info Request into the Plugin Channel
    io.send_packet(&LoginQueryRequestS2c {
        message_id: VarInt(message_id),
        channel: ident!("velocity:player_info").into(),
        data: RawBytes(&[VELOCITY_MIN_SUPPORTED_VERSION]),
    })
    .await?;

    // Get Response
    let plugin_response: LoginQueryResponseC2s = io.recv_packet().await?;

    ensure!(
        plugin_response.message_id.0 == message_id,
        "mismatched plugin response ID (got {}, expected {message_id})",
        plugin_response.message_id.0,
    );

    let data = plugin_response
        .data
        .context("missing plugin response data")?
        .0;

    ensure!(data.len() >= 32, "invalid plugin response data length");
    let (signature, mut data_without_signature) = data.split_at(32);

    // Verify signature
    let mut mac = Hmac::<Sha256>::new_from_slice(velocity_secret.as_bytes())?;
    Mac::update(&mut mac, data_without_signature);
    mac.verify_slice(signature)?;

    // Check Velocity version
    let version = VarInt::decode(&mut data_without_signature)
        .context("failed to decode velocity version")?
        .0;

    // Get client address
    let remote_addr = String::decode(&mut data_without_signature)?.parse()?;

    // Get UUID
    let uuid = Uuid::decode(&mut data_without_signature)?;

    // Get username and validate
    ensure!(
        username == <&str>::decode(&mut data_without_signature)?,
        "mismatched usernames"
    );

    // Read game profile properties
    let properties = Vec::<Property>::decode(&mut data_without_signature)
        .context("decoding velocity game profile properties")?;

    if version >= VELOCITY_MODERN_FORWARDING_WITH_KEY_V2 {
        // TODO
    }

    Ok(NewClientInfo {
        uuid,
        username,
        properties: properties.into(),
        ip: remote_addr,
    })
}

#[cfg(test)]
mod tests {
    use sha1::Digest;

    use super::*;

    #[test]
    fn auth_digest_usernames() {
        assert_eq!(
            auth_digest(&Sha1::digest("Notch")),
            "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48"
        );
        assert_eq!(
            auth_digest(&Sha1::digest("jeb_")),
            "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1"
        );
        assert_eq!(
            auth_digest(&Sha1::digest("simon")),
            "88e16a1019277b15d58faf0541e11910eb756f6"
        );
    }
}
