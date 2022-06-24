use std::collections::HashSet;
use std::error::Error;
use std::iter::FusedIterator;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context};
use flume::{Receiver, Sender};
use num::BigInt;
use parking_lot::Mutex;
use rand::rngs::OsRng;
use rayon::iter::ParallelIterator;
use reqwest::Client as HttpClient;
use rsa::{PaddingScheme, PublicKeyParts, RsaPrivateKey};
use serde::Deserialize;
use serde_json::{json, Value};
use sha1::digest::Update;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::{oneshot, Semaphore};
use uuid::Uuid;

use crate::codec::{Decoder, Encoder};
use crate::config::{Config, ServerListPing};
use crate::packets::handshake::{Handshake, HandshakeNextState};
use crate::packets::login;
use crate::packets::login::c2s::{EncryptionResponse, LoginStart, VerifyTokenOrMsgSig};
use crate::packets::login::s2c::{EncryptionRequest, LoginSuccess, SetCompression};
use crate::packets::play::c2s::C2sPlayPacket;
use crate::packets::play::s2c::S2cPlayPacket;
use crate::packets::status::c2s::{Ping, Request};
use crate::packets::status::s2c::{Pong, Response};
use crate::protocol::{BoundedArray, BoundedString};
use crate::util::valid_username;
use crate::var_int::VarInt;
use crate::world::Worlds;
use crate::{
    Biome, BiomeId, Client, ClientMut, Dimension, DimensionId, Ticks, WorldsMut, PROTOCOL_VERSION,
    VERSION_NAME,
};

/// A handle to a running Minecraft server containing state which is accessible
/// outside the update loop. Servers are internally refcounted and can be shared
/// between threads.
#[derive(Clone)]
pub struct Server(Arc<ServerInner>);

struct ServerInner {
    cfg: Box<dyn Config>,
    address: SocketAddr,
    tick_rate: Ticks,
    online_mode: bool,
    max_connections: usize,
    incoming_packet_capacity: usize,
    outgoing_packet_capacity: usize,
    tokio_handle: Handle,
    /// Store this here so we don't drop it.
    _tokio_runtime: Option<Runtime>,
    dimensions: Vec<Dimension>,
    biomes: Vec<Biome>,
    /// The instant the server was started.
    start_instant: Instant,
    /// Receiver for new clients past the login stage.
    new_clients_rx: Receiver<NewClientMessage>,
    new_clients_tx: Sender<NewClientMessage>,
    /// Incremented on every game tick.
    tick_counter: AtomicI64,
    /// A semaphore used to limit the number of simultaneous connections to the
    /// server. Closing this semaphore stops new connections.
    connection_sema: Arc<Semaphore>,
    /// The result that will be returned when the server is shut down.
    shutdown_result: Mutex<Option<ShutdownResult>>,
    /// The RSA keypair used for encryption with clients.
    rsa_key: RsaPrivateKey,
    /// The public part of `rsa_key` encoded in DER, which is an ASN.1 format.
    /// This is sent to clients during the authentication process.
    public_key_der: Box<[u8]>,
    /// For session server requests.
    http_client: HttpClient,
}

/// Contains information about a new client.
pub struct NewClientData {
    pub uuid: Uuid,
    pub username: String,
    pub remote_addr: SocketAddr,
}

struct NewClientMessage {
    ncd: NewClientData,
    reply: oneshot::Sender<S2cPacketChannels>,
}

/// The result type returned from [`ServerConfig::start`] after the server is
/// shut down.
pub type ShutdownResult = Result<(), ShutdownError>;
pub type ShutdownError = Box<dyn Error + Send + Sync + 'static>;

pub(crate) type S2cPacketChannels = (Sender<C2sPlayPacket>, Receiver<S2cPlayPacket>);
pub(crate) type C2sPacketChannels = (Sender<S2cPlayPacket>, Receiver<C2sPlayPacket>);

impl Server {
    pub fn config(&self) -> &(impl Config + ?Sized) {
        self.0.cfg.as_ref()
    }

    pub fn address(&self) -> SocketAddr {
        self.0.address
    }

    pub fn tick_rate(&self) -> Ticks {
        self.0.tick_rate
    }

    pub fn online_mode(&self) -> bool {
        self.0.online_mode
    }

    pub fn max_connections(&self) -> usize {
        self.0.max_connections
    }

    pub fn incoming_packet_capacity(&self) -> usize {
        self.0.incoming_packet_capacity
    }

    pub fn outgoing_packet_capacity(&self) -> usize {
        self.0.outgoing_packet_capacity
    }

    pub fn tokio_handle(&self) -> &Handle {
        &self.0.tokio_handle
    }

    /// Obtains a [`Dimension`] by using its corresponding [`DimensionId`].
    ///
    /// It is safe but unspecified behavior to call this function using a
    /// [`DimensionId`] not originating from the configuration used to construct
    /// the server.
    pub fn dimension(&self, id: DimensionId) -> &Dimension {
        self.0
            .dimensions
            .get(id.0 as usize)
            .expect("invalid dimension ID")
    }

    /// Returns an iterator over all added dimensions and their associated
    /// [`DimensionId`].
    pub fn dimensions(&self) -> impl FusedIterator<Item = (DimensionId, &Dimension)> + Clone {
        self.0
            .dimensions
            .iter()
            .enumerate()
            .map(|(i, d)| (DimensionId(i as u16), d))
    }

    /// Obtains a [`Biome`] by using its corresponding [`BiomeId`].
    pub fn biome(&self, id: BiomeId) -> &Biome {
        self.0.biomes.get(id.0 as usize).expect("invalid biome ID")
    }

    /// Returns an iterator over all added biomes and their associated
    /// [`BiomeId`] in ascending order.
    pub fn biomes(
        &self,
    ) -> impl ExactSizeIterator<Item = (BiomeId, &Biome)> + DoubleEndedIterator + FusedIterator + Clone
    {
        self.0
            .biomes
            .iter()
            .enumerate()
            .map(|(i, b)| (BiomeId(i as u16), b))
    }

    /// Returns the instant the server was started.
    pub fn start_instant(&self) -> Instant {
        self.0.start_instant
    }

    /// Returns the number of ticks that have elapsed since the server began.
    pub fn current_tick(&self) -> Ticks {
        self.0.tick_counter.load(Ordering::SeqCst)
    }

    /// Immediately stops new connections to the server and initiates server
    /// shutdown. The given result is returned through [`ServerConfig::start`].
    ///
    /// You may want to disconnect all players with a message prior to calling
    /// this function.
    pub fn shutdown<R, E>(&self, res: R)
    where
        R: Into<Result<(), E>>,
        E: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        self.0.connection_sema.close();
        *self.0.shutdown_result.lock() = Some(res.into().map_err(|e| e.into()));
    }
}

/// Consumes the configuration and starts the server.
///
/// The function returns when the server has shut down, a runtime error
/// occurs, or the configuration is invalid.
pub fn start_server(config: impl Config) -> ShutdownResult {
    let server = setup_server(config).map_err(ShutdownError::from)?;

    let _guard = server.tokio_handle().enter();

    let mut worlds = Worlds::new(server.clone());
    let mut worlds_mut = WorldsMut::new(&mut worlds);

    server.config().init(&server, worlds_mut.reborrow());

    tokio::spawn(do_accept_loop(server.clone()));

    do_update_loop(server, worlds_mut)
}

fn setup_server(cfg: impl Config) -> anyhow::Result<Server> {
    let max_connections = cfg.max_connections();
    let address = cfg.address();
    let tick_rate = cfg.tick_rate();

    ensure!(tick_rate > 0, "tick rate must be greater than zero");

    let online_mode = cfg.online_mode();

    let incoming_packet_capacity = cfg.incoming_packet_capacity();

    ensure!(
        incoming_packet_capacity > 0,
        "serverbound packet capacity must be nonzero"
    );

    let outgoing_packet_capacity = cfg.outgoing_packet_capacity();

    ensure!(
        outgoing_packet_capacity > 0,
        "outgoing packet capacity must be nonzero"
    );

    let tokio_handle = cfg.tokio_handle();
    let dimensions = cfg.dimensions();

    ensure!(
        !dimensions.is_empty(),
        "at least one dimension must be added"
    );

    ensure!(
        dimensions.len() <= u16::MAX as usize,
        "more than u16::MAX dimensions added"
    );

    for (i, dim) in dimensions.iter().enumerate() {
        ensure!(
            dim.min_y % 16 == 0 && (-2032..=2016).contains(&dim.min_y),
            "invalid min_y in dimension #{i}",
        );

        ensure!(
            dim.height % 16 == 0
                && (0..=4064).contains(&dim.height)
                && dim.min_y.saturating_add(dim.height) <= 2032,
            "invalid height in dimension #{i}",
        );

        ensure!(
            (0.0..=1.0).contains(&dim.ambient_light),
            "ambient_light is out of range in dimension #{i}",
        );

        if let Some(fixed_time) = dim.fixed_time {
            assert!(
                (0..=24_000).contains(&fixed_time),
                "fixed_time is out of range in dimension #{i}",
            );
        }
    }

    let biomes = cfg.biomes();

    ensure!(!biomes.is_empty(), "at least one biome must be added");

    ensure!(
        biomes.len() <= u16::MAX as usize,
        "more than u16::MAX biomes added"
    );

    let mut names = HashSet::new();

    for biome in biomes.iter() {
        ensure!(
            names.insert(biome.name.clone()),
            "biome \"{}\" already added",
            biome.name
        );
    }

    let rsa_key = RsaPrivateKey::new(&mut OsRng, 1024)?;

    let public_key_der =
        rsa_der::public_key_to_der(&rsa_key.n().to_bytes_be(), &rsa_key.e().to_bytes_be())
            .into_boxed_slice();

    let (new_clients_tx, new_clients_rx) = flume::bounded(1);

    let runtime = if tokio_handle.is_none() {
        Some(Runtime::new()?)
    } else {
        None
    };

    let tokio_handle = match &runtime {
        Some(rt) => rt.handle().clone(),
        None => tokio_handle.unwrap(),
    };

    let server = ServerInner {
        cfg: Box::new(cfg),
        address,
        tick_rate,
        online_mode,
        max_connections,
        incoming_packet_capacity,
        outgoing_packet_capacity,
        tokio_handle,
        _tokio_runtime: runtime,
        dimensions,
        biomes,
        start_instant: Instant::now(),
        new_clients_rx,
        new_clients_tx,
        tick_counter: AtomicI64::new(0),
        connection_sema: Arc::new(Semaphore::new(max_connections)),
        shutdown_result: Mutex::new(None),
        rsa_key,
        public_key_der,
        http_client: HttpClient::new(),
    };

    Ok(Server(Arc::new(server)))
}

fn do_update_loop(server: Server, mut worlds: WorldsMut) -> ShutdownResult {
    let mut tick_start = Instant::now();

    loop {
        if let Some(res) = server.0.shutdown_result.lock().take() {
            return res;
        }

        while let Ok(msg) = server.0.new_clients_rx.try_recv() {
            join_player(&server, worlds.reborrow(), msg);
        }

        // Get serverbound packets first so they are not dealt with a tick late.
        worlds.par_iter_mut().for_each(|(_, mut world)| {
            world.clients.par_iter_mut().for_each(|(_, mut client)| {
                client.handle_serverbound_packets(&world.entities);
            });
        });

        server.config().update(&server, worlds.reborrow());

        worlds.par_iter_mut().for_each(|(_, mut world)| {
            world.chunks.par_iter_mut().for_each(|(_, mut chunk)| {
                if chunk.created_tick() == server.current_tick() {
                    // Chunks created this tick can have their changes applied immediately because
                    // they have not been observed by clients yet. Clients will not have to be sent
                    // the block change packet in this case.
                    chunk.apply_modifications();
                }
            });

            world.spatial_index.update(world.entities.reborrow());

            world.clients.par_iter_mut().for_each(|(_, mut client)| {
                client.update(
                    &server,
                    &world.entities,
                    &world.spatial_index,
                    &world.chunks,
                    &world.meta,
                );
            });

            world.entities.update();

            world.chunks.par_iter_mut().for_each(|(_, mut chunk)| {
                chunk.apply_modifications();
            });
        });

        // Sleep for the remainder of the tick.
        let tick_duration = Duration::from_secs_f64((server.0.tick_rate as f64).recip());
        thread::sleep(tick_duration.saturating_sub(tick_start.elapsed()));

        tick_start = Instant::now();
        server.0.tick_counter.fetch_add(1, Ordering::SeqCst);
    }
}

fn join_player(server: &Server, mut worlds: WorldsMut, msg: NewClientMessage) {
    let (clientbound_tx, clientbound_rx) = flume::bounded(server.0.outgoing_packet_capacity);
    let (serverbound_tx, serverbound_rx) = flume::bounded(server.0.incoming_packet_capacity);

    let client_packet_channels: S2cPacketChannels = (serverbound_tx, clientbound_rx);
    let server_packet_channels: C2sPacketChannels = (clientbound_tx, serverbound_rx);

    let _ = msg.reply.send(client_packet_channels);

    let mut client = Client::new(
        server_packet_channels,
        msg.ncd.username,
        msg.ncd.uuid,
        server,
    );
    let mut client_mut = ClientMut::new(&mut client);

    match server
        .0
        .cfg
        .join(server, client_mut.reborrow(), worlds.reborrow())
    {
        Ok(world_id) => {
            if let Some(mut world) = worlds.get_mut(world_id) {
                if world.entities.get_with_uuid(client.uuid()).is_none() {
                    world.clients.create(client);
                } else {
                    log::warn!(
                        "client '{}' cannot join the server because their UUID ({}) conflicts \
                         with an existing entity",
                        client.username(),
                        client.uuid()
                    );
                }
            } else {
                log::warn!(
                    "client '{}' cannot join the server because the WorldId returned by \
                     Config::join is invalid.",
                    client.username()
                );
            }
        }
        Err(errmsg) => client_mut.disconnect(errmsg),
    }
}

type Codec = (Encoder<OwnedWriteHalf>, Decoder<OwnedReadHalf>);

async fn do_accept_loop(server: Server) {
    log::trace!("entering accept loop");

    let listener = match TcpListener::bind(server.0.address).await {
        Ok(listener) => listener,
        Err(e) => {
            server.shutdown(Err(e).context("failed to start TCP listener"));
            return;
        }
    };

    loop {
        match server.0.connection_sema.clone().acquire_owned().await {
            Ok(permit) => match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let server = server.clone();
                    tokio::spawn(async move {
                        // Setting TCP_NODELAY to true appears to trade some throughput for improved
                        // latency. Testing is required to determine if this is worth keeping.
                        if let Err(e) = stream.set_nodelay(true) {
                            log::error!("failed to set TCP nodelay: {e}");
                        }

                        if let Err(e) = handle_connection(server, stream, remote_addr).await {
                            log::debug!("connection to {remote_addr} ended: {e:#}");
                        }
                        drop(permit);
                    });
                }
                Err(e) => {
                    log::error!("failed to accept incoming connection: {e}");
                }
            },
            // Closed semaphore indicates server shutdown.
            Err(_) => return,
        }
    }
}

async fn handle_connection(
    server: Server,
    stream: TcpStream,
    remote_addr: SocketAddr,
) -> anyhow::Result<()> {
    let timeout = Duration::from_secs(10);

    let (read, write) = stream.into_split();
    let mut c: Codec = (Encoder::new(write, timeout), Decoder::new(read, timeout));

    // TODO: peek stream for 0xFE legacy ping

    match c.1.read_packet::<Handshake>().await?.next_state {
        HandshakeNextState::Status => handle_status(server, &mut c, remote_addr)
            .await
            .context("error during status"),
        HandshakeNextState::Login => match handle_login(&server, &mut c, remote_addr)
            .await
            .context("error during login")?
        {
            Some(npd) => handle_play(&server, c, npd)
                .await
                .context("error during play"),
            None => Ok(()),
        },
    }
}

async fn handle_status(
    server: Server,
    c: &mut Codec,
    remote_addr: SocketAddr,
) -> anyhow::Result<()> {
    c.1.read_packet::<Request>().await?;

    match server.0.cfg.server_list_ping(&server, remote_addr).await {
        ServerListPing::Respond {
            online_players,
            max_players,
            description,
            favicon_png,
        } => {
            let mut json = json!({
                "version": {
                    "name": VERSION_NAME,
                    "protocol": PROTOCOL_VERSION
                },
                "players": {
                    "online": online_players,
                    "max": max_players,
                    // TODO: player sample?
                },
                "description": description,
            });

            if let Some(data) = favicon_png {
                let mut buf = "data:image/png;base64,".to_string();
                base64::encode_config_buf(data, base64::STANDARD, &mut buf);
                json.as_object_mut()
                    .unwrap()
                    .insert("favicon".to_string(), Value::String(buf));
            }

            c.0.write_packet(&Response {
                json_response: json.to_string(),
            })
            .await?;
        }
        ServerListPing::Ignore => return Ok(()),
    }

    let Ping { payload } = c.1.read_packet().await?;

    c.0.write_packet(&Pong { payload }).await?;

    Ok(())
}

/// Handle the login process and return the new player's data if successful.
async fn handle_login(
    server: &Server,
    c: &mut Codec,
    remote_addr: SocketAddr,
) -> anyhow::Result<Option<NewClientData>> {
    let LoginStart {
        username: BoundedString(username),
        sig_data: _, // TODO
    } = c.1.read_packet().await?;

    ensure!(valid_username(&username), "invalid username '{username}'");

    let (uuid, _skin_blob) = if server.0.online_mode {
        let my_verify_token: [u8; 16] = rand::random();

        c.0.write_packet(&EncryptionRequest {
            server_id: Default::default(), // Always empty
            public_key: server.0.public_key_der.to_vec(),
            verify_token: my_verify_token.to_vec().into(),
        })
        .await?;

        let EncryptionResponse {
            shared_secret: BoundedArray(encrypted_shared_secret),
            token_or_sig,
        } = c.1.read_packet().await?;

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

        c.0.enable_encryption(&crypt_key);
        c.1.enable_encryption(&crypt_key);

        #[derive(Debug, Deserialize)]
        struct AuthResponse {
            id: String,
            name: String,
            properties: Vec<Property>,
        }

        #[derive(Debug, Deserialize)]
        struct Property {
            name: String,
            value: String,
        }

        let hash = Sha1::new()
            .chain(&shared_secret)
            .chain(&server.0.public_key_der)
            .finalize();

        let hex_hash = weird_hex_encoding(&hash);

        let url = format!("https://sessionserver.mojang.com/session/minecraft/hasJoined?username={username}&serverId={hex_hash}&ip={}", remote_addr.ip());
        let resp = server.0.http_client.get(url).send().await?;

        let status = resp.status();
        ensure!(
            status.is_success(),
            "session server GET request failed: {status}"
        );

        let data: AuthResponse = resp.json().await?;

        ensure!(data.name == username, "usernames do not match");

        let uuid = Uuid::parse_str(&data.id).context("failed to parse player's UUID")?;

        let skin_blob = match data.properties.iter().find(|p| p.name == "textures") {
            Some(p) => base64::decode(&p.value).context("failed to parse skin blob")?,
            None => bail!("failed to find skin blob in auth response"),
        };

        (uuid, Some(skin_blob))
    } else {
        // Derive the player's UUID from a hash of their username.
        let uuid = Uuid::from_slice(&Sha256::digest(&username)[..16]).unwrap();

        (uuid, None)
    };

    let compression_threshold = 256;
    c.0.write_packet(&SetCompression {
        threshold: VarInt(compression_threshold as i32),
    })
    .await?;

    c.0.enable_compression(compression_threshold);
    c.1.enable_compression(compression_threshold);

    let npd = NewClientData {
        uuid,
        username,
        remote_addr,
    };

    if let Err(reason) = server.0.cfg.login(server, &npd).await {
        log::info!("Disconnect at login: \"{reason}\"");
        c.0.write_packet(&login::s2c::Disconnect { reason }).await?;
        return Ok(None);
    }

    c.0.write_packet(&LoginSuccess {
        uuid: npd.uuid,
        username: npd.username.clone().into(),
        properties: Vec::new(),
    })
    .await?;

    Ok(Some(npd))
}

async fn handle_play(server: &Server, c: Codec, ncd: NewClientData) -> anyhow::Result<()> {
    let (reply_tx, reply_rx) = oneshot::channel();

    server
        .0
        .new_clients_tx
        .send_async(NewClientMessage {
            ncd,
            reply: reply_tx,
        })
        .await?;

    let (packet_tx, packet_rx) = match reply_rx.await {
        Ok(res) => res,
        Err(_) => return Ok(()), // Server closed
    };

    let (mut encoder, mut decoder) = c;

    tokio::spawn(async move {
        while let Ok(pkt) = packet_rx.recv_async().await {
            if let Err(e) = encoder.write_packet(&pkt).await {
                log::debug!("error while sending play packet: {e:#}");
                break;
            }
        }
    });

    loop {
        let pkt = decoder.read_packet().await?;
        if packet_tx.send_async(pkt).await.is_err() {
            break;
        }
    }

    Ok(())
}

fn weird_hex_encoding(bytes: &[u8]) -> String {
    BigInt::from_signed_bytes_be(bytes).to_str_radix(16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weird_hex_encoding_correct() {
        assert_eq!(
            weird_hex_encoding(&Sha1::digest("Notch")),
            "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48"
        );
        assert_eq!(
            weird_hex_encoding(&Sha1::digest("jeb_")),
            "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1"
        );
        assert_eq!(
            weird_hex_encoding(&Sha1::digest("simon")),
            "88e16a1019277b15d58faf0541e11910eb756f6"
        );
    }
}
