use std::time::SystemTime;

use bevy_app::App;
use rsa::pkcs8::DecodePublicKey;
use rsa::{PaddingScheme, PublicKey, RsaPublicKey};
use sha1::{Digest, Sha1};
use tracing::{debug, info, Level, warn};
use valence::client::despawn_disconnected_clients;
use valence::client::event::{
    default_event_handler, ChatMessage, ClientSettings, CommandExecution, MessageAcknowledgment,
    PlayerSession,
};
use valence::prelude::*;
use valence::protocol::packet::c2s::play::client_settings::ChatMode;
use valence::protocol::translation_key::{
    CHAT_DISABLED_OPTIONS, MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED,
    MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY, MULTIPLAYER_DISCONNECT_INVALID_PUBLIC_KEY_SIGNATURE,
    MULTIPLAYER_DISCONNECT_OUT_OF_ORDER_CHAT,
};

const SPAWN_Y: i32 = 64;

const MOJANG_KEY_DATA: &[u8] = include_bytes!("./yggdrasil_session_pubkey.der");

#[derive(Resource)]
struct MojangServicesState {
    public_key: RsaPublicKey,
}

#[derive(Debug, Default)]
struct AcknowledgementValidator {
    messages: Vec<Option<AcknowledgedMessage>>,
    last_signature: Option<Box<[u8; 256]>>,
}

#[derive(Debug)]
struct AcknowledgedMessage {
    pub signature: Box<[u8; 256]>,
    pub pending: bool,
}

impl AcknowledgementValidator {
    pub fn remove_until(&mut self, index: i32) -> bool {
        if index >= 0 && index <= (self.messages.len() - 20) as i32 {
            self.messages.drain(0..index as usize);
            return true;
        }
        false
    }

    pub fn validate(
        &mut self,
        acknowledgements: &[u8; 3],
        message_index: i32,
    ) -> Option<Vec<[u8; 256]>> {
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

        let mut list = Vec::with_capacity(acknowledged_count);
        for i in 0..20 {
            let acknowledgement = acknowledgements[i >> 3] & (0b1 << (i % 8)) != 0;
            let acknowledged_message = unsafe { self.messages.get_unchecked_mut(i) };
            if acknowledgement {
                // Client has acknowledged the i-th message
                if let Some(m) = acknowledged_message {
                    // The validator has the i-th message
                    m.pending = false;
                    list.push(*m.signature);
                } else {
                    // Client has acknowledged a non-existing message
                    return None;
                }
            }
            // Client has not acknowledged the i-th message
            if matches!(acknowledged_message, Some(m) if !m.pending) {
                // The validator has an i-th message that has already been validated
                return None;
            }
            // If the validator has doesn't have an i-th message or it is pending
            unsafe {
                let m = self.messages.get_unchecked_mut(i);
                *m = None;
            }
        }
        Some(list)
    }
}

#[derive(Component)]
struct ChatState {
    last_message_timestamp: u64,
    chat_mode: ChatMode,
    validator: AcknowledgementValidator,
    public_key: Option<RsaPublicKey>,
}

pub fn main() {
    tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();

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
            )
                .in_schedule(EventLoopSchedule)
        )
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(mut commands: Commands, server: Res<Server>) {
    let mojang_pub_key = RsaPublicKey::from_public_key_der(MOJANG_KEY_DATA)
        .expect("Error creating Mojang public key");

   commands.insert_resource(MojangServicesState { public_key: mojang_pub_key });

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
    mut commands: Commands,
    mut clients: Query<(Entity, &mut Client), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.single();

    for (entity, mut client) in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_game_mode(GameMode::Adventure);
        client.send_message("Welcome to Valence! Talk about something.".italic());
        client.set_instance(instance);

        let mut state = ChatState {
            last_message_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Unable to get Unix time")
                .as_millis() as u64,
            chat_mode: ChatMode::Enabled,
            validator: Default::default(),
            public_key: None,
        };

        commands.entity(entity).insert(state);

        info!("{} logged in!", client.username());
    }
}

fn handle_session_events(
    services_state: Res<MojangServicesState>,
    player_list: ResMut<PlayerList>,
    mut clients: Query<&mut Client>,
    mut states: Query<&mut ChatState>,
    mut sessions: EventReader<PlayerSession>,
) {
    let pl = player_list.into_inner();

    for session in sessions.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(session.client) else {
            warn!("Unable to find client for session");
            continue;
        };

        let Some(player_entry) = pl.get_mut(client.uuid()) else {
            warn!("Unable to find '{}' in the player list", client.username());
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
            client.kick(Text::translate(
                MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY,
                [],
            ));
            continue;
        }

        // Serialize the session data
        let mut serialized = Vec::with_capacity(318);
        serialized.extend_from_slice(client.uuid().into_bytes().as_slice());
        serialized.extend_from_slice(session.session_data.expires_at.to_be_bytes().as_ref());
        serialized.extend_from_slice(session.session_data.public_key_data.as_ref());

        // Hash the session data using the SHA-1 algorithm
        let mut hasher = Sha1::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();

        // Verify the session key signature using the hashed session data and the Mojang
        // public key
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
            client.kick(Text::translate(
                MULTIPLAYER_DISCONNECT_INVALID_PUBLIC_KEY_SIGNATURE,
                [],
            ));
        }

        let Ok(mut state) = states.get_component_mut::<ChatState>(session.client) else {
            warn!("Unable to find chat state for client '{:?}'", client.username());
            continue;
        };

        if let Ok(public_key) = RsaPublicKey::from_public_key_der(session.session_data.public_key_data.as_ref()) {
            state.public_key = Some(public_key);
        } else {
            // This shouldn't happen considering that it is highly unlikely that Mojang would
            // provide the client with a malformed key. By this point the key signature has
            // been verified
            warn!("Received malformed profile key data from '{}'", client.username());
            client.kick(Text::translate(
                MULTIPLAYER_DISCONNECT_EXPIRED_PUBLIC_KEY,
                ["Malformed profile key data".color(Color::RED)],
            ));
        }

        player_entry.set_chat_data(Some(session.session_data.clone()));
    }
}

fn handle_message_acknowledgement(
    mut clients: Query<&mut Client>,
    mut states: Query<&mut ChatState>,
    mut acknowledgements: EventReader<MessageAcknowledgment>,
) {
    for acknowledgement in acknowledgements.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(acknowledgement.client) else {
            warn!("Unable to find client for acknowledgement");
            continue;
        };

        let Ok(mut state) = states.get_component_mut::<ChatState>(acknowledgement.client) else {
            warn!("Unable to find chat state for client '{:?}'", client.username());
            continue;
        };

        if !state.validator.remove_until(acknowledgement.message_index) {
            warn!(
                "Failed to validate message acknowledgement from '{:?}'",
                client.username()
            );
            client.kick(Text::translate(
                MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED,
                [],
            ));
            continue;
        }

        debug!("Acknowledgement from '{:?}'", client.username());
    }
}

fn handle_message_events(
    player_list: ResMut<PlayerList>,
    mut clients: Query<&mut Client>,
    mut states: Query<&mut ChatState>,
    mut messages: EventReader<ChatMessage>,
) {
    let pl = player_list.into_inner();

    for message in messages.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(message.client) else {
            warn!("Unable to find client for message '{:?}'", message);
            continue;
        };

        let Ok(mut state) = states.get_component_mut::<ChatState>(message.client) else {
            warn!("Unable to find chat state for client '{:?}'", client.username());
            continue;
        };

        if state.chat_mode == ChatMode::Hidden {
            client.send_message(Text::translate(CHAT_DISABLED_OPTIONS, []).color(Color::RED));
            continue;
        }

        let message_text = message.message.to_string();

        if message.timestamp < state.last_message_timestamp {
            warn!(
                "{:?} sent out-of-order chat: '{:?}'",
                client.username(),
                message_text
            );
            client.kick(Text::translate(
                MULTIPLAYER_DISCONNECT_OUT_OF_ORDER_CHAT,
                [],
            ));
            continue;
        }

        state.last_message_timestamp = message.timestamp;

        let Some(player_entry) = pl.get_mut(client.uuid()) else {
            warn!("Unable to find '{}' in the player list", client.username());
            continue;
        };

        match state.validator.validate(&message.acknowledgements, message.message_index) {
            Some(last_seen) => {
                // This process should probably be done on another thread similarly to chunk loading
                // in the 'terrain.rs' example, as this is what the notchian server does
                
            }
            None => {
                warn!(
                    "Failed to validate acknowledgements from {:?}",
                    client.username()
                );
                client.kick(MULTIPLAYER_DISCONNECT_CHAT_VALIDATION_FAILED);
            }
        };

        info!("{}: {}", client.username(), message_text);

        // ############################################

        let formatted = format!("<{}>: ", client.username())
            .bold()
            .color(Color::YELLOW)
            + message_text.into_text().not_bold().color(Color::WHITE);

        // TODO: write message to instance buffer.
        for mut client in &mut clients {
            client.send_message(formatted.clone());
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

        debug!("Client settings: {:?}", chat_mode);
    }
}
