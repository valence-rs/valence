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
pub(crate) use packet_manager::{PlayPacketReceiver, PlayPacketSender};
use rand::rngs::OsRng;
use rayon::iter::ParallelIterator;
use reqwest::Client as ReqwestClient;
use rsa::{PublicKeyParts, RsaPrivateKey};
use serde_json::{json, Value};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{error, info, info_span, instrument, trace, warn};
use uuid::Uuid;
use valence_nbt::{compound, Compound, List};
use valence_protocol::packets::c2s::handshake::HandshakeOwned;
use valence_protocol::packets::c2s::login::LoginStart;
use valence_protocol::packets::c2s::status::{PingRequest, StatusRequest};
use valence_protocol::packets::s2c::login::{DisconnectLogin, LoginSuccess, SetCompression};
use valence_protocol::packets::s2c::status::{PingResponse, StatusResponse};
use valence_protocol::types::HandshakeNextState;
use valence_protocol::{
    ident, PacketDecoder, PacketEncoder, Username, VarInt, MINECRAFT_VERSION, PROTOCOL_VERSION,
};

use crate::biome::{validate_biomes, Biome, BiomeId};
use crate::chunk::entity_partition::update_entity_partition;
use crate::client::{Client, Clients};
use crate::config::{Config, ConnectionMode, ServerListPing};
use crate::dimension::{validate_dimensions, Dimension, DimensionId};
use crate::entity::Entities;
use crate::inventory::Inventories;
use crate::player_list::PlayerLists;
use crate::player_textures::SignedPlayerTextures;
use crate::server::packet_manager::InitialPacketManager;
use crate::world::Worlds;
use crate::Ticks;

mod byte_channel;
mod login;
mod packet_manager;

/// Contains the entire state of a running Minecraft server, accessible from
/// within the [init] and [update] functions.
///
/// [init]: crate::config::Config::init
/// [update]: crate::config::Config::update
#[non_exhaustive]
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
    /// All of the inventories on the server.
    pub inventories: Inventories<C>,
}

/// A handle to a Minecraft server containing the subset of functionality which
/// is accessible outside the [update] loop.
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
    compression_threshold: Option<u32>,
    max_connections: usize,
    incoming_capacity: usize,
    outgoing_capacity: usize,
    /// The tokio handle used by the server.
    tokio_handle: Handle,
    /// Holding a runtime handle is not enough to keep tokio working. We need
    /// to store the runtime here so we don't drop it.
    _tokio_runtime: Option<Runtime>,
    dimensions: Vec<Dimension>,
    biomes: Vec<Biome>,
    /// Contains info about dimensions, biomes, and chats.
    /// Sent to all clients when joining.
    registry_codec: Compound,
    /// The instant the server was started.
    start_instant: Instant,
    /// Receiver for new clients past the login stage.
    new_clients_send: Sender<NewClientMessage>,
    new_clients_recv: Receiver<NewClientMessage>,
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
    http_client: ReqwestClient,
}

/// Contains information about a new client joining the server.
#[non_exhaustive]
pub struct NewClientData {
    /// The username of the new client.
    pub username: Username<String>,
    /// The UUID of the new client.
    pub uuid: Uuid,
    /// The remote address of the new client.
    pub ip: IpAddr,
    /// The new client's player textures. May be `None` if the client does not
    /// have a skin or cape.
    pub textures: Option<SignedPlayerTextures>,
}

struct NewClientMessage {
    ncd: NewClientData,
    send: PlayPacketSender,
    recv: PlayPacketReceiver,
    permit: OwnedSemaphorePermit,
}

/// The result type returned from [`start_server`].
pub type ShutdownResult = Result<(), Box<dyn Error + Send + Sync + 'static>>;

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

    /// Gets the configured incoming capacity.
    pub fn incoming_capacity(&self) -> usize {
        self.0.incoming_capacity
    }

    /// Gets the configured outgoing incoming capacity.
    pub fn outgoing_capacity(&self) -> usize {
        self.0.outgoing_capacity
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
    pub fn shutdown<E>(&self, res: Result<(), E>)
    where
        E: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        self.0.connection_sema.close();
        *self.0.shutdown_result.lock().unwrap() = Some(res.map_err(|e| e.into()));
    }
}

/// Consumes the configuration and starts the server.
///
/// This function blocks the current thread and returns once the server has shut
/// down, a runtime error occurs, or the configuration is found to be invalid.
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

    info_span!("configured_init").in_scope(|| shared.config().init(&mut server));

    tokio::spawn(do_accept_loop(shared));

    do_update_loop(&mut server)
}

#[instrument(skip_all)]
fn setup_server<C: Config>(cfg: C) -> anyhow::Result<SharedServer<C>> {
    let max_connections = cfg.max_connections();
    let address = cfg.address();
    let tick_rate = cfg.tick_rate();

    ensure!(tick_rate > 0, "tick rate must be greater than zero");

    let connection_mode = cfg.connection_mode();

    let incoming_packet_capacity = cfg.incoming_capacity();

    ensure!(
        incoming_packet_capacity > 0,
        "serverbound packet capacity must be nonzero"
    );

    let outgoing_packet_capacity = cfg.outgoing_capacity();

    ensure!(
        outgoing_packet_capacity > 0,
        "outgoing packet capacity must be nonzero"
    );

    let compression_threshold = cfg.compression_threshold();

    let tokio_handle = cfg.tokio_handle();

    let dimensions = cfg.dimensions();
    validate_dimensions(&dimensions)?;

    let biomes = cfg.biomes();
    validate_biomes(&biomes)?;

    let rsa_key = RsaPrivateKey::new(&mut OsRng, 1024)?;

    let public_key_der =
        rsa_der::public_key_to_der(&rsa_key.n().to_bytes_be(), &rsa_key.e().to_bytes_be())
            .into_boxed_slice();

    let (new_clients_send, new_clients_recv) = flume::bounded(64);

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
        compression_threshold,
        max_connections,
        incoming_capacity: incoming_packet_capacity,
        outgoing_capacity: outgoing_packet_capacity,
        tokio_handle,
        _tokio_runtime: runtime,
        dimensions,
        biomes,
        registry_codec,
        start_instant: Instant::now(),
        new_clients_send,
        new_clients_recv,
        tick_counter: AtomicI64::new(0),
        connection_sema: Arc::new(Semaphore::new(max_connections)),
        shutdown_result: Mutex::new(None),
        rsa_key,
        public_key_der,
        http_client: ReqwestClient::new(),
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
        ident!("chat_type") => compound! {
            "type" => ident!("chat_type"),
            "value" => List::Compound(Vec::new()),
        },
    }
}

fn do_update_loop(server: &mut Server<impl Config>) -> ShutdownResult {
    let mut tick_start = Instant::now();
    let mut current_tick = 0;
    let shared = server.shared.clone();

    loop {
        let _span = info_span!("update_loop", tick = current_tick).entered();

        if let Some(res) = shared.0.shutdown_result.lock().unwrap().take() {
            return res;
        }

        for _ in 0..shared.0.new_clients_recv.len() {
            let Ok(msg) = shared.0.new_clients_recv.try_recv() else {
                break
            };

            info!(
                username = %msg.ncd.username,
                uuid = %msg.ncd.uuid,
                ip = %msg.ncd.ip,
                "inserting client"
            );

            server.clients.insert(Client::new(
                msg.send,
                msg.recv,
                msg.permit,
                msg.ncd,
                Default::default(),
            ));
        }

        // Get serverbound packets first so they are not dealt with a tick late.
        for (_, client) in server.clients.iter_mut() {
            client.prepare_c2s_packets();
        }

        info_span!("configured_update").in_scope(|| shared.config().update(server));

        update_entity_partition(
            &server.entities,
            &mut server.worlds,
            shared.0.compression_threshold,
        );

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

        server.worlds.update();

        server.player_lists.update();

        server.inventories.update();

        // Sleep for the remainder of the tick.
        let tick_duration = Duration::from_secs_f64((shared.0.tick_rate as f64).recip());
        thread::sleep(tick_duration.saturating_sub(tick_start.elapsed()));

        tick_start = Instant::now();
        current_tick = shared.0.tick_counter.fetch_add(1, Ordering::SeqCst) + 1;
    }
}

#[instrument(skip_all)]
async fn do_accept_loop(server: SharedServer<impl Config>) {
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
                    tokio::spawn(handle_connection(
                        server.clone(),
                        stream,
                        remote_addr,
                        permit,
                    ));
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

#[instrument(skip(server, stream))]
async fn handle_connection(
    server: SharedServer<impl Config>,
    stream: TcpStream,
    remote_addr: SocketAddr,
    permit: OwnedSemaphorePermit,
) {
    trace!("handling connection");

    if let Err(e) = stream.set_nodelay(true) {
        error!("failed to set TCP_NODELAY: {e}");
    }

    let (read, write) = stream.into_split();

    let mngr = InitialPacketManager::new(
        read,
        write,
        PacketEncoder::new(),
        PacketDecoder::new(),
        Duration::from_secs(5),
        permit,
    );

    // TODO: peek stream for 0xFE legacy ping

    if let Err(e) = handle_handshake(server, mngr, remote_addr).await {
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

async fn handle_handshake(
    server: SharedServer<impl Config>,
    mut mngr: InitialPacketManager<OwnedReadHalf, OwnedWriteHalf>,
    remote_addr: SocketAddr,
) -> anyhow::Result<()> {
    let handshake = mngr.recv_packet::<HandshakeOwned>().await?;

    ensure!(
        matches!(server.connection_mode(), ConnectionMode::BungeeCord)
            || handshake.server_address.chars().count() <= 255,
        "handshake server address is too long"
    );

    match handshake.next_state {
        HandshakeNextState::Status => handle_status(server, mngr, remote_addr, handshake)
            .await
            .context("error handling status"),
        HandshakeNextState::Login => match handle_login(&server, &mut mngr, remote_addr, handshake)
            .await
            .context("error handling login")?
        {
            Some(ncd) => {
                let (send, recv, permit) = mngr.into_play(
                    server.0.incoming_capacity,
                    server.0.outgoing_capacity,
                    server.tokio_handle().clone(),
                );

                let msg = NewClientMessage {
                    ncd,
                    send,
                    recv,
                    permit,
                };

                let _ = server.0.new_clients_send.send_async(msg).await;
                Ok(())
            }
            None => Ok(()),
        },
    }
}

async fn handle_status(
    server: SharedServer<impl Config>,
    mut mngr: InitialPacketManager<OwnedReadHalf, OwnedWriteHalf>,
    remote_addr: SocketAddr,
    handshake: HandshakeOwned,
) -> anyhow::Result<()> {
    mngr.recv_packet::<StatusRequest>().await?;

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

            if let Some(data) = favicon_png {
                let mut buf = "data:image/png;base64,".to_owned();
                base64::encode_config_buf(data, base64::STANDARD, &mut buf);
                json.as_object_mut()
                    .unwrap()
                    .insert("favicon".to_owned(), Value::String(buf));
            }

            mngr.send_packet(&StatusResponse {
                json: &json.to_string(),
            })
            .await?;
        }
        ServerListPing::Ignore => return Ok(()),
    }

    let PingRequest { payload } = mngr.recv_packet().await?;

    mngr.send_packet(&PingResponse { payload }).await?;

    Ok(())
}

/// Handle the login process and return the new client's data if successful.
async fn handle_login(
    server: &SharedServer<impl Config>,
    mngr: &mut InitialPacketManager<OwnedReadHalf, OwnedWriteHalf>,
    remote_addr: SocketAddr,
    handshake: HandshakeOwned,
) -> anyhow::Result<Option<NewClientData>> {
    if handshake.protocol_version.0 != PROTOCOL_VERSION {
        // TODO: send translated disconnect msg?
        return Ok(None);
    }

    let LoginStart {
        username,
        sig_data: _,   // TODO
        profile_id: _, // TODO
    } = mngr.recv_packet().await?;

    let username = username.to_owned_username();

    let ncd = match server.connection_mode() {
        ConnectionMode::Online => login::online(server, mngr, remote_addr, username).await?,
        ConnectionMode::Offline => login::offline(remote_addr, username)?,
        ConnectionMode::BungeeCord => login::bungeecord(&handshake.server_address, username)?,
        ConnectionMode::Velocity { secret } => login::velocity(mngr, username, secret).await?,
    };

    if let Some(threshold) = server.0.compression_threshold {
        mngr.send_packet(&SetCompression {
            threshold: VarInt(threshold as i32),
        })
        .await?;

        mngr.set_compression(Some(threshold));
    }

    if let Err(reason) = server.0.cfg.login(server, &ncd).await {
        info!("disconnect at login: \"{reason}\"");
        mngr.send_packet(&DisconnectLogin { reason }).await?;
        return Ok(None);
    }

    mngr.send_packet(&LoginSuccess {
        uuid: ncd.uuid,
        username: ncd.username.as_str_username(),
        properties: Vec::new(),
    })
    .await?;

    Ok(Some(ncd))
}
