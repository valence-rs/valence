use std::cmp;

use anyhow::bail;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use bevy_ecs::system::SystemParam;
use glam::{DVec3, Vec3};
use paste::paste;
use tracing::warn;
use uuid::Uuid;
use valence_protocol::entity_meta::Pose;
use valence_protocol::packet::c2s::play::{
    ClientCommand, PlayerAbilitiesC2s, ResourcePackC2s, SeenAdvancements,
};
use valence_protocol::packet::C2sPlayPacket;
use valence_protocol::types::{
    Action, ChatMode, ClickContainerMode, CommandBlockMode, Difficulty, DiggingStatus,
    DisplayedSkinParts, EntityInteraction, Hand, MainHand, RecipeBookId, StructureBlockAction,
    StructureBlockFlags, StructureBlockMirror, StructureBlockMode, StructureBlockRotation,
};
use valence_protocol::{BlockFace, BlockPos, Ident, ItemStack};

use crate::client::Client;
use crate::entity::{EntityAnimation, EntityKind, McEntity, TrackedData};

#[derive(Clone, Debug)]
pub struct QueryBlockEntity {
    pub client: Entity,
    pub position: BlockPos,
    pub transaction_id: i32,
}

#[derive(Clone, Debug)]
pub struct ChangeDifficulty {
    pub client: Entity,
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug)]
pub struct MessageAcknowledgment {
    pub client: Entity,
    pub message_count: i32,
}

#[derive(Clone, Debug)]
pub struct ChatCommand {
    pub client: Entity,
    pub command: Box<str>,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub client: Entity,
    pub message: Box<str>,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct ChatPreview {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct PerformRespawn {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct RequestStats {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct UpdateSettings {
    pub client: Entity,
    /// e.g. en_US
    pub locale: Box<str>,
    /// The client side render distance, in chunks.
    ///
    /// The value is always in `2..=32`.
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    /// `true` if the client has chat colors enabled, `false` otherwise.
    pub chat_colors: bool,
    pub displayed_skin_parts: DisplayedSkinParts,
    pub main_hand: MainHand,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[derive(Clone, Debug)]
pub struct CommandSuggestionsRequest {
    pub client: Entity,
    pub transaction_id: i32,
    pub text: Box<str>,
}

#[derive(Clone, Debug)]
pub struct ClickContainerButton {
    pub client: Entity,
    pub window_id: i8,
    pub button_id: i8,
}

#[derive(Clone, Debug)]
pub struct ClickContainer {
    pub client: Entity,
    pub window_id: u8,
    pub state_id: i32,
    pub slot_id: i16,
    pub button: i8,
    pub mode: ClickContainerMode,
    pub slot_changes: Vec<(i16, Option<ItemStack>)>,
    pub carried_item: Option<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct CloseContainer {
    pub client: Entity,
    pub window_id: i8,
}

#[derive(Clone, Debug)]
pub struct PluginMessage {
    pub client: Entity,
    pub channel: Ident<Box<str>>,
    pub data: Box<[u8]>,
}

#[derive(Clone, Debug)]
pub struct EditBook {
    pub slot: i32,
    pub entries: Vec<Box<str>>,
    pub title: Option<Box<str>>,
}

#[derive(Clone, Debug)]
pub struct QueryEntityTag {
    pub client: Entity,
    pub transaction_id: i32,
    pub entity_id: i32,
}

/// Left or right click interaction with an entity's hitbox.
#[derive(Clone, Debug)]
pub struct InteractWithEntity {
    pub client: Entity,
    /// The raw ID of the entity being interacted with.
    pub entity_id: i32,
    /// If the client was sneaking during the interaction.
    pub sneaking: bool,
    /// The kind of interaction that occurred.
    pub interact: EntityInteraction,
}

#[derive(Clone, Debug)]
pub struct JigsawGenerate {
    pub client: Entity,
    pub position: BlockPos,
    pub levels: i32,
    pub keep_jigsaws: bool,
}

#[derive(Clone, Debug)]
pub struct LockDifficulty {
    pub client: Entity,
    pub locked: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerPosition {
    pub client: Entity,
    pub position: DVec3,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerPositionAndRotation {
    pub client: Entity,
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerRotation {
    pub client: Entity,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerOnGround {
    pub client: Entity,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct MoveVehicle {
    pub client: Entity,
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}

/// Sent whenever one of the other movement events is sent.
#[derive(Clone, Debug)]
pub struct MovePlayer {
    pub client: Entity,
    /// The position of the client prior to the event.
    pub old_position: DVec3,
    /// The position of the client after the event.
    pub position: DVec3,
    /// The yaw of the client prior to the event.
    pub old_yaw: f32,
    /// The yaw of the client after the event.
    pub yaw: f32,
    /// The pitch of the client prior to the event.
    pub old_pitch: f32,
    /// The pitch of the client after the event.
    pub pitch: f32,
    /// If the client was on ground prior to the event.
    pub old_on_ground: bool,
    /// If the client is on ground after the event.
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct StartSneaking {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StopSneaking {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct LeaveBed {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartSprinting {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StopSprinting {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartJumpWithHorse {
    pub client: Entity,
    /// The power of the horse jump in `0..=100`.
    pub jump_boost: u8,
}

#[derive(Clone, Debug)]
pub struct StopJumpWithHorse {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct OpenHorseInventory {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartFlyingWithElytra {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct PaddleBoat {
    pub client: Entity,
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}

#[derive(Clone, Debug)]
pub struct PickItem {
    pub client: Entity,
    pub slot_to_use: i32,
}

#[derive(Clone, Debug)]
pub struct PlaceRecipe {
    pub client: Entity,
    pub window_id: i8,
    pub recipe: Ident<Box<str>>,
    pub make_all: bool,
}

#[derive(Clone, Debug)]
pub struct StopFlying {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartFlying {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartDigging {
    pub client: Entity,
    pub position: BlockPos,
    pub face: BlockFace,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct CancelDigging {
    pub client: Entity,
    pub position: BlockPos,
    pub face: BlockFace,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct FinishDigging {
    pub client: Entity,
    pub position: BlockPos,
    pub face: BlockFace,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct DropItem {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct DropItemStack {
    pub client: Entity,
}

/// Eating food, pulling back bows, using buckets, etc.
#[derive(Clone, Debug)]
pub struct UpdateHeldItemState {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct SwapItemInHand {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct PlayerInput {
    pub client: Entity,
    pub sideways: f32,
    pub forward: f32,
    pub jump: bool,
    pub unmount: bool,
}

#[derive(Clone, Debug)]
pub struct Pong {
    pub client: Entity,
    pub id: i32,
}

#[derive(Clone, Debug)]
pub struct PlayerSession {
    pub client: Entity,
    pub session_id: Uuid,
    pub expires_at: i64,
    pub public_key_data: Box<[u8]>,
    pub key_signature: Box<[u8]>,
}

#[derive(Clone, Debug)]
pub struct ChangeRecipeBookSettings {
    pub client: Entity,
    pub book_id: RecipeBookId,
    pub book_open: bool,
    pub filter_active: bool,
}

#[derive(Clone, Debug)]
pub struct SetSeenRecipe {
    pub client: Entity,
    pub recipe_id: Ident<Box<str>>,
}

#[derive(Clone, Debug)]
pub struct RenameItem {
    pub client: Entity,
    pub name: Box<str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourcePackStatus {
    /// The client has accepted the server's resource pack.
    Accepted,
    /// The client has declined the server's resource pack.
    Declined,
    /// The client has successfully loaded the server's resource pack.
    Loaded,
    /// The client has failed to download the server's resource pack.
    FailedDownload,
}

impl From<ResourcePackC2s> for ResourcePackStatus {
    fn from(packet: ResourcePackC2s) -> Self {
        match packet {
            ResourcePackC2s::Accepted { .. } => Self::Accepted,
            ResourcePackC2s::Declined { .. } => Self::Declined,
            ResourcePackC2s::SuccessfullyLoaded { .. } => Self::Loaded,
            ResourcePackC2s::FailedDownload { .. } => Self::FailedDownload,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourcePackStatusChange {
    pub client: Entity,
    pub status: ResourcePackStatus,
}

#[derive(Clone, Debug)]
pub struct OpenAdvancementTab {
    pub client: Entity,
    pub tab_id: Ident<Box<str>>,
}

#[derive(Clone, Debug)]
pub struct CloseAdvancementScreen {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct SelectTrade {
    pub client: Entity,
    pub slot: i32,
}

#[derive(Clone, Debug)]
pub struct SetBeaconEffect {
    pub client: Entity,
    pub primary_effect: Option<i32>,
    pub secondary_effect: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct SetHeldItem {
    pub client: Entity,
    pub slot: i16,
}

#[derive(Clone, Debug)]
pub struct ProgramCommandBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub command: Box<str>,
    pub mode: CommandBlockMode,
    pub track_output: bool,
    pub conditional: bool,
    pub automatic: bool,
}

#[derive(Clone, Debug)]
pub struct ProgramCommandBlockMinecart {
    pub client: Entity,
    pub entity_id: i32,
    pub command: Box<str>,
    pub track_output: bool,
}

#[derive(Clone, Debug)]
pub struct SetCreativeModeSlot {
    pub client: Entity,
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct ProgramJigsawBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub name: Ident<Box<str>>,
    pub target: Ident<Box<str>>,
    pub pool: Ident<Box<str>>,
    pub final_state: Box<str>,
    pub joint_type: Box<str>,
}

#[derive(Clone, Debug)]
pub struct ProgramStructureBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub action: StructureBlockAction,
    pub mode: StructureBlockMode,
    pub name: Box<str>,
    pub offset_xyz: [i8; 3],
    pub size_xyz: [i8; 3],
    pub mirror: StructureBlockMirror,
    pub rotation: StructureBlockRotation,
    pub metadata: Box<str>,
    pub integrity: f32,
    pub seed: i64,
    pub flags: StructureBlockFlags,
}

#[derive(Clone, Debug)]
pub struct UpdateSign {
    pub client: Entity,
    pub position: BlockPos,
    pub lines: [Box<str>; 4],
}

#[derive(Clone, Debug)]
pub struct SwingArm {
    pub client: Entity,
    pub hand: Hand,
}

#[derive(Clone, Debug)]
pub struct TeleportToEntity {
    pub client: Entity,
    pub target: Uuid,
}

#[derive(Clone, Debug)]
pub struct UseItemOnBlock {
    pub client: Entity,
    /// The hand that was used
    pub hand: Hand,
    /// The location of the block that was interacted with
    pub position: BlockPos,
    /// The face of the block that was clicked
    pub face: BlockFace,
    /// The position inside of the block that was clicked on
    pub cursor_pos: Vec3,
    /// Whether or not the player's head is inside a block
    pub head_inside_block: bool,
    /// Sequence number for synchronization
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct UseItem {
    pub client: Entity,
    pub hand: Hand,
    pub sequence: i32,
}

macro_rules! events {
    (
        $(
            $group_number:tt {
                $($name:ident)*
            }
        )*
    ) => {
        /// Inserts [`Events`] resources into the world for each client event.
        pub(crate) fn register_client_events(world: &mut World) {
            $(
                $(
                    world.insert_resource(Events::<$name>::default());
                )*
            )*
        }

        paste! {
            fn update_all_event_buffers(events: &mut ClientEvents) {
                $(
                    let group = &mut events. $group_number;
                    $(
                        group.[< $name:snake >].update();
                    )*
                )*
            }

            pub(crate) type ClientEvents<'w, 's> = (
                $(
                    [< Group $group_number >]<'w, 's>,
                )*
            );

            $(
                #[derive(SystemParam)]
                pub(crate) struct [< Group $group_number >]<'w, 's> {
                    $(
                        [< $name:snake >]: ResMut<'w, Events<$name>>,
                    )*
                    #[system_param(ignore)]
                    _marker: std::marker::PhantomData<&'s ()>,
                }
            )*
        }
    }
}

// Events are grouped to get around the 16 system parameter maximum.
events! {
    0 {
        QueryBlockEntity
        ChangeDifficulty
        MessageAcknowledgment
        ChatCommand
        ChatMessage
        ChatPreview
        PerformRespawn
        RequestStats
        UpdateSettings
        CommandSuggestionsRequest
        ClickContainerButton
        ClickContainer
        CloseContainer
        PluginMessage
        EditBook
        QueryEntityTag
    }
    1 {
        InteractWithEntity
        JigsawGenerate
        LockDifficulty
        SetPlayerPosition
        SetPlayerPositionAndRotation
        SetPlayerRotation
        SetPlayerOnGround
        MoveVehicle
        MovePlayer
        StartSneaking
        StopSneaking
        LeaveBed
        StartSprinting
        StopSprinting
        StartJumpWithHorse
        StopJumpWithHorse
    }
    2 {
        OpenHorseInventory
        StartFlyingWithElytra
        PaddleBoat
        PickItem
        PlaceRecipe
        StopFlying
        StartFlying
        StartDigging
        CancelDigging
        FinishDigging
        DropItem
        DropItemStack
        UpdateHeldItemState
        SwapItemInHand
        PlayerInput
        Pong
    }
    3 {
        PlayerSession
        ChangeRecipeBookSettings
        SetSeenRecipe
        RenameItem
        ResourcePackStatusChange
        OpenAdvancementTab
        CloseAdvancementScreen
        SelectTrade
        SetBeaconEffect
        SetHeldItem
        ProgramCommandBlock
        ProgramCommandBlockMinecart
        SetCreativeModeSlot
    }
    4 {
        ProgramJigsawBlock
        ProgramStructureBlock
        UpdateSign
        SwingArm
        TeleportToEntity
        UseItemOnBlock
        UseItem
    }
}

pub(crate) fn event_loop_run_criteria(
    mut clients: Query<(Entity, &mut Client)>,
    mut clients_to_check: Local<Vec<Entity>>,
    mut events: ClientEvents,
) -> ShouldRun {
    if clients_to_check.is_empty() {
        // First run of the criteria. Prepare packets.

        update_all_event_buffers(&mut events);

        for (entity, client) in &mut clients {
            let client = client.into_inner();

            let Ok(bytes) = client.conn.try_recv() else {
                // Client is disconnected.
                client.is_disconnected = true;
                continue;
            };

            if bytes.is_empty() {
                // No data was received.
                continue;
            }

            client.dec.queue_bytes(bytes);

            match handle_one_packet(client, entity, &mut events) {
                Ok(had_packet) => {
                    if had_packet {
                        // We decoded one packet, but there might be more.
                        clients_to_check.push(entity);
                    }
                }
                Err(e) => {
                    // TODO: validate packets in separate systems.
                    warn!(
                        username = %client.username,
                        uuid = %client.uuid,
                        ip = %client.ip,
                        "failed to dispatch events: {e:#}"
                    );
                    client.is_disconnected = true;
                }
            }
        }
    } else {
        // Continue to filter the list of clients we need to check until there are none
        // left.
        clients_to_check.retain(|&entity| {
            let Ok((_, mut client)) = clients.get_mut(entity) else {
                // Client was deleted during the last run of the stage.
                return false;
            };

            match handle_one_packet(&mut client, entity, &mut events) {
                Ok(had_packet) => had_packet,
                Err(e) => {
                    // TODO: validate packets in separate systems.
                    warn!(
                        username = %client.username,
                        uuid = %client.uuid,
                        ip = %client.ip,
                        "failed to dispatch events: {e:#}"
                    );
                    client.is_disconnected = true;

                    false
                }
            }
        });
    }

    if clients_to_check.is_empty() {
        ShouldRun::No
    } else {
        ShouldRun::YesAndCheckAgain
    }
}

fn handle_one_packet(
    client: &mut Client,
    entity: Entity,
    events: &mut ClientEvents,
) -> anyhow::Result<bool> {
    let Some(pkt) = client.dec.try_next_packet::<C2sPlayPacket>()? else {
        // No packets to decode.
        return Ok(false);
    };

    match pkt {
        C2sPlayPacket::ConfirmTeleport(p) => {
            if client.pending_teleports == 0 {
                bail!("unexpected teleport confirmation");
            }

            let got = p.teleport_id.0 as u32;
            let expected = client
                .teleport_id_counter
                .wrapping_sub(client.pending_teleports);

            if got == expected {
                client.pending_teleports -= 1;
            } else {
                bail!("unexpected teleport ID (expected {expected}, got {got}");
            }
        }
        C2sPlayPacket::QueryBlockEntityTag(p) => {
            events.0.query_block_entity.send(QueryBlockEntity {
                client: entity,
                position: p.position,
                transaction_id: p.transaction_id.0,
            });
        }
        C2sPlayPacket::ChangeDifficulty(p) => {
            events.0.change_difficulty.send(ChangeDifficulty {
                client: entity,
                difficulty: p.new_difficulty,
            });
        }
        C2sPlayPacket::MessageAcknowledgmentC2s(p) => {
            events.0.message_acknowledgment.send(MessageAcknowledgment {
                client: entity,
                message_count: p.message_count.0,
            });
        }
        C2sPlayPacket::ChatCommand(p) => {
            events.0.chat_command.send(ChatCommand {
                client: entity,
                command: p.command.into(),
                timestamp: p.timestamp,
            });
        }
        C2sPlayPacket::ChatMessage(p) => {
            events.0.chat_message.send(ChatMessage {
                client: entity,
                message: p.message.into(),
                timestamp: p.timestamp,
            });
        }
        C2sPlayPacket::ClientCommand(p) => match p {
            ClientCommand::PerformRespawn => events
                .0
                .perform_respawn
                .send(PerformRespawn { client: entity }),
            ClientCommand::RequestStats => {
                events.0.request_stats.send(RequestStats { client: entity })
            }
        },
        C2sPlayPacket::ClientInformation(p) => {
            events.0.update_settings.send(UpdateSettings {
                client: entity,
                locale: p.locale.into(),
                view_distance: p.view_distance,
                chat_mode: p.chat_mode,
                chat_colors: p.chat_colors,
                displayed_skin_parts: p.displayed_skin_parts,
                main_hand: p.main_hand,
                enable_text_filtering: p.enable_text_filtering,
                allow_server_listings: p.allow_server_listings,
            });
        }
        C2sPlayPacket::CommandSuggestionsRequest(p) => {
            events
                .0
                .command_suggestions_request
                .send(CommandSuggestionsRequest {
                    client: entity,
                    transaction_id: p.transaction_id.0,
                    text: p.text.into(),
                });
        }
        C2sPlayPacket::ClickContainerButton(p) => {
            events.0.click_container_button.send(ClickContainerButton {
                client: entity,
                window_id: p.window_id,
                button_id: p.button_id,
            });
        }
        C2sPlayPacket::ClickContainer(p) => {
            events.0.click_container.send(ClickContainer {
                client: entity,
                window_id: p.window_id,
                state_id: p.state_id.0,
                slot_id: p.slot_idx,
                button: p.button,
                mode: p.mode,
                slot_changes: p.slots,
                carried_item: p.carried_item,
            });
        }
        C2sPlayPacket::CloseContainerC2s(p) => {
            events.0.close_container.send(CloseContainer {
                client: entity,
                window_id: p.window_id,
            });
        }
        C2sPlayPacket::PluginMessageC2s(p) => {
            events.0.plugin_message.send(PluginMessage {
                client: entity,
                channel: p.channel.into(),
                data: p.data.0.into(),
            });
        }
        C2sPlayPacket::EditBook(p) => {
            events.0.edit_book.send(EditBook {
                slot: p.slot.0,
                entries: p.entries.into_iter().map(Into::into).collect(),
                title: p.title.map(Box::from),
            });
        }
        C2sPlayPacket::QueryEntityTag(p) => {
            events.0.query_entity_tag.send(QueryEntityTag {
                client: entity,
                transaction_id: p.transaction_id.0,
                entity_id: p.entity_id.0,
            });
        }
        C2sPlayPacket::Interact(p) => {
            events.1.interact_with_entity.send(InteractWithEntity {
                client: entity,
                entity_id: p.entity_id.0,
                sneaking: p.sneaking,
                interact: p.interact,
            });
        }
        C2sPlayPacket::JigsawGenerate(p) => {
            events.1.jigsaw_generate.send(JigsawGenerate {
                client: entity,
                position: p.position,
                levels: p.levels.0,
                keep_jigsaws: p.keep_jigsaws,
            });
        }
        C2sPlayPacket::KeepAliveC2s(p) => {
            if client.got_keepalive {
                bail!("unexpected keepalive");
            } else if p.id != client.last_keepalive_id {
                bail!(
                    "keepalive IDs don't match (expected {}, got {})",
                    client.last_keepalive_id,
                    p.id
                );
            } else {
                client.got_keepalive = true;
                client.ping = client.keepalive_sent_time.elapsed().as_millis() as i32;
            }
        }
        C2sPlayPacket::LockDifficulty(p) => {
            events.1.lock_difficulty.send(LockDifficulty {
                client: entity,
                locked: p.locked,
            });
        }
        C2sPlayPacket::SetPlayerPosition(p) => {
            if client.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.set_player_position.send(SetPlayerPosition {
                client: entity,
                position: p.position.into(),
                on_ground: p.on_ground,
            });

            events.1.move_player.send(MovePlayer {
                client: entity,
                old_position: client.position,
                position: p.position.into(),
                old_yaw: client.yaw,
                yaw: client.yaw,
                old_pitch: client.pitch,
                pitch: client.pitch,
                old_on_ground: client.on_ground,
                on_ground: client.on_ground,
            });

            client.position = p.position.into();
            client.on_ground = p.on_ground;
        }
        C2sPlayPacket::SetPlayerPositionAndRotation(p) => {
            if client.pending_teleports != 0 {
                return Ok(false);
            }

            events
                .1
                .set_player_position_and_rotation
                .send(SetPlayerPositionAndRotation {
                    client: entity,
                    position: p.position.into(),
                    yaw: p.yaw,
                    pitch: p.pitch,
                    on_ground: p.on_ground,
                });

            events.1.move_player.send(MovePlayer {
                client: entity,
                old_position: client.position,
                position: p.position.into(),
                old_yaw: client.yaw,
                yaw: p.yaw,
                old_pitch: client.pitch,
                pitch: p.pitch,
                old_on_ground: client.on_ground,
                on_ground: p.on_ground,
            });

            client.position = p.position.into();
            client.yaw = p.yaw;
            client.pitch = p.pitch;
            client.on_ground = p.on_ground;
        }
        C2sPlayPacket::SetPlayerRotation(p) => {
            if client.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.set_player_rotation.send(SetPlayerRotation {
                client: entity,
                yaw: p.yaw,
                pitch: p.pitch,
                on_ground: p.on_ground,
            });

            events.1.move_player.send(MovePlayer {
                client: entity,
                old_position: client.position,
                position: client.position,
                old_yaw: client.yaw,
                yaw: p.yaw,
                old_pitch: client.pitch,
                pitch: p.pitch,
                old_on_ground: client.on_ground,
                on_ground: p.on_ground,
            });

            client.yaw = p.yaw;
            client.pitch = p.pitch;
            client.on_ground = p.on_ground;
        }
        C2sPlayPacket::SetPlayerOnGround(p) => {
            if client.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.set_player_on_ground.send(SetPlayerOnGround {
                client: entity,
                on_ground: p.on_ground,
            });

            events.1.move_player.send(MovePlayer {
                client: entity,
                old_position: client.position,
                position: client.position,
                old_yaw: client.yaw,
                yaw: client.yaw,
                old_pitch: client.pitch,
                pitch: client.pitch,
                old_on_ground: client.on_ground,
                on_ground: p.on_ground,
            });

            client.on_ground = p.on_ground;
        }
        C2sPlayPacket::MoveVehicleC2s(p) => {
            if client.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.move_vehicle.send(MoveVehicle {
                client: entity,
                position: p.position.into(),
                yaw: p.yaw,
                pitch: p.pitch,
            });

            events.1.move_player.send(MovePlayer {
                client: entity,
                old_position: client.position,
                position: p.position.into(),
                old_yaw: client.yaw,
                yaw: p.yaw,
                old_pitch: client.pitch,
                pitch: p.pitch,
                old_on_ground: client.on_ground,
                on_ground: client.on_ground,
            });

            client.position = p.position.into();
            client.yaw = p.yaw;
            client.pitch = p.pitch;
        }
        C2sPlayPacket::PlayerCommand(p) => match p.action_id {
            Action::StartSneaking => events
                .1
                .start_sneaking
                .send(StartSneaking { client: entity }),
            Action::StopSneaking => events.1.stop_sneaking.send(StopSneaking { client: entity }),
            Action::LeaveBed => events.1.leave_bed.send(LeaveBed { client: entity }),
            Action::StartSprinting => events
                .1
                .start_sprinting
                .send(StartSprinting { client: entity }),
            Action::StopSprinting => events
                .1
                .stop_sprinting
                .send(StopSprinting { client: entity }),
            Action::StartJumpWithHorse => events.1.start_jump_with_horse.send(StartJumpWithHorse {
                client: entity,
                jump_boost: p.jump_boost.0 as u8,
            }),
            Action::StopJumpWithHorse => events
                .1
                .stop_jump_with_horse
                .send(StopJumpWithHorse { client: entity }),
            Action::OpenHorseInventory => events
                .2
                .open_horse_inventory
                .send(OpenHorseInventory { client: entity }),
            Action::StartFlyingWithElytra => events
                .2
                .start_flying_with_elytra
                .send(StartFlyingWithElytra { client: entity }),
        },
        C2sPlayPacket::PaddleBoat(p) => {
            events.2.paddle_boat.send(PaddleBoat {
                client: entity,
                left_paddle_turning: p.left_paddle_turning,
                right_paddle_turning: p.right_paddle_turning,
            });
        }
        C2sPlayPacket::PickItem(p) => {
            events.2.pick_item.send(PickItem {
                client: entity,
                slot_to_use: p.slot_to_use.0,
            });
        }
        C2sPlayPacket::PlaceRecipe(p) => {
            events.2.place_recipe.send(PlaceRecipe {
                client: entity,
                window_id: p.window_id,
                recipe: p.recipe.into(),
                make_all: p.make_all,
            });
        }
        C2sPlayPacket::PlayerAbilitiesC2s(p) => match p {
            PlayerAbilitiesC2s::StopFlying => {
                events.2.stop_flying.send(StopFlying { client: entity })
            }
            PlayerAbilitiesC2s::StartFlying => {
                events.2.start_flying.send(StartFlying { client: entity })
            }
        },
        C2sPlayPacket::PlayerAction(p) => {
            if p.sequence.0 != 0 {
                client.block_change_sequence = cmp::max(p.sequence.0, client.block_change_sequence);
            }

            match p.status {
                DiggingStatus::StartedDigging => events.2.start_digging.send(StartDigging {
                    client: entity,
                    position: p.position,
                    face: p.face,
                    sequence: p.sequence.0,
                }),
                DiggingStatus::CancelledDigging => events.2.cancel_digging.send(CancelDigging {
                    client: entity,
                    position: p.position,
                    face: p.face,
                    sequence: p.sequence.0,
                }),
                DiggingStatus::FinishedDigging => events.2.finish_digging.send(FinishDigging {
                    client: entity,
                    position: p.position,
                    face: p.face,
                    sequence: p.sequence.0,
                }),
                DiggingStatus::DropItemStack => events
                    .2
                    .drop_item_stack
                    .send(DropItemStack { client: entity }),
                DiggingStatus::DropItem => events.2.drop_item.send(DropItem { client: entity }),
                DiggingStatus::UpdateHeldItemState => events
                    .2
                    .update_held_item_state
                    .send(UpdateHeldItemState { client: entity }),
                DiggingStatus::SwapItemInHand => events
                    .2
                    .swap_item_in_hand
                    .send(SwapItemInHand { client: entity }),
            }
        }
        C2sPlayPacket::PlayerInput(p) => {
            events.2.player_input.send(PlayerInput {
                client: entity,
                sideways: p.sideways,
                forward: p.forward,
                jump: p.flags.jump(),
                unmount: p.flags.unmount(),
            });
        }
        C2sPlayPacket::PongPlay(p) => {
            events.2.pong.send(Pong {
                client: entity,
                id: p.id,
            });
        }
        C2sPlayPacket::PlayerSession(p) => {
            events.3.player_session.send(PlayerSession {
                client: entity,
                session_id: p.session_id,
                expires_at: p.expires_at,
                public_key_data: p.public_key_data.into(),
                key_signature: p.key_signature.into(),
            });
        }
        C2sPlayPacket::ChangeRecipeBookSettings(p) => {
            events
                .3
                .change_recipe_book_settings
                .send(ChangeRecipeBookSettings {
                    client: entity,
                    book_id: p.book_id,
                    book_open: p.book_open,
                    filter_active: p.filter_active,
                });
        }
        C2sPlayPacket::SetSeenRecipe(p) => {
            events.3.set_seen_recipe.send(SetSeenRecipe {
                client: entity,
                recipe_id: p.recipe_id.into(),
            });
        }
        C2sPlayPacket::RenameItem(p) => {
            events.3.rename_item.send(RenameItem {
                client: entity,
                name: p.item_name.into(),
            });
        }
        C2sPlayPacket::ResourcePackC2s(p) => {
            events
                .3
                .resource_pack_status_change
                .send(ResourcePackStatusChange {
                    client: entity,
                    status: p.into(),
                })
        }
        C2sPlayPacket::SeenAdvancements(p) => match p {
            SeenAdvancements::OpenedTab { tab_id } => {
                events.3.open_advancement_tab.send(OpenAdvancementTab {
                    client: entity,
                    tab_id: tab_id.into(),
                })
            }
            SeenAdvancements::ClosedScreen => events
                .3
                .close_advancement_screen
                .send(CloseAdvancementScreen { client: entity }),
        },
        C2sPlayPacket::SelectTrade(p) => {
            events.3.select_trade.send(SelectTrade {
                client: entity,
                slot: p.selected_slot.0,
            });
        }
        C2sPlayPacket::SetBeaconEffect(p) => {
            events.3.set_beacon_effect.send(SetBeaconEffect {
                client: entity,
                primary_effect: p.primary_effect.map(|i| i.0),
                secondary_effect: p.secondary_effect.map(|i| i.0),
            });
        }
        C2sPlayPacket::SetHeldItemC2s(p) => events.3.set_held_item.send(SetHeldItem {
            client: entity,
            slot: p.slot,
        }),
        C2sPlayPacket::ProgramCommandBlock(p) => {
            events.3.program_command_block.send(ProgramCommandBlock {
                client: entity,
                position: p.position,
                command: p.command.into(),
                mode: p.mode,
                track_output: p.flags.track_output(),
                conditional: p.flags.conditional(),
                automatic: p.flags.automatic(),
            });
        }
        C2sPlayPacket::ProgramCommandBlockMinecart(p) => {
            events
                .3
                .program_command_block_minecart
                .send(ProgramCommandBlockMinecart {
                    client: entity,
                    entity_id: p.entity_id.0,
                    command: p.command.into(),
                    track_output: p.track_output,
                });
        }
        C2sPlayPacket::SetCreativeModeSlot(p) => {
            events.3.set_creative_mode_slot.send(SetCreativeModeSlot {
                client: entity,
                slot: p.slot,
                clicked_item: p.clicked_item,
            });
        }
        C2sPlayPacket::ProgramJigsawBlock(p) => {
            events.4.program_jigsaw_block.send(ProgramJigsawBlock {
                client: entity,
                position: p.position,
                name: p.name.into(),
                target: p.target.into(),
                pool: p.pool.into(),
                final_state: p.final_state.into(),
                joint_type: p.joint_type.into(),
            });
        }
        C2sPlayPacket::ProgramStructureBlock(p) => {
            events
                .4
                .program_structure_block
                .send(ProgramStructureBlock {
                    client: entity,
                    position: p.position,
                    action: p.action,
                    mode: p.mode,
                    name: p.name.into(),
                    offset_xyz: p.offset_xyz,
                    size_xyz: p.size_xyz,
                    mirror: p.mirror,
                    rotation: p.rotation,
                    metadata: p.metadata.into(),
                    integrity: p.integrity,
                    seed: p.seed.0,
                    flags: p.flags,
                })
        }
        C2sPlayPacket::UpdateSign(p) => {
            events.4.update_sign.send(UpdateSign {
                client: entity,
                position: p.position,
                lines: p.lines.map(Into::into),
            });
        }
        C2sPlayPacket::SwingArm(p) => {
            events.4.swing_arm.send(SwingArm {
                client: entity,
                hand: p.hand,
            });
        }
        C2sPlayPacket::TeleportToEntity(p) => {
            events.4.teleport_to_entity.send(TeleportToEntity {
                client: entity,
                target: p.target,
            });
        }
        C2sPlayPacket::UseItemOn(p) => {
            if p.sequence.0 != 0 {
                client.block_change_sequence = cmp::max(p.sequence.0, client.block_change_sequence);
            }

            events.4.use_item_on_block.send(UseItemOnBlock {
                client: entity,
                hand: p.hand,
                position: p.position,
                face: p.face,
                cursor_pos: p.cursor_pos.into(),
                head_inside_block: false,
                sequence: 0,
            })
        }
        C2sPlayPacket::UseItem(p) => {
            if p.sequence.0 != 0 {
                client.block_change_sequence = cmp::max(p.sequence.0, client.block_change_sequence);
            }

            events.4.use_item.send(UseItem {
                client: entity,
                hand: p.hand,
                sequence: p.sequence.0,
            });
        }
    }

    Ok(true)
}

/// The default event handler system which handles client events in a
/// reasonable default way.
///
/// For instance, movement events are handled by changing the entity's
/// position/rotation to match the received movement, crouching makes the
/// entity crouch, etc.
///
/// This system's primary purpose is to reduce boilerplate code in the
/// examples, but it can be used as a quick way to get started in your own
/// code. The precise behavior of this system is left unspecified and
/// is subject to change.
///
/// This system must be scheduled to run in the
/// [`EventLoop`](crate::server::EventLoop) stage. Otherwise, it may not
/// function correctly.
#[allow(clippy::too_many_arguments)]
pub fn default_event_handler(
    mut clients: Query<(&mut Client, Option<&mut McEntity>)>,
    mut update_settings: EventReader<UpdateSettings>,
    mut move_player: EventReader<MovePlayer>,
    mut start_sneaking: EventReader<StartSneaking>,
    mut stop_sneaking: EventReader<StopSneaking>,
    mut start_sprinting: EventReader<StartSprinting>,
    mut stop_sprinting: EventReader<StopSprinting>,
    mut swing_arm: EventReader<SwingArm>,
) {
    for UpdateSettings {
        client,
        view_distance,
        displayed_skin_parts,
        main_hand,
        ..
    } in update_settings.iter()
    {
        let Ok((mut client, entity)) = clients.get_mut(*client) else {
            continue
        };

        client.set_view_distance(*view_distance);

        let player = client.player_mut();

        player.set_cape(displayed_skin_parts.cape());
        player.set_jacket(displayed_skin_parts.jacket());
        player.set_left_sleeve(displayed_skin_parts.left_sleeve());
        player.set_right_sleeve(displayed_skin_parts.right_sleeve());
        player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
        player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
        player.set_hat(displayed_skin_parts.hat());
        player.set_main_arm(*main_hand as u8);

        if let Some(mut entity) = entity {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_cape(displayed_skin_parts.cape());
                player.set_jacket(displayed_skin_parts.jacket());
                player.set_left_sleeve(displayed_skin_parts.left_sleeve());
                player.set_right_sleeve(displayed_skin_parts.right_sleeve());
                player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
                player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
                player.set_hat(displayed_skin_parts.hat());
                player.set_main_arm(*main_hand as u8);
            }
        }
    }

    for MovePlayer {
        client,
        position,
        yaw,
        pitch,
        on_ground,
        ..
    } in move_player.iter()
    {
        let Ok((_, Some(mut entity))) = clients.get_mut(*client) else {
            continue
        };

        entity.set_position(*position);
        entity.set_yaw(*yaw);
        entity.set_head_yaw(*yaw);
        entity.set_pitch(*pitch);
        entity.set_on_ground(*on_ground);
    }

    for StartSneaking { client } in start_sneaking.iter() {
        let Ok((_, Some(mut entity))) = clients.get_mut(*client) else {
            continue
        };

        if let TrackedData::Player(player) = entity.data_mut() {
            player.set_pose(Pose::Sneaking);
        }
    }

    for StopSneaking { client } in stop_sneaking.iter() {
        let Ok((_, Some(mut entity))) = clients.get_mut(*client) else {
            continue
        };

        if let TrackedData::Player(player) = entity.data_mut() {
            player.set_pose(Pose::Standing);
        }
    }

    for StartSprinting { client } in start_sprinting.iter() {
        let Ok((_, Some(mut entity))) = clients.get_mut(*client) else {
            continue
        };

        if let TrackedData::Player(player) = entity.data_mut() {
            player.set_sprinting(true);
        }
    }

    for StopSprinting { client } in stop_sprinting.iter() {
        let Ok((_, Some(mut entity))) = clients.get_mut(*client) else {
            continue
        };

        if let TrackedData::Player(player) = entity.data_mut() {
            player.set_sprinting(false);
        }
    }

    for SwingArm { client, hand } in swing_arm.iter() {
        let Ok((_, Some(mut entity))) = clients.get_mut(*client) else {
            continue
        };

        if entity.kind() == EntityKind::Player {
            entity.trigger_animation(match hand {
                Hand::Main => EntityAnimation::SwingMainHand,
                Hand::Off => EntityAnimation::SwingOffHand,
            });
        }
    }
}
