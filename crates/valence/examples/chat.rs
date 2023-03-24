use std::collections::{HashMap, HashSet, VecDeque};
use std::time::SystemTime;

use rsa::pkcs8::DecodePublicKey;
use rsa::{PaddingScheme, PublicKey, RsaPublicKey};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use tracing::{debug, info, warn, Level};
use valence::client::event::{
    ChatMessage, ClientSettings, CommandExecution, MessageAcknowledgment, PlayerSession,
};
use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::entity::player::PlayerBundle;
use valence::prelude::*;
use valence::protocol::packet::c2s::play::client_settings::ChatMode;
use valence::protocol::packet::s2c::play::chat_message::{ChatMessageS2c, MessageFilterType};
use valence::protocol::translation_key::{
    CHAT_DISABLED_CHAIN_BROKEN, CHAT_DISABLED_EXPIRED_PROFILE_KEY,
    CHAT_DISABLED_MISSING_PROFILE_KEY, CHAT_DISABLED_OPTIONS,
    MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED, MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY,
    MULTIPLAYER_DISCONNECT_INVALID_PUBLIC_KEY_SIGNATURE, MULTIPLAYER_DISCONNECT_OUT_OF_ORDER_CHAT,
    MULTIPLAYER_DISCONNECT_TOO_MANY_PENDING_CHATS, MULTIPLAYER_DISCONNECT_UNSIGNED_CHAT,
};
use valence::protocol::types::MessageSignature;
use valence::protocol::var_int::VarInt;

const SPAWN_Y: i32 = 64;

const MOJANG_KEY_DATA: &[u8] = include_bytes!("./yggdrasil_session_pubkey.der");

#[derive(Resource)]
struct MojangServicesState {
    public_key: RsaPublicKey,
}

#[derive(Component, Debug)]
struct ChatState {
    last_message_timestamp: u64,
    chat_mode: ChatMode,
    validator: AcknowledgementValidator,
    chain: MessageChain,
    signature_storage: MessageSignatureStorage,
    session: Option<ChatSession>,
}

#[derive(Clone, Debug)]
struct AcknowledgementValidator {
    messages: Vec<Option<AcknowledgedMessage>>,
    last_signature: Option<[u8; 256]>,
}

#[derive(Clone, Debug)]
struct AcknowledgedMessage {
    pub signature: [u8; 256],
    pub pending: bool,
}

#[derive(Clone, Debug)]
struct MessageChain {
    link: Option<MessageLink>,
}

#[derive(Copy, Clone, Debug)]
struct MessageLink {
    index: i32,
    sender: Uuid,
    session_id: Uuid,
}

#[derive(Clone, Debug)]
struct MessageSignatureStorage {
    signatures: [Option<[u8; 256]>; 128],
    indices: HashMap<[u8; 256], i32>,
}

#[derive(Clone, Debug)]
struct ChatSession {
    expires_at: i64,
    public_key: RsaPublicKey,
}

impl ChatState {
    pub fn add_pending(&mut self, last_seen: &mut VecDeque<[u8; 256]>, signature: &[u8; 256]) {
        self.signature_storage.add(last_seen, signature);
        self.validator.add_pending(signature);
    }
}

impl AcknowledgementValidator {
    pub fn new() -> Self {
        Self {
            messages: vec![None; 20],
            last_signature: None,
        }
    }

    /// Add a message pending acknowledgement via its `signature`.
    pub fn add_pending(&mut self, signature: &[u8; 256]) {
        if matches!(&self.last_signature, Some(last_sig) if signature == last_sig.as_ref()) {
            return;
        }
        self.messages.push(Some(AcknowledgedMessage {
            signature: *signature,
            pending: true,
        }));
        self.last_signature = Some(*signature);
    }

    /// Removes message signatures from the validator before an `index`.
    ///
    /// Message signatures will only be removed if the result leaves the
    /// validator with at least 20 messages. Returns `true` if messages are
    /// removed and `false` if they are not.
    pub fn remove_until(&mut self, index: i32) -> bool {
        // Ensure that there will still be 20 messages in the array
        if index >= 0 && index <= (self.messages.len() - 20) as i32 {
            self.messages.drain(0..index as usize);
            if self.messages.len() < 20 {
                warn!("Message validator 'messages' shrunk!");
            }
            return true;
        }
        false
    }

    /// Validate a set of `acknowledgements` offset by `message_index`.
    ///
    /// Returns a [`VecDeque`] of acknowledged message signatures if the
    /// `acknowledgements` are valid and `None` if they are invalid.
    pub fn validate(
        &mut self,
        acknowledgements: &[u8; 3],
        message_index: i32,
    ) -> Option<VecDeque<[u8; 256]>> {
        if !self.remove_until(message_index) {
            // Invalid message index
            return None;
        }

        let acknowledged_count = {
            let mut sum = 0u32;
            for byte in acknowledgements {
                sum += byte.count_ones();
            }
            sum as usize
        };

        if acknowledged_count > 20 {
            // Too many message acknowledgements, protocol error?
            return None;
        }

        let mut list = VecDeque::with_capacity(acknowledged_count);
        for i in 0..20 {
            let acknowledgement = acknowledgements[i >> 3] & (0b1 << (i % 8)) != 0;
            // SAFETY: The length of messages is never less than 20
            let acknowledged_message = unsafe { self.messages.get_unchecked_mut(i) };
            // Client has acknowledged the i-th message
            if acknowledgement {
                // The validator has the i-th message
                if let Some(m) = acknowledged_message {
                    m.pending = false;
                    list.push_back(m.signature);
                } else {
                    // Client has acknowledged a non-existing message
                    warn!("Client has acknowledged a non-existing message");
                    return None;
                }
            } else {
                // Client has not acknowledged the i-th message
                if matches!(acknowledged_message, Some(m) if !m.pending) {
                    // The validator has an i-th message that has been validated but the client
                    // claims that it hasn't been validated yet
                    warn!(
                        "The validator has an i-th message that has been validated but the client \
                         claims that it hasn't been validated yet"
                    );
                    return None;
                }
                // Honestly not entirely sure why this is done
                if acknowledged_message.is_some() {
                    *acknowledged_message = None;
                }
            }
        }
        Some(list)
    }

    /// The number of pending messages in the validator.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

impl MessageChain {
    pub fn next_link(&mut self) -> Option<MessageLink> {
        match &mut self.link {
            None => self.link,
            Some(current) => {
                let temp = *current;
                current.index += 1;
                Some(temp)
            }
        }
    }
}

impl MessageLink {
    pub fn update_hash(&self, hasher: &mut impl Digest) {
        hasher.update(self.sender.into_bytes());
        hasher.update(self.session_id.into_bytes());
        hasher.update(self.index.to_be_bytes());
    }
}

impl MessageSignatureStorage {
    /// Get the index of the `signature` in the storage if it exists.
    pub fn index_of(&self, signature: &[u8; 256]) -> Option<i32> {
        self.indices.get(signature).copied()
    }

    /// Update the signature storage according to `last_seen` while adding
    /// `signature` to the storage.
    ///
    /// Warning: this consumes `last_seen`.
    pub fn add(&mut self, last_seen: &mut VecDeque<[u8; 256]>, signature: &[u8; 256]) {
        last_seen.push_back(*signature);
        let mut sig_set = HashSet::new();
        for sig in last_seen.iter() {
            sig_set.insert(*sig);
        }
        for i in 0..128 {
            if last_seen.is_empty() {
                return;
            }
            // Remove old message
            let message_sig_data = self.signatures[i];
            // Add previously seen message
            self.signatures[i] = last_seen.pop_back();
            if let Some(data) = self.signatures[i] {
                self.indices.insert(data, i as i32);
            }
            // Reinsert old message if it is not already in last_seen
            if let Some(data) = message_sig_data {
                self.indices.remove(&data);
                if sig_set.insert(data) {
                    last_seen.push_front(data);
                }
            }
        }
    }
}

impl ChatSession {
    pub fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Unable to get Unix time")
            .as_millis()
            >= self.expires_at as u128
    }
}

pub fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems(
            (
                default_event_handler,
                handle_session_events,
                handle_message_events,
                handle_message_acknowledgement,
                handle_command_events,
                handle_chat_settings_event,
            )
                .in_schedule(EventLoopSchedule),
        )
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(mut commands: Commands, server: Res<Server>) {
    let mojang_pub_key = RsaPublicKey::from_public_key_der(MOJANG_KEY_DATA)
        .expect("Error creating Mojang public key");

    commands.insert_resource(MojangServicesState {
        public_key: mojang_pub_key,
    });

    let mut instance = server.new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::BEDROCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(Entity, &UniqueId, &Username, &mut Client, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, username, mut client, mut game_mode) in &mut clients {
        *game_mode = GameMode::Adventure;
        client.send_message("Welcome to Valence! Talk about something.".italic());

        commands
            .entity(entity)
            .insert(PlayerBundle {
                location: Location(instances.single()),
                position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 0.0]),
                uuid: *uuid,
                ..Default::default()
            })
            .insert(ChatState {
                last_message_timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Unable to get Unix time")
                    .as_millis() as u64,
                chat_mode: ChatMode::Enabled,
                validator: AcknowledgementValidator::new(),
                chain: MessageChain { link: None },
                signature_storage: MessageSignatureStorage {
                    signatures: [None; 128],
                    indices: HashMap::new(),
                },
                session: None,
            });

        info!("{} logged in!", username.0);
    }
}

fn handle_session_events(
    services_state: Res<MojangServicesState>,
    player_list: ResMut<PlayerList>,
    mut clients: Query<(&UniqueId, &Username, &mut ChatState)>,
    mut sessions: EventReader<PlayerSession>,
    mut commands: Commands,
) {
    let pl = player_list.into_inner();

    for session in sessions.iter() {
        let Ok((uuid, username, mut state)) = clients.get_mut(session.client) else {
            warn!("Unable to find client for session");
            continue;
        };

        let Some(player_entry) = pl.get_mut(uuid.0) else {
            warn!("Unable to find '{}' in the player list", username.0);
            continue;
        };

        // Verify that the session key has not expired
        if SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Unable to get Unix time")
            .as_millis()
            >= session.session_data.expires_at as u128
        {
            warn!("Failed to validate profile key: expired public key");
            commands.add(DisconnectClient {
                client: session.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY, []),
            });
            continue;
        }

        // Serialize the session data
        let mut serialized = Vec::with_capacity(318);
        serialized.extend_from_slice(uuid.0.into_bytes().as_slice());
        serialized.extend_from_slice(session.session_data.expires_at.to_be_bytes().as_ref());
        serialized.extend_from_slice(session.session_data.public_key_data.as_ref());

        // Hash the session data using the SHA-1 algorithm
        let mut hasher = Sha1::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();

        // Verify the session data using Mojang's public key and the hashed session data
        // against the message signature
        if services_state
            .public_key
            .verify(
                PaddingScheme::new_pkcs1v15_sign::<Sha1>(),
                &hash,
                session.session_data.key_signature.as_ref(),
            )
            .is_err()
        {
            warn!("Failed to validate profile key: invalid public key signature");
            commands.add(DisconnectClient {
                client: session.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_INVALID_PUBLIC_KEY_SIGNATURE, []),
            });
        }

        // Decode the player's session public key from the data
        if let Ok(public_key) =
            RsaPublicKey::from_public_key_der(session.session_data.public_key_data.as_ref())
        {
            // Update the player's chat state data with the new player session data
            state.chain.link = Some(MessageLink {
                index: 0,
                sender: uuid.0,
                session_id: session.session_data.session_id,
            });
            state.session = Some(ChatSession {
                expires_at: session.session_data.expires_at,
                public_key,
            });
        } else {
            // This shouldn't happen considering that it is highly unlikely that Mojang
            // would provide the client with a malformed key. By this point the
            // key signature has been verified
            warn!("Received malformed profile key data from '{}'", username.0);
            commands.add(DisconnectClient {
                client: session.client,
                reason: Text::translate(
                    MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY,
                    ["Malformed profile key data".color(Color::RED)],
                ),
            });
        }

        // Update the player list with the new session data
        // The player list will then send this new session data to the other clients
        player_entry.set_chat_data(Some(session.session_data.clone()));
    }
}

fn handle_message_acknowledgement(
    mut clients: Query<(&Username, &mut ChatState)>,
    mut acknowledgements: EventReader<MessageAcknowledgment>,
    mut commands: Commands,
) {
    for acknowledgement in acknowledgements.iter() {
        let Ok((username, mut state)) = clients.get_mut(acknowledgement.client) else {
            warn!("Unable to find client for acknowledgement");
            continue;
        };

        if !state.validator.remove_until(acknowledgement.message_index) {
            warn!(
                "Failed to validate message acknowledgement from '{:?}'",
                username.0
            );
            commands.add(DisconnectClient {
                client: acknowledgement.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED, []),
            });
            continue;
        }

        debug!("Acknowledgement from '{:?}'", username.0);
    }
}

fn handle_message_events(
    mut clients: Query<(&Username, &mut Client)>,
    mut states: Query<&mut ChatState>,
    mut messages: EventReader<ChatMessage>,
    mut instances: Query<&mut Instance>,
    mut commands: Commands,
) {
    let mut instance = instances.single_mut();

    for message in messages.iter() {
        let Ok((username, mut client)) = clients.get_mut(message.client) else {
            warn!("Unable to find client for message '{:?}'", message);
            continue;
        };

        let Ok(mut state) = states.get_component_mut::<ChatState>(message.client) else {
            warn!("Unable to find chat state for client '{:?}'", username.0);
            continue;
        };

        // Ensure that the client isn't sending messages while their chat is hidden
        if state.chat_mode == ChatMode::Hidden {
            client.send_message(Text::translate(CHAT_DISABLED_OPTIONS, []).color(Color::RED));
            continue;
        }

        // Ensure we are receiving chat messages in order
        if message.timestamp < state.last_message_timestamp {
            warn!(
                "{:?} sent out-of-order chat: '{:?}'",
                username.0,
                message.message.as_ref()
            );
            commands.add(DisconnectClient {
                client: message.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_OUT_OF_ORDER_CHAT, []),
            });
            continue;
        }

        state.last_message_timestamp = message.timestamp;

        // Validate the message acknowledgements
        match state
            .validator
            .validate(&message.acknowledgements, message.message_index)
        {
            None => {
                warn!("Failed to validate acknowledgements from `{}`", username.0);
                commands.add(DisconnectClient {
                    client: message.client,
                    reason: Text::translate(MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED, []),
                });
                continue;
            }
            Some(mut last_seen) => {
                // This whole process should probably be done on another thread similarly to
                // chunk loading in the 'terrain.rs' example, as this is what
                // the notchian server does

                let Some(link) = &state.chain.next_link() else {
                    client.send_message(Text::translate(
                        CHAT_DISABLED_CHAIN_BROKEN,
                        [],
                    ).color(Color::RED));
                    continue;
                };

                let Some(session) = &state.session else {
                    commands.add(DisconnectClient {
                        client: message.client,
                        reason: Text::translate(CHAT_DISABLED_MISSING_PROFILE_KEY, [])
                    });
                    continue;
                };

                // Verify that the player's session has not expired
                if session.is_expired() {
                    commands.add(DisconnectClient {
                        client: message.client,
                        reason: Text::translate(CHAT_DISABLED_EXPIRED_PROFILE_KEY, []),
                    });
                    continue;
                }

                // Verify that the chat message is signed
                let Some(message_signature) = &message.signature else {
                    commands.add(DisconnectClient {
                        client: message.client,
                        reason: Text::translate(MULTIPLAYER_DISCONNECT_UNSIGNED_CHAT, [])
                    });
                    continue;
                };

                // Create the hash digest used to verify the chat message
                let mut hasher = Sha256::new_with_prefix([0u8, 0, 0, 1]);

                // Update the hash with the player's message chain state
                link.update_hash(&mut hasher);

                // Update the hash with the message contents
                hasher.update(message.salt.to_be_bytes());
                hasher.update((message.timestamp / 1000).to_be_bytes());
                let bytes = message.message.as_bytes();
                hasher.update((bytes.len() as u32).to_be_bytes());
                hasher.update(bytes);
                hasher.update((last_seen.len() as u32).to_be_bytes());
                for sig in last_seen.iter() {
                    hasher.update(sig);
                }
                let hashed = hasher.finalize();

                // Verify the chat message using the player's session public key and hashed data
                // against the message signature
                if session
                    .public_key
                    .verify(
                        PaddingScheme::new_pkcs1v15_sign::<Sha256>(),
                        &hashed,
                        message_signature.as_ref(),
                    )
                    .is_err()
                {
                    commands.add(DisconnectClient {
                        client: message.client,
                        reason: Text::translate(MULTIPLAYER_DISCONNECT_UNSIGNED_CHAT, []),
                    });
                    continue;
                }

                // Create a list of messages that have been seen by the client
                let previous = last_seen
                    .iter()
                    .map(|sig| match state.signature_storage.index_of(sig) {
                        Some(index) => MessageSignature::ByIndex(index),
                        None => MessageSignature::BySignature(sig),
                    })
                    .collect::<Vec<_>>();

                info!("{}: {}", username.0, message.message.as_ref());

                instance.write_packet(&ChatMessageS2c {
                    sender: link.sender,
                    index: VarInt(link.index),
                    message_signature: Some(message_signature.as_ref()),
                    message: message.message.as_ref(),
                    time_stamp: message.timestamp,
                    salt: message.salt,
                    previous_messages: previous,
                    unsigned_content: None,
                    filter_type: MessageFilterType::PassThrough,
                    chat_type: VarInt(0),
                    network_name: Text::from(username.0.clone()).into(),
                    network_target_name: None,
                });

                // Update the other clients' chat states
                for mut state in states.iter_mut() {
                    // Add pending acknowledgement
                    state.add_pending(&mut last_seen, message_signature.as_ref());
                    if state.validator.message_count() > 4096 {
                        commands.add(DisconnectClient {
                            client: message.client,
                            reason: Text::translate(
                                MULTIPLAYER_DISCONNECT_TOO_MANY_PENDING_CHATS,
                                [],
                            ),
                        });
                        continue;
                    }
                }
            }
        }
    }
}

fn handle_command_events(
    mut clients: Query<&mut Client>,
    mut commands: EventReader<CommandExecution>,
) {
    for command in commands.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(command.client) else {
            warn!("Unable to find client for message: {:?}", command);
            continue;
        };

        let message = command.command.to_string();

        let formatted =
            "You sent the command ".into_text() + ("/".into_text() + (message).into_text()).bold();

        client.send_message(formatted);
    }
}

fn handle_chat_settings_event(
    mut states: Query<&mut ChatState>,
    mut settings: EventReader<ClientSettings>,
) {
    for ClientSettings {
        client, chat_mode, ..
    } in settings.iter()
    {
        let Ok(mut state) = states.get_component_mut::<ChatState>(*client) else {
            warn!("Unable to find chat state for client");
            continue;
        };

        state.chat_mode = *chat_mode;
    }
}
