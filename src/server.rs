//! The heart of the server.

use std::error::Error;
use std::iter::FusedIterator;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{io, thread};

use anyhow::{ensure, Context};
use flume::{Receiver, Sender};
use rand::rngs::OsRng;
use rayon::iter::ParallelIterator;
use reqwest::Client as HttpClient;
use rsa::{PublicKeyParts, RsaPrivateKey};
use serde_json::{json, Value};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::{oneshot, Semaphore};
use uuid::Uuid;
use valence_nbt::{compound, Compound, List};

use crate::biome::{validate_biomes, Biome, BiomeId};
use crate::client::{Client, Clients};
use crate::config::{Config, ConnectionMode, ServerListPing};
use crate::dimension::{validate_dimensions, Dimension, DimensionId};
use crate::entity::Entities;
use crate::inventory::Inventories;
use crate::player_list::PlayerLists;
use crate::player_textures::SignedPlayerTextures;
use crate::protocol::codec::{Decoder, Encoder};
use crate::protocol::packets::c2s::handshake::{Handshake, HandshakeNextState};
use crate::protocol::packets::c2s::login::LoginStart;
use crate::protocol::packets::c2s::play::C2sPlayPacket;
use crate::protocol::packets::c2s::status::{PingRequest, StatusRequest};
use crate::protocol::packets::s2c::login::{DisconnectLogin, LoginSuccess, SetCompression};
use crate::protocol::packets::s2c::play::S2cPlayPacket;
use crate::protocol::packets::s2c::status::{PingResponse, StatusResponse};
use crate::protocol::{BoundedString, VarInt};
use crate::util::valid_username;
use crate::world::Worlds;
use crate::{ident, Ticks, PROTOCOL_VERSION, VERSION_NAME};

mod login;

/// Contains the entire state of a running Minecraft server, accessible from
/// within the [update](crate::config::Config::update) loop.
pub struct Server<C: Config> {
    /// Custom state.
    pub state: C::ServerState,
    /// A handle to this server's [`SharedServer`].
    pub shared: SharedServer<C>,
    /// All of the clients on the server.
    pub clients: Clients<C>,
    /// All of entities on the server.
    pub entities: Entities<C>,
    /// All of the worlds on the server.
    pub worlds: Worlds<C>,
    /// All of the player lists on the server.
    pub player_lists: PlayerLists<C>,
    pub inventories: Inventories,
}

/// A handle to a Minecraft server containing the subset of functionality which
/// is accessible outside the [update][update] loop.
///
/// `SharedServer`s are internally refcounted and can
/// be shared between threads.
///
/// [update]: crate::config::Config::update
pub struct SharedServer<C: Config>(Arc<SharedServerInner<C>>);

impl<C: Config> Clone for SharedServer<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct SharedServerInner<C: Config> {
    cfg: C,
    address: SocketAddr,
    tick_rate: Ticks,
    connection_mode: ConnectionMode,
    max_connections: usize,
    incoming_packet_capacity: usize,
    outgoing_packet_capacity: usize,
    tokio_handle: Handle,
    /// Store this here so we don't drop it.
    _tokio_runtime: Option<Runtime>,
    dimensions: Vec<Dimension>,
    biomes: Vec<Biome>,
    /// Contains info about dimensions, biomes, and chats.
    /// Sent to all clients when joining.
    registry_codec: Compound,
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
    /// The UUID of the new client.
    pub uuid: Uuid,
    /// The username of the new client.
    pub username: String,
    /// The new client's player textures. May be `None` if the client does not
    /// have a skin or cape.
    pub textures: Option<SignedPlayerTextures>,
    /// The remote address of the new client.
    pub remote_addr: IpAddr,
}

struct NewClientMessage {
    ncd: NewClientData,
    reply: oneshot::Sender<S2cPacketChannels>,
}

/// The result type returned from [`start_server`].
pub type ShutdownResult = Result<(), Box<dyn Error + Send + Sync + 'static>>;

pub(crate) type S2cPacketChannels = (Sender<C2sPlayPacket>, Receiver<S2cPlayMessage>);
pub(crate) type C2sPacketChannels = (Sender<S2cPlayMessage>, Receiver<C2sPlayPacket>);

/// Messages sent to packet encoders.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub(crate) enum S2cPlayMessage {
    /// Queue a play packet for sending.
    Queue(S2cPlayPacket),
    /// Instructs the encoder to flush all queued packets to the TCP stream.
    Flush,
}

impl<P: Into<S2cPlayPacket>> From<P> for S2cPlayMessage {
    fn from(pkt: P) -> Self {
        Self::Queue(pkt.into())
    }
}

impl<C: Config> SharedServer<C> {
    /// Gets a reference to the config object used to start the server.
    pub fn config(&self) -> &C {
        &self.0.cfg
    }

    /// Gets the socket address this server is bound to.
    pub fn address(&self) -> SocketAddr {
        self.0.address
    }

    /// Gets the configured tick rate of this server.
    pub fn tick_rate(&self) -> Ticks {
        self.0.tick_rate
    }

    /// Gets the connection mode of the server.
    pub fn connection_mode(&self) -> &ConnectionMode {
        &self.0.connection_mode
    }

    /// Gets the maximum number of connections allowed to the server at once.
    pub fn max_connections(&self) -> usize {
        self.0.max_connections
    }

    /// Gets the configured incoming packet capacity.
    pub fn incoming_packet_capacity(&self) -> usize {
        self.0.incoming_packet_capacity
    }

    /// Gets the configured outgoing incoming packet capacity.
    pub fn outgoing_packet_capacity(&self) -> usize {
        self.0.outgoing_packet_capacity
    }

    /// Gets a handle to the tokio instance this server is using.
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
    ///
    /// It is safe but unspecified behavior to call this function using a
    /// [`BiomeId`] not originating from the configuration used to construct
    /// the server.
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

    pub(crate) fn registry_codec(&self) -> &Compound {
        &self.0.registry_codec
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
    /// shutdown. The given result is returned through [`start_server`].
    ///
    /// You may want to disconnect all players with a message prior to calling
    /// this function.
    pub fn shutdown<R, E>(&self, res: R)
    where
        R: Into<Result<(), E>>,
        E: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        self.0.connection_sema.close();
        *self.0.shutdown_result.lock().unwrap() = Some(res.into().map_err(|e| e.into()));
    }
}

/// Consumes the configuration and starts the server.
///
/// The function returns once the server has shut down, a runtime error
/// occurs, or the configuration is found to be invalid.
pub fn start_server<C: Config>(config: C, data: C::ServerState) -> ShutdownResult {
    let shared = setup_server(config)
        .context("failed to initialize server")
        .map_err(Box::<dyn Error + Send + Sync + 'static>::from)?;

    let _guard = shared.tokio_handle().enter();

    let mut server = Server {
        state: data,
        shared: shared.clone(),
        clients: Clients::new(),
        entities: Entities::new(),
        worlds: Worlds::new(shared.clone()),
        player_lists: PlayerLists::new(),
        inventories: Inventories::new(),
    };

    shared.config().init(&mut server);

    tokio::spawn(do_accept_loop(shared));

    do_update_loop(&mut server)
}

fn setup_server<C: Config>(cfg: C) -> anyhow::Result<SharedServer<C>> {
    let max_connections = cfg.max_connections();
    let address = cfg.address();
    let tick_rate = cfg.tick_rate();

    ensure!(tick_rate > 0, "tick rate must be greater than zero");

    let connection_mode = cfg.connection_mode();

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
    validate_dimensions(&dimensions)?;

    let biomes = cfg.biomes();
    validate_biomes(&biomes)?;

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

    let registry_codec = make_registry_codec(&dimensions, &biomes);

    let server = SharedServerInner {
        cfg,
        address,
        tick_rate,
        connection_mode,
        max_connections,
        incoming_packet_capacity,
        outgoing_packet_capacity,
        tokio_handle,
        _tokio_runtime: runtime,
        dimensions,
        biomes,
        registry_codec,
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

    Ok(SharedServer(Arc::new(server)))
}

fn make_registry_codec(dimensions: &[Dimension], biomes: &[Biome]) -> Compound {
    compound! {
        ident!("dimension_type") => compound! {
            "type" => ident!("dimension_type"),
            "value" => List::Compound(dimensions.iter().enumerate().map(|(id, dim)| compound! {
                "name" => DimensionId(id as u16).dimension_type_name(),
                "id" => id as i32,
                "element" => dim.to_dimension_registry_item(),
            }).collect()),
        },
        ident!("worldgen/biome") => compound! {
            "type" => ident!("worldgen/biome"),
            "value" => {
                List::Compound(biomes
                    .iter()
                    .enumerate()
                    .map(|(id, biome)| biome.to_biome_registry_item(id as i32))
                    .collect())
            }

        },
        ident!("chat_type_registry") => compound! {
            "type" => ident!("chat_type"),
            "value" => List::Compound(Vec::new()),
        },
    }
}

fn do_update_loop<C: Config>(server: &mut Server<C>) -> ShutdownResult {
    let mut tick_start = Instant::now();

    let shared = server.shared.clone();
    loop {
        if let Some(res) = shared.0.shutdown_result.lock().unwrap().take() {
            return res;
        }

        while let Ok(msg) = shared.0.new_clients_rx.try_recv() {
            join_player(server, msg);
        }

        // Get serverbound packets first so they are not dealt with a tick late.
        server.clients.par_iter_mut().for_each(|(_, client)| {
            client.handle_serverbound_packets(&server.entities);
        });

        shared.config().update(server);

        server.worlds.par_iter_mut().for_each(|(id, world)| {
            world.spatial_index.update(&server.entities, id);
        });

        server.clients.par_iter_mut().for_each(|(_, client)| {
            client.update(
                &shared,
                &server.entities,
                &server.worlds,
                &server.player_lists,
                &server.inventories,
            );
        });

        server.entities.update();

        server.worlds.par_iter_mut().for_each(|(_, world)| {
            world.chunks.update();
        });

        server.player_lists.update();
        server.inventories.update();

        // Sleep for the remainder of the tick.
        let tick_duration = Duration::from_secs_f64((shared.0.tick_rate as f64).recip());
        thread::sleep(tick_duration.saturating_sub(tick_start.elapsed()));

        tick_start = Instant::now();
        shared.0.tick_counter.fetch_add(1, Ordering::SeqCst);
    }
}

fn join_player<C: Config>(server: &mut Server<C>, msg: NewClientMessage) {
    let (clientbound_tx, clientbound_rx) = flume::bounded(server.shared.0.outgoing_packet_capacity);
    let (serverbound_tx, serverbound_rx) = flume::bounded(server.shared.0.incoming_packet_capacity);

    let s2c_packet_channels: S2cPacketChannels = (serverbound_tx, clientbound_rx);
    let c2s_packet_channels: C2sPacketChannels = (clientbound_tx, serverbound_rx);

    let _ = msg.reply.send(s2c_packet_channels);

    let client = Client::new(c2s_packet_channels, msg.ncd, C::ClientState::default());

    server.clients.insert(client);
}

struct Codec {
    enc: Encoder<OwnedWriteHalf>,
    dec: Decoder<OwnedReadHalf>,
}

async fn do_accept_loop<C: Config>(server: SharedServer<C>) {
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
                        if let Err(e) = stream.set_nodelay(true) {
                            log::error!("failed to set TCP_NODELAY: {e}");
                        }

                        if let Err(e) = handle_connection(server, stream, remote_addr).await {
                            if let Some(e) = e.downcast_ref::<io::Error>() {
                                if e.kind() == io::ErrorKind::UnexpectedEof {
                                    return;
                                }
                            }
                            log::error!("connection to {remote_addr} ended: {e:#}");
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

async fn handle_connection<C: Config>(
    server: SharedServer<C>,
    stream: TcpStream,
    remote_addr: SocketAddr,
) -> anyhow::Result<()> {
    let timeout = Duration::from_secs(10);

    let (read, write) = stream.into_split();
    let mut c = Codec {
        enc: Encoder::new(write, timeout),
        dec: Decoder::new(read, timeout),
    };

    // TODO: peek stream for 0xFE legacy ping

    let handshake: Handshake = c.dec.read_packet().await?;

    ensure!(
        matches!(server.connection_mode(), ConnectionMode::BungeeCord)
            || handshake.server_address.chars().count() <= 255,
        "handshake server address is too long"
    );

    match handshake.next_state {
        HandshakeNextState::Status => handle_status(server, &mut c, remote_addr, handshake)
            .await
            .context("error during status"),
        HandshakeNextState::Login => match handle_login(&server, &mut c, remote_addr, handshake)
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

async fn handle_status<C: Config>(
    server: SharedServer<C>,
    c: &mut Codec,
    remote_addr: SocketAddr,
    handshake: Handshake,
) -> anyhow::Result<()> {
    c.dec.read_packet::<StatusRequest>().await?;

    match server
        .0
        .cfg
        .server_list_ping(&server, remote_addr, handshake.protocol_version.0)
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
                    "name": VERSION_NAME,
                    "protocol": PROTOCOL_VERSION
                },
                "players": {
                    "online": online_players,
                    "max": max_players,
                    "sample": player_sample,
                },
                "description": description,
            });

            if let Some(data) = favicon_png {
                let mut buf = "data:image/png;base64,".to_owned();
                base64::encode_config_buf(data, base64::STANDARD, &mut buf);
                json.as_object_mut()
                    .unwrap()
                    .insert("favicon".to_owned(), Value::String(buf));
            }

            c.enc
                .write_packet(&StatusResponse {
                    json_response: json.to_string(),
                })
                .await?;
        }
        ServerListPing::Ignore => return Ok(()),
    }

    let PingRequest { payload } = c.dec.read_packet().await?;

    c.enc.write_packet(&PingResponse { payload }).await?;

    Ok(())
}

/// Handle the login process and return the new client's data if successful.
async fn handle_login(
    server: &SharedServer<impl Config>,
    c: &mut Codec,
    remote_addr: SocketAddr,
    handshake: Handshake,
) -> anyhow::Result<Option<NewClientData>> {
    if handshake.protocol_version.0 != PROTOCOL_VERSION {
        // TODO: send translated disconnect msg?
        return Ok(None);
    }

    let LoginStart {
        username: BoundedString(username),
        sig_data: _,   // TODO
        profile_id: _, // TODO
    } = c.dec.read_packet().await?;

    ensure!(valid_username(&username), "invalid username '{username}'");

    let ncd = match server.connection_mode() {
        ConnectionMode::Online => login::online(server, c, remote_addr, username).await?,
        ConnectionMode::Offline => login::offline(remote_addr, username)?,
        ConnectionMode::BungeeCord => login::bungeecord(&handshake.server_address, username)?,
        ConnectionMode::Velocity { secret } => {
            login::velocity(c, username, secret).await?
        }
    };

    let compression_threshold = 256;
    c.enc
        .write_packet(&SetCompression {
            threshold: VarInt(compression_threshold as i32),
        })
        .await?;

    c.enc.enable_compression(compression_threshold);
    c.dec.enable_compression(compression_threshold);

    if let Err(reason) = server.0.cfg.login(server, &ncd).await {
        log::info!("Disconnect at login: \"{reason}\"");
        c.enc.write_packet(&DisconnectLogin { reason }).await?;
        return Ok(None);
    }

    c.enc
        .write_packet(&LoginSuccess {
            uuid: ncd.uuid,
            username: ncd.username.clone().into(),
            properties: Vec::new(),
        })
        .await?;

    Ok(Some(ncd))
}

async fn handle_play<C: Config>(
    server: &SharedServer<C>,
    c: Codec,
    ncd: NewClientData,
) -> anyhow::Result<()> {
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

    let Codec { mut enc, mut dec } = c;

    tokio::spawn(async move {
        while let Ok(msg) = packet_rx.recv_async().await {
            match msg {
                S2cPlayMessage::Queue(pkt) => {
                    if let Err(e) = enc.queue_packet(&pkt) {
                        log::debug!("error while queueing play packet: {e:#}");
                        break;
                    }
                }
                S2cPlayMessage::Flush => {
                    if let Err(e) = enc.flush().await {
                        log::debug!("error while flushing packet queue: {e:#}");
                        break;
                    }
                }
            }
        }
    });

    loop {
        let pkt = dec.read_packet().await?;
        if packet_tx.send_async(pkt).await.is_err() {
            break;
        }
    }

    Ok(())
}
