//! Contains login procedures for the different [`ConnectionMode`]s.
//!
//! [`ConnectionMode`]: crate::config::ConnectionMode

use std::net::SocketAddr;

use anyhow::{anyhow, bail, ensure, Context};
use hmac::digest::Update;
use hmac::{Hmac, Mac};
use num::BigInt;
use reqwest::StatusCode;
use rsa::PaddingScheme;
use serde::Deserialize;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use uuid::Uuid;
use valence_protocol::ident::Ident;
use valence_protocol::packets::c2s::login::{EncryptionResponse, LoginPluginResponse};
use valence_protocol::packets::s2c::login::{
    DisconnectLogin, EncryptionRequest, LoginPluginRequest,
};
use valence_protocol::raw_bytes::RawBytes;
use valence_protocol::text::Text;
use valence_protocol::types::{MsgSigOrVerifyToken, SignedProperty, SignedPropertyOwned};
use valence_protocol::username::Username;
use valence_protocol::var_int::VarInt;
use valence_protocol::Decode;

use crate::config::Config;
use crate::player_textures::SignedPlayerTextures;
use crate::server::packet_controller::InitialPacketController;
use crate::server::{NewClientData, SharedServer};

/// Login sequence for
/// [`ConnectionMode::Online`](crate::config::ConnectionMode).
pub(super) async fn online(
    server: &SharedServer<impl Config>,
    ctrl: &mut InitialPacketController<OwnedReadHalf, OwnedWriteHalf>,
    remote_addr: SocketAddr,
    username: Username<String>,
) -> anyhow::Result<NewClientData> {
    let my_verify_token: [u8; 16] = rand::random();

    ctrl.send_packet(&EncryptionRequest {
        server_id: "", // Always empty
        public_key: &server.0.public_key_der,
        verify_token: &my_verify_token,
    })
    .await?;

    let EncryptionResponse {
        shared_secret,
        sig_or_token,
    } = ctrl.recv_packet().await?;

    let shared_secret = server
        .0
        .rsa_key
        .decrypt(PaddingScheme::PKCS1v15Encrypt, shared_secret)
        .context("failed to decrypt shared secret")?;

    match sig_or_token {
        MsgSigOrVerifyToken::VerifyToken(encrypted_verify_token) => {
            let verify_token = server
                .0
                .rsa_key
                .decrypt(PaddingScheme::PKCS1v15Encrypt, &encrypted_verify_token)
                .context("failed to decrypt verify token")?;

            ensure!(
                my_verify_token.as_slice() == verify_token,
                "verify tokens do not match"
            );
        }
        MsgSigOrVerifyToken::MsgSig { .. } => {}
    };

    let crypt_key: [u8; 16] = shared_secret
        .as_slice()
        .try_into()
        .context("shared secret has the wrong length")?;

    ctrl.enable_encryption(&crypt_key);

    let hash = Sha1::new()
        .chain(&shared_secret)
        .chain(&server.0.public_key_der)
        .finalize();

    let url = server.config().session_server(
        server,
        username.as_str_username(),
        &auth_digest(&hash),
        &remote_addr.ip(),
    );

    let resp = server.0.http_client.get(url).send().await?;

    match resp.status() {
        StatusCode::OK => {}
        StatusCode::NO_CONTENT => {
            let reason = Text::translate("multiplayer.disconnect.unverified_username");
            ctrl.send_packet(&DisconnectLogin { reason }).await?;
            bail!("session server could not verify username");
        }
        status => {
            bail!("session server GET request failed (status code {status})");
        }
    }

    #[derive(Debug, Deserialize)]
    struct AuthResponse {
        id: String,
        name: Username<String>,
        properties: Vec<SignedPropertyOwned>,
    }

    let data: AuthResponse = resp.json().await?;

    ensure!(data.name == username, "usernames do not match");

    let uuid = Uuid::parse_str(&data.id).context("failed to parse player's UUID")?;

    let textures = match data.properties.into_iter().find(|p| p.name == "textures") {
        Some(p) => SignedPlayerTextures::from_base64(
            p.value,
            p.signature.context("missing signature for textures")?,
        )?,
        None => bail!("failed to find textures in auth response"),
    };

    Ok(NewClientData {
        uuid,
        username,
        textures: Some(textures),
        remote_addr: remote_addr.ip(),
    })
}

/// Login sequence for
/// [`ConnectionMode::Offline`](crate::config::ConnectionMode).
pub(super) fn offline(
    remote_addr: SocketAddr,
    username: Username<String>,
) -> anyhow::Result<NewClientData> {
    Ok(NewClientData {
        // Derive the client's UUID from a hash of their username.
        uuid: Uuid::from_slice(&Sha256::digest(username.as_str())[..16])?,
        username,
        textures: None,
        remote_addr: remote_addr.ip(),
    })
}

/// Login sequence for
/// [`ConnectionMode::BungeeCord`](crate::config::ConnectionMode).
pub(super) fn bungeecord(
    server_address: &str,
    username: Username<String>,
) -> anyhow::Result<NewClientData> {
    // Get data from server_address field of the handshake
    let [_, client_ip, uuid, properties]: [&str; 4] = server_address
        .split('\0')
        .take(4)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow!("malformed BungeeCord server address data"))?;

    // Read properties and get textures
    let properties: Vec<SignedProperty> =
        serde_json::from_str(properties).context("failed to parse BungeeCord player properties")?;

    let mut textures = None;
    for prop in properties {
        if prop.name == "textures" {
            textures = Some(
                SignedPlayerTextures::from_base64(
                    prop.value,
                    prop.signature
                        .context("missing player textures signature")?,
                )
                .context("failed to parse signed player textures")?,
            );
            break;
        }
    }

    Ok(NewClientData {
        uuid: uuid.parse()?,
        username,
        textures,
        remote_addr: client_ip.parse()?,
    })
}

fn auth_digest(bytes: &[u8]) -> String {
    BigInt::from_signed_bytes_be(bytes).to_str_radix(16)
}

pub(super) async fn velocity(
    ctrl: &mut InitialPacketController<OwnedReadHalf, OwnedWriteHalf>,
    username: Username<String>,
    velocity_secret: &str,
) -> anyhow::Result<NewClientData> {
    const VELOCITY_MIN_SUPPORTED_VERSION: u8 = 1;
    const VELOCITY_MODERN_FORWARDING_WITH_KEY_V2: i32 = 3;

    let message_id = 0;

    // Send Player Info Request into the Plugin Channel
    ctrl.send_packet(&LoginPluginRequest {
        message_id: VarInt(message_id),
        channel: Ident::new("velocity:player_info").unwrap(),
        data: RawBytes(&[VELOCITY_MIN_SUPPORTED_VERSION]),
    })
    .await?;

    // Get Response
    let plugin_response: LoginPluginResponse = ctrl.recv_packet().await?;

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
        username == Username::decode(&mut data_without_signature)?,
        "mismatched usernames"
    );

    // Read properties and get textures
    let mut textures = None;
    for prop in Vec::<SignedProperty>::decode(&mut data_without_signature)
        .context("failed to decode velocity player properties")?
    {
        if prop.name == "textures" {
            textures = Some(
                SignedPlayerTextures::from_base64(
                    prop.value,
                    prop.signature
                        .context("missing player textures signature")?,
                )
                .context("failed to parse signed player textures")?,
            );
            break;
        }
    }

    if version >= VELOCITY_MODERN_FORWARDING_WITH_KEY_V2 {
        // TODO
    }

    Ok(NewClientData {
        uuid,
        username,
        textures,
        remote_addr,
    })
}

#[cfg(test)]
mod tests {
    use sha1::Digest;

    use super::*;

    #[test]
    fn auth_digest_correct() {
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
