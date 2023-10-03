#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

pub mod command;
pub mod message;

#[cfg(feature = "secure")]
use std::collections::VecDeque;
use std::time::SystemTime;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tracing::warn;
use valence_lang::keys::{CHAT_DISABLED_OPTIONS, DISCONNECT_GENERIC_REASON};
use valence_protocol::packets::play::client_settings_c2s::ChatMode;
use valence_protocol::packets::play::{ChatMessageC2s, CommandExecutionC2s};
use valence_registry::chat_type::ChatTypePlugin;
use valence_server::client::{Client, SpawnClientsSet};
use valence_server::client_settings::ClientSettings;
use valence_server::event_loop::{EventLoopPreUpdate, PacketEvent};
use valence_server::protocol::packets::play::chat_message_s2c::{
    MessageFilterType, MessageSignature,
};
use valence_server::protocol::packets::play::{ChatMessageS2c, ProfilelessChatMessageS2c};
use valence_server::protocol::WritePacket;
use valence_text::{Color, Text};
#[cfg(feature = "secure")]
use {
    crate::command::ArgumentSignature,
    crate::message::ChatMessageType,
    anyhow::bail,
    rsa::pkcs1v15::Pkcs1v15Sign,
    rsa::pkcs8::DecodePublicKey,
    rsa::RsaPublicKey,
    rustc_hash::{FxHashMap, FxHashSet},
    sha1::{Digest, Sha1},
    sha2::Sha256,
    uuid::Uuid,
    valence_lang::keys::{
        CHAT_DISABLED_CHAIN_BROKEN, CHAT_DISABLED_EXPIRED_PROFILE_KEY,
        CHAT_DISABLED_MISSING_PROFILE_KEY, MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED,
        MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY,
        MULTIPLAYER_DISCONNECT_INVALID_PUBLIC_KEY_SIGNATURE,
        MULTIPLAYER_DISCONNECT_OUT_OF_ORDER_CHAT, MULTIPLAYER_DISCONNECT_TOO_MANY_PENDING_CHATS,
        MULTIPLAYER_DISCONNECT_UNSIGNED_CHAT,
    },
    valence_player_list::{ChatSession, PlayerListEntry},
    valence_server::client::{DisconnectClient, Username},
    valence_server::protocol::packets::play::{MessageAcknowledgmentC2s, PlayerSessionC2s},
    valence_server_common::UniqueId,
    valence_text::IntoText,
};

use crate::command::CommandExecutionEvent;
use crate::message::{ChatMessageEvent, SendMessage};

#[cfg(feature = "secure")]
const MOJANG_KEY_DATA: &[u8] = include_bytes!("../yggdrasil_session_pubkey.der");

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(ChatTypePlugin)
            .add_systems(PreUpdate, init_chat_states.after(SpawnClientsSet))
            .add_systems(
                EventLoopPreUpdate,
                (
                    #[cfg(feature = "secure")]
                    handle_acknowledgement_packets,
                    #[cfg(not(feature = "secure"))]
                    handle_message_packets,
                    handle_command_packets,
                ),
            );

        #[cfg(feature = "secure")]
        {
            let mojang_pub_key = RsaPublicKey::from_public_key_der(MOJANG_KEY_DATA)
                .expect("Error creating Mojang public key");

            app.insert_resource(MojangServicesState::new(mojang_pub_key))
                .add_systems(
                    EventLoopPreUpdate,
                    (handle_session_packets, handle_message_packets).chain(),
                );
        }

        command::build(app);
        message::build(app);
    }
}

#[cfg(feature = "secure")]
#[derive(Resource)]
struct MojangServicesState {
    public_key: RsaPublicKey,
}

#[cfg(feature = "secure")]
impl MojangServicesState {
    fn new(public_key: RsaPublicKey) -> Self {
        Self { public_key }
    }
}

#[cfg(feature = "secure")]
#[derive(Debug, Component)]
pub struct ChatState {
    pub last_message_timestamp: u64,
    validator: AcknowledgementValidator,
    chain: MessageChain,
    signature_storage: MessageSignatureStorage,
}

#[cfg(feature = "secure")]
impl Default for ChatState {
    fn default() -> Self {
        Self {
            last_message_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Unable to get Unix time")
                .as_millis() as u64,
            validator: AcknowledgementValidator::new(),
            chain: MessageChain::new(),
            signature_storage: MessageSignatureStorage::new(),
        }
    }
}

#[cfg(feature = "secure")]
impl ChatState {
    pub fn send_chat_message(
        &mut self,
        client: &mut Client,
        username: &Username,
        message: &ChatMessageEvent,
    ) -> anyhow::Result<()> {
        match &message.message_type {
            ChatMessageType::Signed {
                salt,
                signature,
                message_index,
                last_seen,
                sender,
            } => {
                // Create a list of messages that have been seen by the client.
                let previous = last_seen
                    .iter()
                    .map(|sig| match self.signature_storage.index_of(sig) {
                        Some(index) => MessageSignature::ByIndex(index),
                        None => MessageSignature::BySignature(sig),
                    })
                    .collect::<Vec<_>>();

                client.write_packet(&ChatMessageS2c {
                    sender: *sender,
                    index: (*message_index).into(),
                    message_signature: Some((*signature).as_ref()),
                    message: message.message.as_ref().into(),
                    timestamp: message.timestamp,
                    salt: *salt,
                    previous_messages: previous,
                    unsigned_content: None,
                    filter_type: MessageFilterType::PassThrough,
                    chat_type: 0.into(), // TODO: Make chat type for player messages selectable
                    network_name: Text::from(username.0.clone()).into(),
                    network_target_name: None,
                });
                // Add pending acknowledgement.
                self.add_pending(last_seen, signature);
                if self.validator.message_count() > 4096 {
                    warn!("User has too many pending chats `{}`", username.0);
                    bail!(MULTIPLAYER_DISCONNECT_TOO_MANY_PENDING_CHATS);
                }
            }
            ChatMessageType::Unsigned => client.write_packet(&ProfilelessChatMessageS2c {
                message: Text::from(message.message.to_string()).into(),
                chat_type: 0.into(),
                chat_type_name: Text::from(username.0.clone()).into(),
                target_name: None,
            }),
        }
        Ok(())
    }

    /// Updates the chat state's previously seen signatures with a new one
    /// `signature`.
    fn add_pending(&mut self, last_seen: &[[u8; 256]], signature: &[u8; 256]) {
        self.signature_storage.add(last_seen, signature);
        self.validator.add_pending(signature);
    }
}

#[cfg(feature = "secure")]
#[derive(Clone, Debug)]
struct AcknowledgementValidator {
    messages: Vec<Option<AcknowledgedMessage>>,
    last_signature: Option<[u8; 256]>,
}

#[cfg(feature = "secure")]
impl AcknowledgementValidator {
    fn new() -> Self {
        Self {
            messages: vec![None; 20],
            last_signature: None,
        }
    }

    /// Add a message pending acknowledgement via its `signature`.
    fn add_pending(&mut self, signature: &[u8; 256]) {
        // Attempting to add the last signature again.
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
    fn remove_until(&mut self, index: i32) -> bool {
        // Ensure that there will still be 20 messages in the array.
        if index >= 0 && index <= (self.messages.len() - 20) as i32 {
            self.messages.drain(0..index as usize);
            debug_assert!(
                self.messages.len() >= 20,
                "Message validator 'messages' shrunk!"
            );
            return true;
        }
        false
    }

    /// Validate a set of `acknowledgements` offset by `message_index`.
    ///
    /// Returns a [`VecDeque`] of acknowledged message signatures if the
    /// `acknowledgements` are valid and `None` if they are invalid.
    fn validate(
        &mut self,
        acknowledgements: &[u8; 3],
        message_index: i32,
    ) -> anyhow::Result<Vec<[u8; 256]>> {
        if !self.remove_until(message_index) {
            bail!("Invalid message index");
        }

        let acknowledged_count = {
            let mut sum = 0u32;
            for byte in acknowledgements {
                sum += byte.count_ones();
            }
            sum as usize
        };

        if acknowledged_count > 20 {
            bail!("Too many message acknowledgements, protocol error?");
        }

        let mut list = Vec::with_capacity(acknowledged_count);
        for i in 0..20 {
            let acknowledgement = acknowledgements[i >> 3] & (0b1 << (i % 8)) != 0;
            let acknowledged_message = &mut self.messages[i];
            // Client has acknowledged the i-th message
            if acknowledgement {
                // The validator has the i-th message
                if let Some(m) = acknowledged_message {
                    m.pending = false;
                    list.push(m.signature);
                } else {
                    // Client has acknowledged a non-existing message
                    bail!("Client has acknowledged a non-existing message");
                }
            } else {
                // Client has not acknowledged the i-th message
                if matches!(acknowledged_message, Some(m) if !m.pending) {
                    // The validator has an i-th message that has been validated but the client
                    // claims that it hasn't been validated yet
                    bail!(
                        "The validator has an i-th message that has been validated but the client \
                         claims that it hasn't been validated yet"
                    );
                }
                // Honestly not entirely sure why this is done
                *acknowledged_message = None;
            }
        }
        Ok(list)
    }

    /// The number of pending messages in the validator.
    fn message_count(&self) -> usize {
        self.messages.len()
    }
}

#[cfg(feature = "secure")]
#[derive(Clone, Debug)]
struct AcknowledgedMessage {
    signature: [u8; 256],
    pending: bool,
}

#[cfg(feature = "secure")]
#[derive(Clone, Default, Debug)]
struct MessageChain {
    link: Option<MessageLink>,
}

#[cfg(feature = "secure")]
impl MessageChain {
    fn new() -> Self {
        Self::default()
    }

    fn next_link(&mut self) -> Option<MessageLink> {
        match &mut self.link {
            None => self.link,
            Some(current) => {
                let temp = *current;
                current.index = current.index.wrapping_add(1);
                Some(temp)
            }
        }
    }
}

#[cfg(feature = "secure")]
#[derive(Copy, Clone, Debug)]
struct MessageLink {
    index: i32,
    sender: Uuid,
    session_id: Uuid,
}

#[cfg(feature = "secure")]
impl MessageLink {
    fn update_hash(&self, hasher: &mut impl Digest) {
        hasher.update(self.sender.into_bytes());
        hasher.update(self.session_id.into_bytes());
        hasher.update(self.index.to_be_bytes());
    }
}

#[cfg(feature = "secure")]
#[derive(Clone, Debug)]
struct MessageSignatureStorage {
    signatures: [Option<[u8; 256]>; 128],
    indices: FxHashMap<[u8; 256], i32>,
}

#[cfg(feature = "secure")]
impl Default for MessageSignatureStorage {
    fn default() -> Self {
        Self {
            signatures: [None; 128],
            indices: FxHashMap::default(),
        }
    }
}

#[cfg(feature = "secure")]
impl MessageSignatureStorage {
    fn new() -> Self {
        Self::default()
    }

    /// Get the index of the `signature` in the storage if it exists.
    fn index_of(&self, signature: &[u8; 256]) -> Option<i32> {
        self.indices.get(signature).copied()
    }

    /// Update the signature storage according to `last_seen` while adding
    /// `signature` to the storage.
    ///
    /// Warning: this consumes `last_seen`.
    fn add(&mut self, last_seen: &[[u8; 256]], signature: &[u8; 256]) {
        let mut sig_set = FxHashSet::default();

        last_seen
            .iter()
            .chain(std::iter::once(signature))
            .for_each(|sig| {
                sig_set.insert(*sig);
            });

        let mut retained_sigs = VecDeque::new();
        let mut index = 0usize;
        let mut seen_iter = last_seen.iter().chain(std::iter::once(signature)).rev();

        while let Some(seen_sig) = seen_iter.next().or(retained_sigs.pop_front().as_ref()) {
            if index > 127 {
                return;
            }
            // Remove the old signature
            let previous_sig = self.signatures[index];
            // Add the new signature
            self.signatures[index] = Some(*seen_sig);
            self.indices.insert(*seen_sig, index as i32);
            // Reinsert old signature if it is not already in `last_seen`
            if let Some(data) = previous_sig {
                // Remove the index for the old sig
                self.indices.remove(&data);
                // If the old sig is still unique, reinsert
                if sig_set.insert(data) {
                    retained_sigs.push_back(data);
                }
            }
            index += 1;
        }
    }
}

#[cfg(feature = "secure")]
fn init_chat_states(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in clients.iter() {
        commands.entity(entity).insert(ChatState::default());
    }
}

#[cfg(feature = "secure")]
fn handle_session_packets(
    services_state: Res<MojangServicesState>,
    mut clients: Query<(&UniqueId, &Username, &mut ChatState), With<PlayerListEntry>>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands,
) {
    for packet in packets.iter() {
        let Some(session) = packet.decode::<PlayerSessionC2s>() else {
            continue;
        };

        let Ok((uuid, username, mut state)) = clients.get_mut(packet.client) else {
            warn!("Unable to find client in player list for session");
            continue;
        };

        // Verify that the session key has not expired.
        if SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Unable to get Unix time")
            .as_millis()
            >= session.0.expires_at as u128
        {
            warn!("Failed to validate profile key: expired public key");
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY, []),
            });
            continue;
        }

        // Hash the session data using the SHA-1 algorithm.
        let mut hasher = Sha1::new();
        hasher.update(uuid.0.into_bytes());
        hasher.update(session.0.expires_at.to_be_bytes());
        hasher.update(&session.0.public_key_data);
        let hash = hasher.finalize();

        // Verify the session data using Mojang's public key and the hashed session data
        // against the message signature.
        if services_state
            .public_key
            .verify(
                Pkcs1v15Sign::new::<Sha1>(), // PaddingScheme::new_pkcs1v15_sign::<Sha1>(),
                &hash,
                session.0.key_signature.as_ref(),
            )
            .is_err()
        {
            warn!("Failed to validate profile key: invalid public key signature");
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_INVALID_PUBLIC_KEY_SIGNATURE, []),
            });
        }

        // Decode the player's session public key from the data.
        if let Ok(public_key) =
            RsaPublicKey::from_public_key_der(session.0.public_key_data.as_ref())
        {
            // Update the player's chat state data with the new player session data.
            state.chain.link = Some(MessageLink {
                index: 0,
                sender: uuid.0,
                session_id: session.0.session_id,
            });

            // Add the chat session data to player.
            // The player list will then send this new session data to the other clients.
            commands.entity(packet.client).insert(ChatSession {
                public_key,
                session_data: session.0.into_owned(),
            });
        } else {
            // This shouldn't happen considering that it is highly unlikely that Mojang
            // would provide the client with a malformed key. By this point the
            // key signature has been verified.
            warn!("Received malformed profile key data from '{}'", username.0);
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(
                    DISCONNECT_GENERIC_REASON,
                    ["Malformed profile key data".color(Color::RED)],
                ),
            });
        }
    }
}

#[cfg(feature = "secure")]
fn handle_acknowledgement_packets(
    mut clients: Query<(&Username, &mut ChatState)>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands,
) {
    for packet in packets.iter() {
        let Some(acknowledgement) = packet.decode::<MessageAcknowledgmentC2s>() else {
            continue;
        };

        let Ok((username, mut state)) = clients.get_mut(packet.client) else {
            warn!("Unable to find client for acknowledgement");
            continue;
        };

        if !state
            .validator
            .remove_until(acknowledgement.message_index.0)
        {
            warn!(
                "Failed to validate message acknowledgement from '{:?}'",
                username.0
            );
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED, []),
            });
            continue;
        }
    }
}

#[cfg(feature = "secure")]
fn handle_message_packets(
    mut clients: Query<
        (&mut ChatState, &mut Client, &Username, &ClientSettings),
        With<PlayerListEntry>,
    >,
    sessions: Query<&ChatSession, With<PlayerListEntry>>,
    mut packets: EventReader<PacketEvent>,
    mut message_events: EventWriter<ChatMessageEvent>,
    mut commands: Commands,
) {
    for packet in packets.iter() {
        let Some(message) = packet.decode::<ChatMessageC2s>() else {
            continue;
        };

        let Ok((mut state, mut client, username, settings)) = clients.get_mut(packet.client) else {
            warn!("Unable to find client for message '{:?}'", message);
            continue;
        };

        // Ensure that the client isn't sending messages while their chat is hidden.
        if settings.chat_mode == ChatMode::Hidden {
            client.send_game_message(Text::translate(CHAT_DISABLED_OPTIONS, []).color(Color::RED));
            continue;
        }

        // Ensure we are receiving chat messages in order.
        if message.timestamp < state.last_message_timestamp {
            warn!(
                "{:?} sent out-of-order chat: '{:?}'",
                username.0, message.message
            );
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_OUT_OF_ORDER_CHAT, []),
            });
            continue;
        }

        state.last_message_timestamp = message.timestamp;

        // Check if the message is signed
        let Some(message_signature) = message.signature else {
            // TODO: Cleanup
            warn!("Received unsigned chat message from `{}`", username.0);
            /*commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_UNSIGNED_CHAT, [])
            });*/
            message_events.send(ChatMessageEvent {
                client: packet.client,
                message: message.message.0.into(),
                timestamp: message.timestamp,
                message_type: message::ChatMessageType::Unsigned,
            });
            continue;
        };

        // Validate the message acknowledgements.
        let last_seen = match state
            .validator
            .validate(&message.acknowledgement.0, message.message_index.0)
        {
            Err(error) => {
                warn!(
                    "Failed to validate acknowledgements from `{}`: {}",
                    username.0, error
                );
                commands.add(DisconnectClient {
                    client: packet.client,
                    reason: Text::translate(MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED, []),
                });
                continue;
            }
            Ok(last_seen) => last_seen,
        };

        let Some(link) = &state.chain.next_link() else {
            client.send_game_message(
                Text::translate(CHAT_DISABLED_CHAIN_BROKEN, []).color(Color::RED),
            );
            continue;
        };

        let Ok(chat_session) = sessions.get(packet.client) else {
            warn!("Player `{}` doesn't have a chat session", username.0);
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(CHAT_DISABLED_MISSING_PROFILE_KEY, []),
            });
            continue;
        };

        // Verify that the player's session has not expired.
        if SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Unable to get Unix time")
            .as_millis()
            >= chat_session.session_data.expires_at as u128
        {
            warn!("Player `{}` has an expired chat session", username.0);
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(CHAT_DISABLED_EXPIRED_PROFILE_KEY, []),
            });
            continue;
        }

        // Create the hash digest used to verify the chat message.
        let mut hasher = Sha256::new_with_prefix([0u8, 0, 0, 1]);

        // Update the hash with the player's message chain state.
        link.update_hash(&mut hasher);

        // Update the hash with the message contents.
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
        // against the message signature.
        if chat_session
            .public_key
            .verify(
                Pkcs1v15Sign::new::<Sha256>(), // PaddingScheme::new_pkcs1v15_sign::<Sha256>(),
                &hashed,
                message_signature.as_ref(),
            )
            .is_err()
        {
            warn!("Failed to verify chat message from `{}`", username.0);
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_UNSIGNED_CHAT, []),
            });
            continue;
        }

        message_events.send(ChatMessageEvent {
            client: packet.client,
            message: message.message.0.into(),
            timestamp: message.timestamp,
            message_type: message::ChatMessageType::Signed {
                salt: message.salt,
                signature: (*message_signature).into(),
                message_index: link.index,
                sender: link.sender,
                last_seen,
            },
        });
    }
}

#[cfg(not(feature = "secure"))]
fn handle_message_packets(
    mut clients: Query<(&mut Client, &ClientSettings)>,
    mut packets: EventReader<PacketEvent>,
    mut message_events: EventWriter<ChatMessageEvent>,
) {
    for packet in packets.iter() {
        let Some(message) = packet.decode::<ChatMessageC2s>() else {
            continue;
        };

        let Ok((mut client, settings)) = clients.get_mut(packet.client) else {
            warn!("Unable to find client for message '{:?}'", message);
            continue;
        };

        // Ensure that the client isn't sending messages while their chat is hidden.
        if settings.chat_mode == ChatMode::Hidden {
            client.send_game_message(Text::translate(CHAT_DISABLED_OPTIONS, []).color(Color::RED));
            continue;
        }

        message_events.send(ChatMessageEvent {
            client: packet.client,
            message: message.message.into(),
            timestamp: message.timestamp,
        })
    }
}

fn handle_command_packets(
    mut clients: Query<
        (&mut ChatState, &mut Client, &Username, &ClientSettings),
        With<PlayerListEntry>,
    >,
    _sessions: Query<&ChatSession, With<PlayerListEntry>>,
    mut packets: EventReader<PacketEvent>,
    mut command_events: EventWriter<CommandExecutionEvent>,
    mut commands: Commands,
) {
    for packet in packets.iter() {
        let Some(command) = packet.decode::<CommandExecutionC2s>() else {
            continue;
        };

        let Ok((mut state, mut client, username, settings)) = clients.get_mut(packet.client) else {
            warn!("Unable to find client for message '{:?}'", command);
            continue;
        };

        // Ensure that the client isn't sending messages while their chat is hidden.
        if settings.chat_mode == ChatMode::Hidden {
            client.send_game_message(Text::translate(CHAT_DISABLED_OPTIONS, []).color(Color::RED));
            continue;
        }

        // Ensure we are receiving chat messages in order.
        if command.timestamp < state.last_message_timestamp {
            warn!(
                "{:?} sent out-of-order chat: '{:?}'",
                username.0, command.command
            );
            commands.add(DisconnectClient {
                client: packet.client,
                reason: Text::translate(MULTIPLAYER_DISCONNECT_OUT_OF_ORDER_CHAT, []),
            });
            continue;
        }

        state.last_message_timestamp = command.timestamp;

        // Validate the message acknowledgements.
        let _last_seen = match state
            .validator
            .validate(&command.acknowledgement.0, command.message_index.0)
        {
            Err(error) => {
                warn!(
                    "Failed to validate acknowledgements from `{}`: {}",
                    username.0, error
                );
                commands.add(DisconnectClient {
                    client: packet.client,
                    reason: Text::translate(MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED, []),
                });
                continue;
            }
            Ok(last_seen) => last_seen,
        };

        // TODO: Implement proper argument verification
        // This process will invlove both `_sessions` and `_last_seen`

        warn!("{:?}", command);
        command_events.send(CommandExecutionEvent {
            client: packet.client,
            command: command.command.0.into(),
            timestamp: command.timestamp,
            salt: command.salt,
            argument_signatures: command
                .argument_signatures
                .0
                .iter()
                .map(|sig| ArgumentSignature {
                    name: sig.argument_name.0.into(),
                    signature: (*sig.signature).into(),
                })
                .collect(),
        })
    }
}
