use std::error::Error;
use std::iter::FusedIterator;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
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
use crate::config::{Biome, BiomeId, Dimension, DimensionId, Handler, Login, ServerListPing};
use crate::packets::handshake::{Handshake, HandshakeNextState};
use crate::packets::login::{
    self, EncryptionRequest, EncryptionResponse, LoginStart, LoginSuccess, SetCompression,
};
use crate::packets::play::{ClientPlayPacket, ServerPlayPacket};
use crate::packets::status::{Ping, Pong, Request, Response};
use crate::protocol::{BoundedArray, BoundedString};
use crate::util::valid_username;
use crate::var_int::VarInt;
use crate::{
    ChunkStore, Client, ClientStore, EntityStore, ServerConfig, Ticks, WorldStore,
    PROTOCOL_VERSION, VERSION_NAME,
};

/// Holds the state of a running Minecraft server which is accessible inside the
/// update loop. To start a server, see [`ServerConfig`].
///
/// Fields of this struct are made public to enable disjoint borrows. For
/// instance, it is possible to create and delete entities while
/// having read-only access to world data.
///
/// Note the `Deref` and `DerefMut` impls on `Server` are (ab)used to
/// allow convenient access to the `other` field.
#[non_exhaustive]
pub struct Server {
    pub entities: EntityStore,
    pub clients: ClientStore,
    pub worlds: WorldStore,
    pub chunks: ChunkStore,
    pub other: Other,
}

pub struct Other {
    /// The shared portion of the server.
    shared: SharedServer,
    new_players_rx: Receiver<NewClientMessage>,
    /// Incremented on every game tick.
    tick_counter: Ticks,
    /// The instant the current game tick began.
    tick_start: Instant,
    /// The time the last keep alive packet was sent to all players.
    pub(crate) last_keepalive: Instant,
}

/// A server handle providing the subset of functionality which can be performed
/// outside the update loop. `SharedServer`s are interally refcounted and can be
/// freely cloned and shared between threads.
#[derive(Clone)]
pub struct SharedServer(Arc<SharedServerInner>);

struct SharedServerInner {
    handler: Box<dyn Handler>,
    address: SocketAddr,
    update_duration: Duration,
    online_mode: bool,
    max_clients: usize,
    clientbound_packet_capacity: usize,
    serverbound_packet_capacity: usize,
    tokio_handle: Handle,
    dimensions: Vec<Dimension>,
    biomes: Vec<Biome>,
    /// The instant the server was started.
    start_instant: Instant,
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
    new_clients_tx: Sender<NewClientMessage>,
    client_count: AtomicUsize,
}

/// Contains information about a new player.
pub struct NewClientData {
    pub uuid: Uuid,
    pub username: String,
    pub remote_addr: SocketAddr,
}

struct NewClientMessage {
    ncd: NewClientData,
    reply: oneshot::Sender<anyhow::Result<ClientPacketChannels>>,
}

/// The result type returned from [`ServerConfig::start`] after the server is
/// shut down.
pub type ShutdownResult = Result<(), ShutdownError>;
pub type ShutdownError = Box<dyn Error + Send + Sync + 'static>;

pub(crate) type ClientPacketChannels = (Sender<ServerPlayPacket>, Receiver<ClientPlayPacket>);
pub(crate) type ServerPacketChannels = (Sender<ClientPlayPacket>, Receiver<ServerPlayPacket>);

impl Other {
    /// Returns a reference to a [`SharedServer`].
    pub fn shared(&self) -> &SharedServer {
        &self.shared
    }

    /// Returns the number of ticks that have elapsed since the server began.
    pub fn current_tick(&self) -> Ticks {
        self.tick_counter
    }

    /// Returns the instant the current tick began.
    pub fn tick_start(&self) -> Instant {
        self.tick_start
    }
}

impl SharedServer {
    pub fn handler(&self) -> &(impl Handler + ?Sized) {
        self.0.handler.as_ref()
    }

    pub fn address(&self) -> SocketAddr {
        self.0.address
    }

    pub fn update_duration(&self) -> Duration {
        self.0.update_duration
    }

    pub fn online_mode(&self) -> bool {
        self.0.online_mode
    }

    pub fn max_clients(&self) -> usize {
        self.0.max_clients
    }

    pub fn clientbound_packet_capacity(&self) -> usize {
        self.0.clientbound_packet_capacity
    }

    pub fn serverbound_packet_capacity(&self) -> usize {
        self.0.serverbound_packet_capacity
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
    pub fn dimensions(&self) -> impl FusedIterator<Item = (&Dimension, DimensionId)> + Clone {
        self.0
            .dimensions
            .iter()
            .enumerate()
            .map(|(i, d)| (d, DimensionId(i as u16)))
    }

    /// Obtains a [`Biome`] by using its corresponding [`BiomeId`].
    ///
    /// It is safe but unspecified behavior to call this function using a
    /// [`BiomeId`] not originating from the configuration used to construct the
    /// server.
    pub fn biome(&self, id: BiomeId) -> &Biome {
        self.0.biomes.get(id.0 as usize).expect("invalid biome ID")
    }

    /// Returns an iterator over all added biomes and their associated
    /// [`BiomeId`].
    pub fn biomes(&self) -> impl FusedIterator<Item = (&Biome, BiomeId)> + Clone {
        self.0
            .biomes
            .iter()
            .enumerate()
            .map(|(i, b)| (b, BiomeId(i as u16)))
    }

    /// Returns the instant the server was started.
    pub fn start_instant(&self) -> Instant {
        self.0.start_instant
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

    /// Returns the number of clients past the login stage that are currently
    /// connected to the server.
    pub fn client_count(&self) -> usize {
        self.0.client_count.load(Ordering::SeqCst)
    }

    /// Increment the client count iff it is below the maximum number of
    /// clients. Returns true if the client count was incremented, false
    /// otherwise.
    fn try_inc_player_count(&self) -> bool {
        self.0
            .client_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                if count < self.0.max_clients {
                    count.checked_add(1)
                } else {
                    None
                }
            })
            .is_ok()
    }

    pub(crate) fn dec_client_count(&self) {
        let prev = self.0.client_count.fetch_sub(1, Ordering::SeqCst);
        assert!(prev != 0);
    }
}

impl Deref for Server {
    type Target = Other;

    fn deref(&self) -> &Self::Target {
        &self.other
    }
}

impl DerefMut for Server {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.other
    }
}

impl Deref for Other {
    type Target = SharedServer;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

pub(crate) fn start_server(config: ServerConfig) -> ShutdownResult {
    let rsa_key = RsaPrivateKey::new(&mut OsRng, 1024)?;

    let public_key_der =
        rsa_der::public_key_to_der(&rsa_key.n().to_bytes_be(), &rsa_key.e().to_bytes_be())
            .into_boxed_slice();

    let (new_players_tx, new_players_rx) = flume::bounded(1);

    let rt = if config.tokio_handle.is_none() {
        Some(Runtime::new()?)
    } else {
        None
    };

    let handle = match &rt {
        Some(rt) => rt.handle().clone(),
        None => config.tokio_handle.unwrap(),
    };

    let _guard = handle.enter();

    let connection_sema = Arc::new(Semaphore::new(config.max_clients.saturating_add(64)));

    struct DummyHandler;
    impl Handler for DummyHandler {}

    let shared = SharedServer(Arc::new(SharedServerInner {
        handler: config.handler.unwrap_or_else(|| Box::new(DummyHandler)),
        address: config.address,
        update_duration: config.update_duration,
        online_mode: config.online_mode,
        max_clients: config.max_clients,
        clientbound_packet_capacity: config.clientbound_packet_capacity,
        serverbound_packet_capacity: config.serverbound_packet_capacity,
        tokio_handle: handle.clone(),
        dimensions: config.dimensions,
        biomes: config.biomes,
        start_instant: Instant::now(),
        connection_sema,
        shutdown_result: Mutex::new(None),
        rsa_key,
        public_key_der,
        http_client: HttpClient::new(),
        new_clients_tx: new_players_tx,
        client_count: AtomicUsize::new(0),
    }));

    let mut server = Server {
        entities: EntityStore::new(),
        clients: ClientStore::new(),
        worlds: WorldStore::new(),
        chunks: ChunkStore::new(),
        other: Other {
            shared: shared.clone(),
            tick_counter: 0,
            tick_start: Instant::now(),
            new_players_rx,
            last_keepalive: Instant::now(),
        },
    };

    shared.handler().init(&mut server);

    tokio::spawn(do_accept_loop(shared));

    do_update_loop(&mut server)
}

fn do_update_loop(server: &mut Server) -> ShutdownResult {
    server.tick_start = Instant::now();
    let shared = server.shared().clone();

    loop {
        if let Some(res) = server.0.shutdown_result.lock().take() {
            return res;
        }

        while let Ok(msg) = server.new_players_rx.try_recv() {
            join_player(server, msg);
        }

        const KEEPALIVE_FREQ: Duration = Duration::from_secs(8);
        if server.tick_start().duration_since(server.last_keepalive) >= KEEPALIVE_FREQ {
            server.last_keepalive = server.tick_start();
        }

        {
            server.clients.par_iter_mut().for_each(|(_, client)| {
                client.update(
                    &server.entities,
                    &server.worlds,
                    &server.chunks,
                    &server.other,
                )
            });
        }

        server.entities.update();

        server
            .chunks
            .par_iter_mut()
            .for_each(|(_, chunk)| chunk.apply_modifications());

        shared.handler().update(server);

        // Chunks modified this tick can have their changes applied immediately because
        // they have not been observed by clients yet.
        server.chunks.par_iter_mut().for_each(|(_, chunk)| {
            if chunk.created_this_tick() {
                chunk.clear_created_this_tick();
                chunk.apply_modifications();
            }
        });

        // Sleep for the remainder of the tick.
        thread::sleep(
            server
                .0
                .update_duration
                .saturating_sub(server.tick_start.elapsed()),
        );
        server.tick_start = Instant::now();

        server.tick_counter += 1;
    }
}

fn join_player(server: &mut Server, msg: NewClientMessage) {
    let (clientbound_tx, clientbound_rx) = flume::bounded(server.0.clientbound_packet_capacity);
    let (serverbound_tx, serverbound_rx) = flume::bounded(server.0.serverbound_packet_capacity);

    let client_packet_channels: ClientPacketChannels = (serverbound_tx, clientbound_rx);
    let server_packet_channels: ServerPacketChannels = (clientbound_tx, serverbound_rx);

    let _ = msg.reply.send(Ok(client_packet_channels));

    let client_backed_entity = match server.entities.create_with_uuid(msg.ncd.uuid) {
        Some(id) => id,
        None => {
            log::error!(
                "player '{}' cannot join the server because their UUID ({}) conflicts with an \
                 existing entity",
                msg.ncd.username,
                msg.ncd.uuid
            );
            return;
        }
    };

    let client_id = server.clients.create(Client::new(
        server_packet_channels,
        client_backed_entity,
        msg.ncd.username,
        server,
    ));
}

type Codec = (Encoder<OwnedWriteHalf>, Decoder<OwnedReadHalf>);

async fn do_accept_loop(server: SharedServer) {
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
                            log::error!("failed to set TCP nodelay: {e}")
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
    server: SharedServer,
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
    server: SharedServer,
    c: &mut Codec,
    remote_addr: SocketAddr,
) -> anyhow::Result<()> {
    c.1.read_packet::<Request>().await?;

    match server
        .0
        .handler
        .server_list_ping(&server, remote_addr)
        .await
    {
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
    server: &SharedServer,
    c: &mut Codec,
    remote_addr: SocketAddr,
) -> anyhow::Result<Option<NewClientData>> {
    let LoginStart {
        username: BoundedString(username),
    } = c.1.read_packet().await?;

    ensure!(valid_username(&username), "invalid username '{username}'");

    let (uuid, skin_blob) = if server.0.online_mode {
        let verify_token: [u8; 16] = rand::random();

        c.0.write_packet(&EncryptionRequest {
            server_id: Default::default(), // Always empty
            public_key: server.0.public_key_der.to_vec(),
            verify_token: verify_token.to_vec().into(),
        })
        .await?;

        let EncryptionResponse {
            shared_secret: BoundedArray(encrypted_shared_secret),
            verify_token: BoundedArray(encrypted_verify_token),
        } = c.1.read_packet().await?;

        let shared_secret = server
            .0
            .rsa_key
            .decrypt(PaddingScheme::PKCS1v15Encrypt, &encrypted_shared_secret)
            .context("Failed to decrypt shared secret")?;

        let new_verify_token = server
            .0
            .rsa_key
            .decrypt(PaddingScheme::PKCS1v15Encrypt, &encrypted_verify_token)
            .context("Failed to decrypt verify token")?;

        ensure!(
            verify_token.as_slice() == new_verify_token,
            "Verify tokens do not match"
        );

        let crypt_key: [u8; 16] = shared_secret
            .as_slice()
            .try_into()
            .context("Shared secret has the wrong length")?;

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

    if !server.try_inc_player_count() {
        let reason = server.0.handler.max_client_message(server, &npd).await;
        log::info!("Disconnect at login: \"{reason}\"");
        c.0.write_packet(&login::Disconnect { reason }).await?;
        return Ok(None);
    }

    if let Login::Disconnect(reason) = server.0.handler.login(server, &npd).await {
        log::info!("Disconnect at login: \"{reason}\"");
        c.0.write_packet(&login::Disconnect { reason }).await?;
        return Ok(None);
    }

    c.0.write_packet(&LoginSuccess {
        uuid: npd.uuid,
        username: npd.username.clone().into(),
    })
    .await?;

    Ok(Some(npd))
}

async fn handle_play(server: &SharedServer, c: Codec, ncd: NewClientData) -> anyhow::Result<()> {
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
        Ok(res) => res?,
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
