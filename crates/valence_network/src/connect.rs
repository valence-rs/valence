//! Handles new connections to the server and the log-in process.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{bail, ensure, Context};
use base64::prelude::*;
use hmac::digest::Update;
use hmac::{Hmac, Mac};
use num_bigint::BigInt;
use reqwest::StatusCode;
use rsa::Pkcs1v15Encrypt;
use serde::Deserialize;
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, trace, warn};
use uuid::Uuid;
use valence_lang::keys;
use valence_protocol::packets::configuration::select_known_packs_s2c::KnownPack;
use valence_protocol::packets::configuration::{
    ClientInformationC2s, CustomPayloadC2s, CustomPayloadS2c, FinishConfigurationC2s,
    FinishConfigurationS2c, RegistryDataS2c, SelectKnownPacksC2s, SelectKnownPacksS2c,
    StoreCookieS2c, UpdateEnabledFeaturesS2c, UpdateTagsS2c,
};
use valence_protocol::packets::login::{LoginAcknowledgedC2s, LoginFinishedS2c};
use valence_protocol::packets::status::{
    PingRequestC2s, PongResponseS2c, StatusRequestC2s, StatusResponseS2c,
};
use valence_protocol::profile::Property;
use valence_protocol::{Bounded, Decode, Packet};
use valence_server::client::Properties;
use valence_server::protocol::packets::handshake::intention_c2s::HandshakeNextState;
use valence_server::protocol::packets::handshake::IntentionC2s;
use valence_server::protocol::packets::login::{
    CustomQueryAnswerC2s, CustomQueryS2c, HelloC2s, HelloS2c, KeyC2s, LoginCompressionS2c,
    LoginDisconnectS2c,
};
use valence_server::protocol::{PacketDecoder, PacketEncoder, RawBytes, VarInt};
use valence_server::text::{Color, IntoText};
use valence_server::{ident, Ident, Text, MINECRAFT_VERSION, PROTOCOL_VERSION};

use crate::legacy_ping::try_handle_legacy_ping;
use crate::packet_io::PacketIo;
use crate::tags::default_tags;
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

    let timeout = Duration::from_secs(5);

    loop {
        match shared.0.connection_sema.clone().acquire_owned().await {
            Ok(permit) => match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let shared = shared.clone();

                    tokio::spawn(async move {
                        if let Err(e) = tokio::time::timeout(
                            timeout,
                            handle_connection(shared, stream, remote_addr),
                        )
                        .await
                        {
                            warn!("initial connection timed out: {e}");
                        }

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

async fn handle_connection(
    shared: SharedNetworkState,
    mut stream: TcpStream,
    remote_addr: SocketAddr,
) {
    trace!("handling connection");

    if let Err(e) = stream.set_nodelay(true) {
        error!("failed to set TCP_NODELAY: {e}");
    }

    match try_handle_legacy_ping(&shared, &mut stream, remote_addr).await {
        Ok(true) => return, // Legacy ping succeeded.
        Ok(false) => {}     // No legacy ping.
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
        Err(e) => {
            warn!("legacy ping ended with error: {e:#}");
        }
    }

    let io = PacketIo::new(stream, PacketEncoder::new(), PacketDecoder::new());

    if let Err(e) = handle_handshake(shared, io, remote_addr).await {
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

/// Basic information about a client, provided at the beginning of the
/// connection
#[derive(Default, Debug)]
pub struct HandshakeData {
    /// The protocol version of the client.
    pub protocol_version: i32,
    /// The address that the client used to connect.
    pub server_address: String,
    /// The port that the client used to connect.
    pub server_port: u16,
}

async fn handle_handshake(
    shared: SharedNetworkState,
    mut io: PacketIo,
    remote_addr: SocketAddr,
) -> anyhow::Result<()> {
    let handshake = io.recv_packet::<IntentionC2s>().await?;

    let next_state = handshake.next_state;

    let handshake = HandshakeData {
        protocol_version: handshake.protocol_version.0,
        server_address: handshake.server_address.0.to_owned(),
        server_port: handshake.server_port,
    };

    // TODO: this is borked.
    ensure!(
        shared.0.connection_mode == ConnectionMode::BungeeCord
            || handshake.server_address.encode_utf16().count() <= 255,
        "handshake server address is too long"
    );

    match next_state {
        HandshakeNextState::Status => handle_status(shared, io, remote_addr, handshake)
            .await
            .context("handling status"),
        HandshakeNextState::Login => {
            match handle_login(&shared, &mut io, remote_addr, handshake)
                .await
                .context("handling login")?
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
    io.recv_packet::<StatusRequestC2s>().await?;

    match shared
        .0
        .callbacks
        .inner
        .server_list_ping(&shared, remote_addr, &handshake)
        .await
    {
        ServerListPing::Respond {
            online_players,
            max_players,
            player_sample,
            mut description,
            favicon_png,
            version_name,
            protocol,
        } => {
            // For pre-1.16 clients, replace all webcolors with their closest
            // normal colors Because webcolor support was only
            // added at 1.16.
            if handshake.protocol_version < 735 {
                fn fallback_webcolors(txt: &mut Text) {
                    if let Some(Color::Rgb(color)) = txt.color {
                        txt.color = Some(Color::Named(color.to_named_lossy()));
                    }
                    for child in &mut txt.extra {
                        fallback_webcolors(child);
                    }
                }

                fallback_webcolors(&mut description);
            }

            let mut json = json!({
                "version": {
                    "name": version_name,
                    "protocol": protocol,
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

            io.send_packet(&StatusResponseS2c {
                json: &json.to_string(),
            })
            .await?;
        }
        ServerListPing::Ignore => return Ok(()),
    }

    let PingRequestC2s { payload } = io.recv_packet().await?;

    io.send_packet(&PongResponseS2c { payload }).await?;

    Ok(())
}

/// Handle the login process and return the new client's data if successful.
async fn handle_login(
    shared: &SharedNetworkState,
    io: &mut PacketIo,
    remote_addr: SocketAddr,
    handshake: HandshakeData,
) -> anyhow::Result<Option<(NewClientInfo, CleanupOnDrop)>> {
    if handshake.protocol_version != PROTOCOL_VERSION {
        io.send_packet(&LoginDisconnectS2c {
            // TODO: use correct translation key.
            reason: format!("Mismatched Minecraft version (server is on {MINECRAFT_VERSION})")
                .color(Color::RED)
                .into(),
        })
        .await?;

        return Ok(None);
    }

    let HelloC2s {
        username,
        .. // TODO: profile_id
    } = io.recv_packet().await?;

    let username = username.0.to_owned();

    let info = match shared.connection_mode() {
        ConnectionMode::Online { .. } => login_online(shared, io, remote_addr, username).await?,
        ConnectionMode::Offline => login_offline(remote_addr, username)?,
        ConnectionMode::BungeeCord => {
            login_bungeecord(remote_addr, &handshake.server_address, username)?
        }
        ConnectionMode::Velocity { secret } => login_velocity(io, username, secret).await?,
    };

    if shared.0.threshold.0 > 0 {
        io.send_packet(&LoginCompressionS2c {
            threshold: shared.0.threshold.0.into(),
        })
        .await?;

        io.set_compression(shared.0.threshold);
    }

    let cleanup = match shared.0.callbacks.inner.login(shared, &info).await {
        Ok(f) => CleanupOnDrop(Some(f)),
        Err(reason) => {
            info!("disconnect at login: \"{reason}\"");
            io.send_packet(&LoginDisconnectS2c {
                reason: reason.into(),
            })
            .await?;
            return Ok(None);
        }
    };

    io.send_packet(&LoginFinishedS2c {
        uuid: info.uuid,
        username: info.username.as_str().into(),
        properties: Default::default(),
        // strict_error_handling: true,
    })
    .await?;
    let LoginAcknowledgedC2s {} = io.recv_packet().await?;

    let custom_query: CustomQueryAnswerC2s = io.recv_packet().await?;
    info!(
        "query: {} {:?}",
        custom_query.message_id.0, custom_query.data.0 .0
    );

    let client_information: ClientInformationC2s = io.recv_packet().await?;
    info!("information: {:?}", client_information);

    io.send_packet(&CustomPayloadS2c {
        channel: Ident::new("minecraft:brand").unwrap(),
        data: Bounded(RawBytes(&[&[0x07], "vanilla".as_bytes()].concat())),
        // key: Ident::new("minecraft:brand").unwrap(),
        // payload: "valence".as_bytes().into(),
    })
    .await?;

    io.send_packet(&UpdateEnabledFeaturesS2c {
        features: vec![ident!("minecraft:vanilla").into()],
    })
    .await?;

    io.send_packet(&SelectKnownPacksS2c {
        packs: vec![KnownPack {
            namespace: "minecraft".into(),
            id: "core".into(),
            version: "1.21.3".into(),
        }],
    })
    .await?;

    let _know_packs: SelectKnownPacksC2s = io.recv_packet().await?;
    // info!("Know packs: {know_packs:?}");

    // now in configuration state

    // TODO: send our regestries and stuff, the client will not be happy to join if
    // the regestries to show the current stuff is not present

    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:worldgen/biome")?,
        entries: [
            "minecraft:sparse_jungle",
            "minecraft:the_end",
            "minecraft:deep_dark",
            "minecraft:frozen_peaks",
            "minecraft:ice_spikes",
            "minecraft:soul_sand_valley",
            "minecraft:snowy_beach",
            "minecraft:deep_lukewarm_ocean",
            "minecraft:end_highlands",
            "minecraft:old_growth_birch_forest",
            "minecraft:windswept_hills",
            "minecraft:dripstone_caves",
            "minecraft:eroded_badlands",
            "minecraft:jungle",
            "minecraft:windswept_savanna",
            "minecraft:the_void",
            "minecraft:beach",
            "minecraft:cherry_grove",
            "minecraft:end_midlands",
            "minecraft:grove",
            "minecraft:swamp",
            "minecraft:savanna_plateau",
            "minecraft:river",
            "minecraft:jagged_peaks",
            "minecraft:cold_ocean",
            "minecraft:dark_forest",
            "minecraft:desert",
            "minecraft:frozen_ocean",
            "minecraft:old_growth_pine_taiga",
            "minecraft:flower_forest",
            "minecraft:snowy_plains",
            "minecraft:end_barrens",
            "minecraft:warped_forest",
            "minecraft:basalt_deltas",
            "minecraft:windswept_gravelly_hills",
            "minecraft:deep_ocean",
            "minecraft:old_growth_spruce_taiga",
            "minecraft:savanna",
            "minecraft:deep_cold_ocean",
            "minecraft:ocean",
            "minecraft:meadow",
            "minecraft:small_end_islands",
            "minecraft:stony_shore",
            "minecraft:snowy_slopes",
            "minecraft:birch_forest",
            "minecraft:warm_ocean",
            "minecraft:nether_wastes",
            "minecraft:mangrove_swamp",
            "minecraft:snowy_taiga",
            "minecraft:deep_frozen_ocean",
            "minecraft:badlands",
            "minecraft:plains",
            "minecraft:lush_caves",
            "minecraft:frozen_river",
            "minecraft:crimson_forest",
            "minecraft:forest",
            "minecraft:lukewarm_ocean",
            "minecraft:stony_peaks",
            "minecraft:sunflower_plains",
            "minecraft:windswept_forest",
            "minecraft:wooded_badlands",
            "minecraft:mushroom_fields",
            "minecraft:bamboo_jungle",
            "minecraft:taiga",
            // Missing?
            "minecraft:is_badlands",
            "minecraft:is_jungle",
            "minecraft:is_savanna",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:chat_type")?,
        entries: [
            "minecraft:chat",
            "minecraft:say_command",
            "minecraft:team_msg_command_incoming",
            "minecraft:msg_command_incoming",
            "minecraft:emote_command",
            "minecraft:msg_command_outgoing",
            "minecraft:team_msg_command_outgoing",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:trim_pattern")?,
        entries: [
            "minecraft:flow",
            "minecraft:coast",
            "minecraft:ward",
            "minecraft:rib",
            "minecraft:bolt",
            "minecraft:dune",
            "minecraft:host",
            "minecraft:eye",
            "minecraft:raiser",
            "minecraft:snout",
            "minecraft:tide",
            "minecraft:sentry",
            "minecraft:vex",
            "minecraft:wayfinder",
            "minecraft:wild",
            "minecraft:shaper",
            "minecraft:spire",
            "minecraft:silence",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:trim_material")?,
        entries: [
            "minecraft:copper",
            "minecraft:netherite",
            "minecraft:diamond",
            "minecraft:emerald",
            "minecraft:quartz",
            "minecraft:redstone",
            "minecraft:iron",
            "minecraft:lapis",
            "minecraft:gold",
            "minecraft:amethyst",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:wolf_variant")?,
        entries: [
            "minecraft:ashen",
            "minecraft:pale",
            "minecraft:rusty",
            "minecraft:spotted",
            "minecraft:striped",
            "minecraft:woods",
            "minecraft:snowy",
            "minecraft:black",
            "minecraft:chestnut",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:painting_variant")?,
        entries: [
            "minecraft:wasteland",
            "minecraft:endboss",
            "minecraft:courbet",
            "minecraft:baroque",
            "minecraft:creebet",
            "minecraft:kebab",
            "minecraft:burning_skull",
            "minecraft:passage",
            "minecraft:changing",
            "minecraft:plant",
            "minecraft:prairie_ride",
            "minecraft:void",
            "minecraft:wanderer",
            "minecraft:wind",
            "minecraft:skull_and_roses",
            "minecraft:earth",
            "minecraft:fern",
            "minecraft:fighters",
            "minecraft:stage",
            "minecraft:unpacked",
            "minecraft:donkey_kong",
            "minecraft:owlemons",
            "minecraft:alban",
            "minecraft:bouquet",
            "minecraft:graham",
            "minecraft:pond",
            "minecraft:aztec",
            "minecraft:finding",
            "minecraft:cavebird",
            "minecraft:fire",
            "minecraft:cotan",
            "minecraft:pointer",
            "minecraft:match",
            "minecraft:skeleton",
            "minecraft:bust",
            "minecraft:sunflowers",
            "minecraft:wither",
            "minecraft:tides",
            "minecraft:bomb",
            "minecraft:aztec2",
            "minecraft:humble",
            "minecraft:sea",
            "minecraft:meditative",
            "minecraft:lowmist",
            "minecraft:pool",
            "minecraft:backyard",
            "minecraft:orb",
            "minecraft:pigscene",
            "minecraft:sunset",
            "minecraft:water",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:dimension_type")?,
        entries: [
            "minecraft:overworld",
            "minecraft:the_nether",
            "minecraft:overworld_caves",
            "minecraft:the_end",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:damage_type")?,
        entries: [
            "minecraft:out_of_world",
            "minecraft:wind_charge",
            "minecraft:mob_attack_no_aggro",
            "minecraft:arrow",
            "minecraft:lightning_bolt",
            "minecraft:freeze",
            "minecraft:thorns",
            "minecraft:falling_stalactite",
            "minecraft:mob_projectile",
            "minecraft:trident",
            "minecraft:explosion",
            "minecraft:cramming",
            "minecraft:starve",
            "minecraft:dry_out",
            "minecraft:indirect_magic",
            "minecraft:sting",
            "minecraft:wither_skull",
            "minecraft:fly_into_wall",
            "minecraft:hot_floor",
            "minecraft:drown",
            "minecraft:player_explosion",
            "minecraft:player_attack",
            "minecraft:dragon_breath",
            "minecraft:sonic_boom",
            "minecraft:sweet_berry_bush",
            "minecraft:cactus",
            "minecraft:fall",
            "minecraft:in_fire",
            "minecraft:campfire",
            "minecraft:generic",
            "minecraft:outside_border",
            "minecraft:spit",
            "minecraft:lava",
            "minecraft:fireball",
            "minecraft:thrown",
            "minecraft:falling_block",
            "minecraft:generic_kill",
            "minecraft:on_fire",
            "minecraft:stalagmite",
            "minecraft:wither",
            "minecraft:falling_anvil",
            "minecraft:magic",
            "minecraft:in_wall",
            "minecraft:unattributed_fireball",
            "minecraft:bad_respawn_point",
            "minecraft:fireworks",
            "minecraft:mace_smash",
            "minecraft:mob_attack",
            "minecraft:ender_pearl",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:banner_pattern")?,
        entries: [
            "minecraft:small_stripes",
            "minecraft:mojang",
            "minecraft:stripe_downright",
            "minecraft:square_bottom_right",
            "minecraft:flow",
            "minecraft:half_vertical_right",
            "minecraft:piglin",
            "minecraft:straight_cross",
            "minecraft:stripe_right",
            "minecraft:border",
            "minecraft:stripe_middle",
            "minecraft:triangles_bottom",
            "minecraft:triangles_top",
            "minecraft:bricks",
            "minecraft:diagonal_up_left",
            "minecraft:half_horizontal",
            "minecraft:stripe_downleft",
            "minecraft:skull",
            "minecraft:curly_border",
            "minecraft:flower",
            "minecraft:stripe_left",
            "minecraft:globe",
            "minecraft:half_vertical",
            "minecraft:gradient",
            "minecraft:rhombus",
            "minecraft:cross",
            "minecraft:square_bottom_left",
            "minecraft:triangle_bottom",
            "minecraft:diagonal_left",
            "minecraft:stripe_center",
            "minecraft:circle",
            "minecraft:creeper",
            "minecraft:square_top_left",
            "minecraft:stripe_bottom",
            "minecraft:diagonal_up_right",
            "minecraft:gradient_up",
            "minecraft:triangle_top",
            "minecraft:half_horizontal_bottom",
            "minecraft:stripe_top",
            "minecraft:square_top_right",
            "minecraft:diagonal_right",
            "minecraft:guster",
            "minecraft:base",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:enchantment")?,
        entries: [
            "minecraft:aqua_affinity",
            "minecraft:frost_walker",
            "minecraft:multishot",
            "minecraft:feather_falling",
            "minecraft:soul_speed",
            "minecraft:looting",
            "minecraft:flame",
            "minecraft:riptide",
            "minecraft:luck_of_the_sea",
            "minecraft:binding_curse",
            "minecraft:fortune",
            "minecraft:loyalty",
            "minecraft:silk_touch",
            "minecraft:thorns",
            "minecraft:mending",
            "minecraft:efficiency",
            "minecraft:breach",
            "minecraft:smite",
            "minecraft:bane_of_arthropods",
            "minecraft:density",
            "minecraft:infinity",
            "minecraft:lure",
            "minecraft:unbreaking",
            "minecraft:impaling",
            "minecraft:piercing",
            "minecraft:fire_aspect",
            "minecraft:projectile_protection",
            "minecraft:sharpness",
            "minecraft:sweeping_edge",
            "minecraft:wind_burst",
            "minecraft:quick_charge",
            "minecraft:channeling",
            "minecraft:protection",
            "minecraft:depth_strider",
            "minecraft:fire_protection",
            "minecraft:blast_protection",
            "minecraft:power",
            "minecraft:respiration",
            "minecraft:swift_sneak",
            "minecraft:vanishing_curse",
            "minecraft:punch",
            "minecraft:knockback",
            // Missing?
            "minecraft:exclusive_set/armor",
            "minecraft:exclusive_set/boots",
            "minecraft:exclusive_set/bow",
            "minecraft:exclusive_set/crossbow",
            "minecraft:exclusive_set/damage",
            "minecraft:exclusive_set/mining",
            "minecraft:exclusive_set/riptide",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:jukebox_song")?,
        entries: [
            "minecraft:precipice",
            "minecraft:wait",
            "minecraft:strad",
            "minecraft:otherside",
            "minecraft:13",
            "minecraft:chirp",
            "minecraft:far",
            "minecraft:relic",
            "minecraft:creator",
            "minecraft:mall",
            "minecraft:stal",
            "minecraft:creator_music_box",
            "minecraft:blocks",
            "minecraft:ward",
            "minecraft:mellohi",
            "minecraft:5",
            "minecraft:cat",
            "minecraft:11",
            "minecraft:pigstep",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    io.send_packet(&RegistryDataS2c {
        id: Ident::new("minecraft:instrument")?,
        entries: [
            "minecraft:dream_goat_horn",
            "minecraft:yearn_goat_horn",
            "minecraft:call_goat_horn",
            "minecraft:feel_goat_horn",
            "minecraft:seek_goat_horn",
            "minecraft:sing_goat_horn",
            "minecraft:admire_goat_horn",
            "minecraft:ponder_goat_horn",
        ]
        .into_iter()
        .map(|e: &str| (e.try_into().unwrap(), None))
        .collect(),
    })
    .await?;
    // io.send_packet(&UpdateTagsS2c {
    //     groups: std::borrow::Cow::Owned(default_tags()),
    // })
    // .await?;
    io.send_packet(&FinishConfigurationS2c {}).await?;
    info!("Waiting...");
    let _: FinishConfigurationC2s = io.recv_packet().await?;
    info!("Connected");
    // loop {
    //     // info!("In loop");
    //     if let Ok(frame) = io.try_recv_packet().await {
    //         match frame.id {
    //             FinishConfigurationC2s::ID => {
    //                 info!("Finished configuration");
    //                 break;
    //             }

    //             e => info!("got packet id: {}", e), /* ignore any packets that do
    // not progress to
    //                                                  * next step */
    //         }
    //     }
    // }

    Ok(Some((info, cleanup)))
}

/// Login procedure for online mode.
async fn login_online(
    shared: &SharedNetworkState,
    io: &mut PacketIo,
    remote_addr: SocketAddr,
    username: String,
) -> anyhow::Result<NewClientInfo> {
    let my_verify_token: [u8; 16] = rand::random();

    io.send_packet(&HelloS2c {
        server_id: "".into(), // Always empty
        public_key: &shared.0.public_key_der,
        verify_token: &my_verify_token,
        should_authenticate: true,
    })
    .await?;

    let KeyC2s {
        shared_secret,
        verify_token: encrypted_verify_token,
    } = io.recv_packet().await?;

    let shared_secret = shared
        .0
        .rsa_key
        .decrypt(Pkcs1v15Encrypt, shared_secret)
        .context("failed to decrypt shared secret")?;

    let verify_token = shared
        .0
        .rsa_key
        .decrypt(Pkcs1v15Encrypt, encrypted_verify_token)
        .context("failed to decrypt verify token")?;

    ensure!(
        my_verify_token.as_slice() == verify_token,
        "verify tokens do not match"
    );

    let crypt_key: [u8; 16] = shared_secret
        .as_slice()
        .try_into()
        .context("shared secret has the wrong length")?;

    io.enable_encryption(&crypt_key);

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
            let reason = Text::translate(keys::MULTIPLAYER_DISCONNECT_UNVERIFIED_USERNAME, []);
            io.send_packet(&LoginDisconnectS2c {
                reason: reason.into(),
            })
            .await?;
            bail!("session server could not verify username");
        }
        status => {
            bail!("session server GET request failed (status code {status})");
        }
    }

    #[derive(Deserialize)]
    struct GameProfile {
        id: Uuid,
        name: String,
        properties: Vec<Property>,
    }

    let profile: GameProfile = resp.json().await.context("parsing game profile")?;

    ensure!(profile.name == username, "usernames do not match");

    Ok(NewClientInfo {
        uuid: profile.id,
        username,
        ip: remote_addr.ip(),
        properties: Properties(profile.properties),
    })
}

fn auth_digest(bytes: &[u8]) -> String {
    BigInt::from_signed_bytes_be(bytes).to_str_radix(16)
}

fn offline_uuid(username: &str) -> anyhow::Result<Uuid> {
    Uuid::from_slice(&Sha256::digest(username)[..16]).map_err(Into::into)
}

/// Login procedure for offline mode.
fn login_offline(remote_addr: SocketAddr, username: String) -> anyhow::Result<NewClientInfo> {
    Ok(NewClientInfo {
        // Derive the client's UUID from a hash of their username.
        uuid: offline_uuid(username.as_str())?,
        username,
        properties: Default::default(),
        ip: remote_addr.ip(),
    })
}

/// Login procedure for `BungeeCord`.
fn login_bungeecord(
    remote_addr: SocketAddr,
    server_address: &str,
    username: String,
) -> anyhow::Result<NewClientInfo> {
    // Get data from server_address field of the handshake
    let data = server_address.split('\0').take(4).collect::<Vec<_>>();

    // Ip of player, only given if ip_forward on bungee is true
    let ip = match data.get(1) {
        Some(ip) => ip.parse()?,
        None => remote_addr.ip(),
    };

    // Uuid of player, only given if ip_forward on bungee is true
    let uuid = match data.get(2) {
        Some(uuid) => uuid.parse()?,
        None => offline_uuid(username.as_str())?,
    };

    // Read properties and get textures
    // Properties of player's game profile, only given if ip_forward and online_mode
    // on bungee both are true
    let properties: Vec<Property> = match data.get(3) {
        Some(properties) => serde_json::from_str(properties)
            .context("failed to parse BungeeCord player properties")?,
        None => vec![],
    };

    Ok(NewClientInfo {
        uuid,
        username,
        properties: Properties(properties),
        ip,
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
    io.send_packet(&CustomQueryS2c {
        message_id: VarInt(message_id),
        channel: ident!("velocity:player_info").into(),
        data: RawBytes(&[VELOCITY_MIN_SUPPORTED_VERSION]).into(),
    })
    .await?;

    // Get Response
    let plugin_response: CustomQueryAnswerC2s = io.recv_packet().await?;

    ensure!(
        plugin_response.message_id.0 == message_id,
        "mismatched plugin response ID (got {}, expected {message_id})",
        plugin_response.message_id.0,
    );

    let data = plugin_response
        .data
        // .context("missing plugin response data")?
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
        properties: Properties(properties),
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
