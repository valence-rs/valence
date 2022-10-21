//! Contains login procedures for the different [`ConnectionMode`]s.

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
use uuid::Uuid;

use crate::config::Config;
use crate::ident;
use crate::player_textures::SignedPlayerTextures;
use crate::protocol::packets::c2s::login::{
    EncryptionResponse, LoginPluginResponse, VerifyTokenOrMsgSig,
};
use crate::protocol::packets::s2c::login::{
    DisconnectLogin, EncryptionRequest, LoginPluginRequest,
};
use crate::protocol::packets::Property;
use crate::protocol::{BoundedArray, BoundedString, Decode, RawBytes, VarInt};
use crate::server::{Codec, NewClientData, SharedServer};
use crate::text::Text;

/// Login sequence for [`ConnectionMode::Online`].
pub(super) async fn online(
    server: &SharedServer<impl Config>,
    c: &mut Codec,
    remote_addr: SocketAddr,
    username: String,
) -> anyhow::Result<NewClientData> {
    let my_verify_token: [u8; 16] = rand::random();

    c.enc
        .write_packet(&EncryptionRequest {
            server_id: Default::default(), // Always empty
            public_key: server.0.public_key_der.to_vec(),
            verify_token: my_verify_token.to_vec().into(),
        })
        .await?;

    let EncryptionResponse {
        shared_secret: BoundedArray(encrypted_shared_secret),
        token_or_sig,
    } = c.dec.read_packet().await?;

    let shared_secret = server
        .0
        .rsa_key
        .decrypt(PaddingScheme::PKCS1v15Encrypt, &encrypted_shared_secret)
        .context("failed to decrypt shared secret")?;

    let _opt_signature = match token_or_sig {
        VerifyTokenOrMsgSig::VerifyToken(BoundedArray(encrypted_verify_token)) => {
            let verify_token = server
                .0
                .rsa_key
                .decrypt(PaddingScheme::PKCS1v15Encrypt, &encrypted_verify_token)
                .context("failed to decrypt verify token")?;

            ensure!(
                my_verify_token.as_slice() == verify_token,
                "verify tokens do not match"
            );
            None
        }
        VerifyTokenOrMsgSig::MsgSig(sig) => Some(sig),
    };

    let crypt_key: [u8; 16] = shared_secret
        .as_slice()
        .try_into()
        .context("shared secret has the wrong length")?;

    c.enc.enable_encryption(&crypt_key);
    c.dec.enable_encryption(&crypt_key);

    #[derive(Debug, Deserialize)]
    struct AuthResponse {
        id: String,
        name: String,
        properties: Vec<Property>,
    }

    let hash = Sha1::new()
        .chain(&shared_secret)
        .chain(&server.0.public_key_der)
        .finalize();

    let url = server.config().format_session_server_url(
        server,
        &username,
        &auth_digest(&hash),
        &remote_addr.ip(),
    );

    let resp = server.0.http_client.get(url).send().await?;

    match resp.status() {
        StatusCode::OK => {}
        StatusCode::NO_CONTENT => {
            let reason = Text::translate("multiplayer.disconnect.unverified_username");
            c.enc.write_packet(&DisconnectLogin { reason }).await?;
            bail!("session server could not verify username");
        }
        status => {
            bail!("session server GET request failed (status code {status})");
        }
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

/// Login sequence for [`ConnectionMode::Offline`].
pub(super) fn offline(remote_addr: SocketAddr, username: String) -> anyhow::Result<NewClientData> {
    Ok(NewClientData {
        // Derive the client's UUID from a hash of their username.
        uuid: Uuid::from_slice(&Sha256::digest(&username)[..16])?,
        username,
        textures: None,
        remote_addr: remote_addr.ip(),
    })
}

/// Login sequence for [`ConnectionMode::BungeeCord`].
pub(super) fn bungeecord(
    server_address: &str,
    username: String,
) -> anyhow::Result<NewClientData> {
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
    c: &mut Codec,
    username: String,
    velocity_secret: &str,
) -> anyhow::Result<NewClientData> {
    const VELOCITY_MIN_SUPPORTED_VERSION: u8 = 1;
    const VELOCITY_MODERN_FORWARDING_WITH_KEY_V2: i32 = 3;

    let message_id = 0;

    // Send Player Info Request into the Plugin Channel
    c.enc
        .write_packet(&LoginPluginRequest {
            message_id: VarInt(message_id),
            channel: ident!("velocity:player_info"),
            data: RawBytes(vec![VELOCITY_MIN_SUPPORTED_VERSION]),
        })
        .await?;

    // Get Response
    let plugin_response: LoginPluginResponse = c.dec.read_packet().await?;

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
    let velocity_username = BoundedString::<0, 16>::decode(&mut data_without_signature)?.0;
    ensure!(username == velocity_username, "mismatched usernames");

    // Read properties and get textures
    let mut textures = None;
    for prop in Vec::<Property>::decode(&mut data_without_signature)
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
