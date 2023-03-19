use std::cmp;

use anyhow::bail;
use bevy_app::{CoreSet, Plugin};
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::{SystemParam, SystemState};
use glam::{DVec3, Vec3};
use paste::paste;
use tracing::warn;
use uuid::Uuid;
use valence_protocol::block_pos::BlockPos;
use valence_protocol::ident::Ident;
use valence_protocol::item::ItemStack;
use valence_protocol::packet::c2s::play::click_slot::{ClickMode, Slot};
use valence_protocol::packet::c2s::play::client_command::Action as ClientCommandAction;
use valence_protocol::packet::c2s::play::client_settings::{
    ChatMode, DisplayedSkinParts, MainHand,
};
use valence_protocol::packet::c2s::play::player_action::Action as PlayerAction;
use valence_protocol::packet::c2s::play::player_interact::Interaction;
use valence_protocol::packet::c2s::play::recipe_category_options::RecipeBookId;
use valence_protocol::packet::c2s::play::update_command_block::Mode as CommandBlockMode;
use valence_protocol::packet::c2s::play::update_structure_block::{
    Action as StructureBlockAction, Flags as StructureBlockFlags, Mirror as StructureBlockMirror,
    Mode as StructureBlockMode, Rotation as StructureBlockRotation,
};
use valence_protocol::packet::c2s::play::{
    AdvancementTabC2s, ClientStatusC2s, ResourcePackStatusC2s, UpdatePlayerAbilitiesC2s,
};
use valence_protocol::packet::C2sPlayPacket;
use valence_protocol::types::{Difficulty, Direction, Hand};

use super::{
    CursorItem, KeepaliveState, PlayerActionSequence, PlayerInventoryState, TeleportState,
    ViewDistance,
};
use crate::client::Client;
use crate::component::{Look, OnGround, Ping, Position};
use crate::entity::{EntityAnimation, EntityKind};
use crate::inventory::Inventory;

#[derive(Clone, Debug)]
pub struct QueryBlockNbt {
    pub client: Entity,
    pub position: BlockPos,
    pub transaction_id: i32,
}

#[derive(Clone, Debug)]
pub struct UpdateDifficulty {
    pub client: Entity,
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug)]
pub struct MessageAcknowledgment {
    pub client: Entity,
    pub message_count: i32,
}

#[derive(Clone, Debug)]
pub struct CommandExecution {
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
pub struct PerformRespawn {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct RequestStats {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct ClientSettings {
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
pub struct RequestCommandCompletions {
    pub client: Entity,
    pub transaction_id: i32,
    pub text: Box<str>,
}

#[derive(Clone, Debug)]
pub struct ButtonClick {
    pub client: Entity,
    pub window_id: i8,
    pub button_id: i8,
}

#[derive(Clone, Debug)]
pub struct ClickSlot {
    pub client: Entity,
    pub window_id: u8,
    pub state_id: i32,
    pub slot_id: i16,
    pub button: i8,
    pub mode: ClickMode,
    pub slot_changes: Vec<Slot>,
    pub carried_item: Option<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct CloseHandledScreen {
    pub client: Entity,
    pub window_id: i8,
}

#[derive(Clone, Debug)]
pub struct CustomPayload {
    pub client: Entity,
    pub channel: Ident<Box<str>>,
    pub data: Box<[u8]>,
}

#[derive(Clone, Debug)]
pub struct BookUpdate {
    pub slot: i32,
    pub entries: Vec<Box<str>>,
    pub title: Option<Box<str>>,
}

#[derive(Clone, Debug)]
pub struct QueryEntityNbt {
    pub client: Entity,
    pub transaction_id: i32,
    pub entity_id: i32,
}

/// Left or right click interaction with an entity's hitbox.
#[derive(Clone, Debug)]
pub struct PlayerInteract {
    pub client: Entity,
    /// The raw ID of the entity being interacted with.
    pub entity_id: i32,
    /// If the client was sneaking during the interaction.
    pub sneaking: bool,
    /// The kind of interaction that occurred.
    pub interact: Interaction,
}

#[derive(Clone, Debug)]
pub struct JigsawGenerating {
    pub client: Entity,
    pub position: BlockPos,
    pub levels: i32,
    pub keep_jigsaws: bool,
}

#[derive(Clone, Debug)]
pub struct UpdateDifficultyLock {
    pub client: Entity,
    pub locked: bool,
}

#[derive(Clone, Debug)]
pub struct PlayerMove {
    pub client: Entity,
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct VehicleMove {
    pub client: Entity,
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
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
pub struct BoatPaddleState {
    pub client: Entity,
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}

#[derive(Clone, Debug)]
pub struct PickFromInventory {
    pub client: Entity,
    pub slot_to_use: i32,
}

#[derive(Clone, Debug)]
pub struct CraftRequest {
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
    pub direction: Direction,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct AbortDestroyBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub direction: Direction,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct StopDestroyBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub direction: Direction,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct DropItemStack {
    pub client: Entity,
    pub from_slot: Option<u16>,
    pub stack: ItemStack,
}

/// Eating food, pulling back bows, using buckets, etc.
#[derive(Clone, Debug)]
pub struct ReleaseUseItem {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct SwapItemWithOffhand {
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
pub struct PlayPong {
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
pub struct RecipeCategoryOptions {
    pub client: Entity,
    pub book_id: RecipeBookId,
    pub book_open: bool,
    pub filter_active: bool,
}

#[derive(Clone, Debug)]
pub struct RecipeBookData {
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

impl From<ResourcePackStatusC2s> for ResourcePackStatus {
    fn from(packet: ResourcePackStatusC2s) -> Self {
        match packet {
            ResourcePackStatusC2s::Accepted => Self::Accepted,
            ResourcePackStatusC2s::Declined => Self::Declined,
            ResourcePackStatusC2s::SuccessfullyLoaded => Self::Loaded,
            ResourcePackStatusC2s::FailedDownload => Self::FailedDownload,
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
pub struct SelectMerchantTrade {
    pub client: Entity,
    pub slot: i32,
}

#[derive(Clone, Debug)]
pub struct UpdateBeacon {
    pub client: Entity,
    pub primary_effect: Option<i32>,
    pub secondary_effect: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct UpdateSelectedSlot {
    pub client: Entity,
    pub slot: i16,
}

#[derive(Clone, Debug)]
pub struct UpdateCommandBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub command: Box<str>,
    pub mode: CommandBlockMode,
    pub track_output: bool,
    pub conditional: bool,
    pub automatic: bool,
}

#[derive(Clone, Debug)]
pub struct UpdateCommandBlockMinecart {
    pub client: Entity,
    pub entity_id: i32,
    pub command: Box<str>,
    pub track_output: bool,
}

#[derive(Clone, Debug)]
pub struct CreativeInventoryAction {
    pub client: Entity,
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct UpdateJigsaw {
    pub client: Entity,
    pub position: BlockPos,
    pub name: Ident<Box<str>>,
    pub target: Ident<Box<str>>,
    pub pool: Ident<Box<str>>,
    pub final_state: Box<str>,
    pub joint_type: Box<str>,
}

#[derive(Clone, Debug)]
pub struct UpdateStructureBlock {
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
pub struct HandSwing {
    pub client: Entity,
    pub hand: Hand,
}

#[derive(Clone, Debug)]
pub struct SpectatorTeleport {
    pub client: Entity,
    pub target: Uuid,
}

#[derive(Clone, Debug)]
pub struct PlayerInteractBlock {
    pub client: Entity,
    /// The hand that was used
    pub hand: Hand,
    /// The location of the block that was interacted with
    pub position: BlockPos,
    /// The face of the block that was clicked
    pub direction: Direction,
    /// The position inside of the block that was clicked on
    pub cursor_pos: Vec3,
    /// Whether or not the player's head is inside a block
    pub head_inside_block: bool,
    /// Sequence number for synchronization
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct PlayerInteractItem {
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
        fn register_client_events(world: &mut World) {
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
        QueryBlockNbt
        UpdateDifficulty
        MessageAcknowledgment
        CommandExecution
        ChatMessage
        PerformRespawn
        RequestStats
        ClientSettings
        RequestCommandCompletions
        ButtonClick
        ClickSlot
        CloseHandledScreen
        CustomPayload
        BookUpdate
        QueryEntityNbt
    }
    1 {
        PlayerInteract
        JigsawGenerating
        UpdateDifficultyLock
        PlayerMove
        VehicleMove
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
        BoatPaddleState
        PickFromInventory
        CraftRequest
        StopFlying
        StartFlying
        StartDigging
        AbortDestroyBlock
        StopDestroyBlock
        DropItemStack
        ReleaseUseItem
        SwapItemWithOffhand
        PlayerInput
        PlayPong
    }
    3 {
        PlayerSession
        RecipeCategoryOptions
        RecipeBookData
        RenameItem
        ResourcePackStatusChange
        OpenAdvancementTab
        CloseAdvancementScreen
        SelectMerchantTrade
        UpdateBeacon
        UpdateSelectedSlot
        UpdateCommandBlock
        UpdateCommandBlockMinecart
        CreativeInventoryAction
    }
    4 {
        UpdateJigsaw
        UpdateStructureBlock
        UpdateSign
        HandSwing
        SpectatorTeleport
        PlayerInteractBlock
        PlayerInteractItem
    }
}

pub(crate) struct ClientEventPlugin;

/// The [`ScheduleLabel`] for the event loop [`Schedule`].
#[derive(ScheduleLabel, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EventLoopSchedule;

/// The default base set for [`EventLoopSchedule`].
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EventLoopSet;

impl Plugin for ClientEventPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        register_client_events(&mut app.world);

        app.configure_set(EventLoopSet.in_base_set(CoreSet::PreUpdate))
            .add_system(run_event_loop.in_set(EventLoopSet));

        // Add the event loop schedule.
        let mut event_loop = Schedule::new();
        event_loop.set_default_base_set(EventLoopSet);

        app.add_schedule(EventLoopSchedule, event_loop);
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub(crate) struct EventLoopQuery {
    entity: Entity,
    client: &'static mut Client,
    teleport_state: &'static mut TeleportState,
    keepalive_state: &'static mut KeepaliveState,
    cursor_item: &'static mut CursorItem,
    inventory: &'static mut Inventory,
    position: &'static mut Position,
    look: &'static mut Look,
    on_ground: &'static mut OnGround,
    ping: &'static mut Ping,
    player_action_sequence: &'static mut PlayerActionSequence,
    player_inventory_state: &'static mut PlayerInventoryState,
}

/// An exclusive system for running the event loop schedule.
fn run_event_loop(
    world: &mut World,
    state: &mut SystemState<(Query<EventLoopQuery>, ClientEvents, Commands)>,
    mut clients_to_check: Local<Vec<Entity>>,
) {
    let (mut clients, mut events, mut commands) = state.get_mut(world);

    update_all_event_buffers(&mut events);

    for mut q in &mut clients {
        let Ok(bytes) = q.client.conn.try_recv() else {
            // Client is disconnected.
            commands.entity(q.entity).remove::<Client>();
            continue;
        };

        if bytes.is_empty() {
            // No data was received.
            continue;
        }

        q.client.dec.queue_bytes(bytes);

        match handle_one_packet(&mut q, &mut events) {
            Ok(had_packet) => {
                if had_packet {
                    // We decoded one packet, but there might be more.
                    clients_to_check.push(q.entity);
                }
            }
            Err(e) => {
                warn!("failed to dispatch events for client {:?}: {e:?}", q.entity);
                commands.entity(q.entity).remove::<Client>();
            }
        }
    }

    state.apply(world);

    // Keep looping until all serverbound packets are decoded.
    while !clients_to_check.is_empty() {
        world.run_schedule(EventLoopSchedule);

        let (mut clients, mut events, mut commands) = state.get_mut(world);

        clients_to_check.retain(|&entity| {
            let Ok(mut q) = clients.get_mut(entity) else {
                // Client must have been deleted during the last run of the schedule.
                return false;
            };

            match handle_one_packet(&mut q, &mut events) {
                Ok(had_packet) => had_packet,
                Err(e) => {
                    warn!("failed to dispatch events for client {:?}: {e:?}", q.entity);
                    commands.entity(entity).remove::<Client>();
                    false
                }
            }
        });

        state.apply(world);
    }
}

fn handle_one_packet(
    q: &mut EventLoopQueryItem,
    events: &mut ClientEvents,
) -> anyhow::Result<bool> {
    let Some(pkt) = q.client.dec.try_next_packet::<C2sPlayPacket>()? else {
        // No packets to decode.
        return Ok(false);
    };

    let entity = q.entity;

    match pkt {
        C2sPlayPacket::TeleportConfirmC2s(p) => {
            if q.teleport_state.pending_teleports == 0 {
                bail!("unexpected teleport confirmation");
            }

            let got = p.teleport_id.0 as u32;
            let expected = q
                .teleport_state
                .teleport_id_counter
                .wrapping_sub(q.teleport_state.pending_teleports);

            if got == expected {
                q.teleport_state.pending_teleports -= 1;
            } else {
                bail!("unexpected teleport ID (expected {expected}, got {got}");
            }
        }
        C2sPlayPacket::QueryBlockNbtC2s(p) => {
            events.0.query_block_nbt.send(QueryBlockNbt {
                client: entity,
                position: p.position,
                transaction_id: p.transaction_id.0,
            });
        }
        C2sPlayPacket::UpdateDifficultyC2s(p) => {
            events.0.update_difficulty.send(UpdateDifficulty {
                client: entity,
                difficulty: p.difficulty,
            });
        }
        C2sPlayPacket::MessageAcknowledgmentC2s(p) => {
            events.0.message_acknowledgment.send(MessageAcknowledgment {
                client: entity,
                message_count: p.message_count.0,
            });
        }
        C2sPlayPacket::CommandExecutionC2s(p) => {
            events.0.command_execution.send(CommandExecution {
                client: entity,
                command: p.command.into(),
                timestamp: p.timestamp,
            });
        }
        C2sPlayPacket::ChatMessageC2s(p) => {
            events.0.chat_message.send(ChatMessage {
                client: entity,
                message: p.message.into(),
                timestamp: p.timestamp,
            });
        }
        C2sPlayPacket::ClientStatusC2s(p) => match p {
            ClientStatusC2s::PerformRespawn => events
                .0
                .perform_respawn
                .send(PerformRespawn { client: entity }),
            ClientStatusC2s::RequestStats => {
                events.0.request_stats.send(RequestStats { client: entity })
            }
        },
        C2sPlayPacket::ClientSettingsC2s(p) => {
            events.0.client_settings.send(ClientSettings {
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
        C2sPlayPacket::RequestCommandCompletionsC2s(p) => {
            events
                .0
                .request_command_completions
                .send(RequestCommandCompletions {
                    client: entity,
                    transaction_id: p.transaction_id.0,
                    text: p.text.into(),
                });
        }
        C2sPlayPacket::ButtonClickC2s(p) => {
            events.0.button_click.send(ButtonClick {
                client: entity,
                window_id: p.window_id,
                button_id: p.button_id,
            });
        }
        C2sPlayPacket::ClickSlotC2s(p) => {
            if p.slot_idx < 0 {
                if let Some(stack) = q.cursor_item.0.take() {
                    events.2.drop_item_stack.send(DropItemStack {
                        client: entity,
                        from_slot: None,
                        stack,
                    });
                }
            } else if p.mode == ClickMode::DropKey {
                let entire_stack = p.button == 1;
                if let Some(stack) = q.inventory.slot(p.slot_idx as u16) {
                    let dropped = if entire_stack || stack.count() == 1 {
                        q.inventory.replace_slot(p.slot_idx as u16, None)
                    } else {
                        let mut stack = stack.clone();
                        stack.set_count(stack.count() - 1);
                        let mut old_slot = q.inventory.replace_slot(p.slot_idx as u16, Some(stack));
                        // we already checked that the slot was not empty and that the
                        // stack count is > 1
                        old_slot.as_mut().unwrap().set_count(1);
                        old_slot
                    }
                    .expect("dropped item should exist"); // we already checked that the slot was not empty
                    events.2.drop_item_stack.send(DropItemStack {
                        client: entity,
                        from_slot: Some(p.slot_idx as u16),
                        stack: dropped,
                    });
                }
            } else {
                events.0.click_slot.send(ClickSlot {
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
        }
        C2sPlayPacket::CloseHandledScreenC2s(p) => {
            events.0.close_handled_screen.send(CloseHandledScreen {
                client: entity,
                window_id: p.window_id,
            });
        }
        C2sPlayPacket::CustomPayloadC2s(p) => {
            events.0.custom_payload.send(CustomPayload {
                client: entity,
                channel: p.channel.into(),
                data: p.data.0.into(),
            });
        }
        C2sPlayPacket::BookUpdateC2s(p) => {
            events.0.book_update.send(BookUpdate {
                slot: p.slot.0,
                entries: p.entries.into_iter().map(Into::into).collect(),
                title: p.title.map(Box::from),
            });
        }
        C2sPlayPacket::QueryEntityNbtC2s(p) => {
            events.0.query_entity_nbt.send(QueryEntityNbt {
                client: entity,
                transaction_id: p.transaction_id.0,
                entity_id: p.entity_id.0,
            });
        }
        C2sPlayPacket::PlayerInteractC2s(p) => {
            events.1.player_interact.send(PlayerInteract {
                client: entity,
                entity_id: p.entity_id.0,
                sneaking: p.sneaking,
                interact: p.interact,
            });
        }
        C2sPlayPacket::JigsawGeneratingC2s(p) => {
            events.1.jigsaw_generating.send(JigsawGenerating {
                client: entity,
                position: p.position,
                levels: p.levels.0,
                keep_jigsaws: p.keep_jigsaws,
            });
        }
        C2sPlayPacket::KeepAliveC2s(p) => {
            if q.keepalive_state.got_keepalive {
                bail!("unexpected keepalive");
            } else if p.id != q.keepalive_state.last_keepalive_id {
                bail!(
                    "keepalive IDs don't match (expected {}, got {})",
                    q.keepalive_state.last_keepalive_id,
                    p.id
                );
            } else {
                q.keepalive_state.got_keepalive = true;
                q.ping.0 = q.keepalive_state.keepalive_sent_time.elapsed().as_millis() as i32;
            }
        }
        C2sPlayPacket::UpdateDifficultyLockC2s(p) => {
            events.1.update_difficulty_lock.send(UpdateDifficultyLock {
                client: entity,
                locked: p.locked,
            });
        }
        C2sPlayPacket::PositionAndOnGroundC2s(p) => {
            if q.teleport_state.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.player_move.send(PlayerMove {
                client: entity,
                position: p.position.into(),
                yaw: q.look.yaw,
                pitch: q.look.pitch,
                on_ground: q.on_ground.0,
            });

            q.position.0 = p.position.into();
            q.teleport_state.synced_pos = p.position.into();
            q.on_ground.0 = p.on_ground;
        }
        C2sPlayPacket::FullC2s(p) => {
            if q.teleport_state.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.player_move.send(PlayerMove {
                client: entity,
                position: p.position.into(),
                yaw: p.yaw,
                pitch: p.pitch,
                on_ground: p.on_ground,
            });

            q.position.0 = p.position.into();
            q.teleport_state.synced_pos = p.position.into();
            q.look.yaw = p.yaw;
            q.teleport_state.synced_look.yaw = p.yaw;
            q.look.pitch = p.pitch;
            q.teleport_state.synced_look.pitch = p.pitch;
            q.on_ground.0 = p.on_ground;
        }
        C2sPlayPacket::LookAndOnGroundC2s(p) => {
            if q.teleport_state.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.player_move.send(PlayerMove {
                client: entity,
                position: q.position.0,
                yaw: p.yaw,
                pitch: p.pitch,
                on_ground: p.on_ground,
            });

            q.look.yaw = p.yaw;
            q.teleport_state.synced_look.yaw = p.yaw;
            q.look.pitch = p.pitch;
            q.teleport_state.synced_look.pitch = p.pitch;
            q.on_ground.0 = p.on_ground;
        }
        C2sPlayPacket::OnGroundOnlyC2s(p) => {
            if q.teleport_state.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.player_move.send(PlayerMove {
                client: entity,
                position: q.position.0,
                yaw: q.look.yaw,
                pitch: q.look.pitch,
                on_ground: p.on_ground,
            });

            q.on_ground.0 = p.on_ground;
        }
        C2sPlayPacket::VehicleMoveC2s(p) => {
            if q.teleport_state.pending_teleports != 0 {
                return Ok(false);
            }

            events.1.vehicle_move.send(VehicleMove {
                client: entity,
                position: p.position.into(),
                yaw: p.yaw,
                pitch: p.pitch,
            });

            q.position.0 = p.position.into();
            q.teleport_state.synced_pos = p.position.into();
            q.look.yaw = p.yaw;
            q.teleport_state.synced_look.yaw = p.yaw;
            q.look.pitch = p.pitch;
            q.teleport_state.synced_look.pitch = p.pitch;
        }
        C2sPlayPacket::BoatPaddleStateC2s(p) => {
            events.2.boat_paddle_state.send(BoatPaddleState {
                client: entity,
                left_paddle_turning: p.left_paddle_turning,
                right_paddle_turning: p.right_paddle_turning,
            });
        }
        C2sPlayPacket::PickFromInventoryC2s(p) => {
            events.2.pick_from_inventory.send(PickFromInventory {
                client: entity,
                slot_to_use: p.slot_to_use.0,
            });
        }
        C2sPlayPacket::CraftRequestC2s(p) => {
            events.2.craft_request.send(CraftRequest {
                client: entity,
                window_id: p.window_id,
                recipe: p.recipe.into(),
                make_all: p.make_all,
            });
        }
        C2sPlayPacket::UpdatePlayerAbilitiesC2s(p) => match p {
            UpdatePlayerAbilitiesC2s::StopFlying => {
                events.2.stop_flying.send(StopFlying { client: entity })
            }
            UpdatePlayerAbilitiesC2s::StartFlying => {
                events.2.start_flying.send(StartFlying { client: entity })
            }
        },
        C2sPlayPacket::PlayerActionC2s(p) => {
            if p.sequence.0 != 0 {
                q.player_action_sequence.0 = cmp::max(p.sequence.0, q.player_action_sequence.0);
            }

            match p.action {
                PlayerAction::StartDestroyBlock => events.2.start_digging.send(StartDigging {
                    client: entity,
                    position: p.position,
                    direction: p.direction,
                    sequence: p.sequence.0,
                }),
                PlayerAction::AbortDestroyBlock => {
                    events.2.abort_destroy_block.send(AbortDestroyBlock {
                        client: entity,
                        position: p.position,
                        direction: p.direction,
                        sequence: p.sequence.0,
                    })
                }
                PlayerAction::StopDestroyBlock => {
                    events.2.stop_destroy_block.send(StopDestroyBlock {
                        client: entity,
                        position: p.position,
                        direction: p.direction,
                        sequence: p.sequence.0,
                    })
                }
                PlayerAction::DropAllItems => {
                    if let Some(stack) = q
                        .inventory
                        .replace_slot(q.player_inventory_state.held_item_slot(), None)
                    {
                        q.player_inventory_state.slots_changed |=
                            1 << q.player_inventory_state.held_item_slot();
                        events.2.drop_item_stack.send(DropItemStack {
                            client: entity,
                            from_slot: Some(q.player_inventory_state.held_item_slot()),
                            stack,
                        });
                    }
                }
                PlayerAction::DropItem => {
                    if let Some(stack) = q.inventory.slot(q.player_inventory_state.held_item_slot())
                    {
                        let mut old_slot = if stack.count() == 1 {
                            q.inventory
                                .replace_slot(q.player_inventory_state.held_item_slot(), None)
                        } else {
                            let mut stack = stack.clone();
                            stack.set_count(stack.count() - 1);
                            q.inventory.replace_slot(
                                q.player_inventory_state.held_item_slot(),
                                Some(stack.clone()),
                            )
                        }
                        .expect("old slot should exist"); // we already checked that the slot was not empty
                        q.player_inventory_state.slots_changed |=
                            1 << q.player_inventory_state.held_item_slot();
                        old_slot.set_count(1);

                        events.2.drop_item_stack.send(DropItemStack {
                            client: entity,
                            from_slot: Some(q.player_inventory_state.held_item_slot()),
                            stack: old_slot,
                        });
                    }
                }
                PlayerAction::ReleaseUseItem => events
                    .2
                    .release_use_item
                    .send(ReleaseUseItem { client: entity }),
                PlayerAction::SwapItemWithOffhand => events
                    .2
                    .swap_item_with_offhand
                    .send(SwapItemWithOffhand { client: entity }),
            }
        }
        C2sPlayPacket::ClientCommandC2s(p) => match p.action {
            ClientCommandAction::StartSneaking => events
                .1
                .start_sneaking
                .send(StartSneaking { client: entity }),
            ClientCommandAction::StopSneaking => {
                events.1.stop_sneaking.send(StopSneaking { client: entity })
            }
            ClientCommandAction::LeaveBed => events.1.leave_bed.send(LeaveBed { client: entity }),
            ClientCommandAction::StartSprinting => events
                .1
                .start_sprinting
                .send(StartSprinting { client: entity }),
            ClientCommandAction::StopSprinting => events
                .1
                .stop_sprinting
                .send(StopSprinting { client: entity }),
            ClientCommandAction::StartJumpWithHorse => {
                events.1.start_jump_with_horse.send(StartJumpWithHorse {
                    client: entity,
                    jump_boost: p.jump_boost.0 as u8,
                })
            }
            ClientCommandAction::StopJumpWithHorse => events
                .1
                .stop_jump_with_horse
                .send(StopJumpWithHorse { client: entity }),
            ClientCommandAction::OpenHorseInventory => events
                .2
                .open_horse_inventory
                .send(OpenHorseInventory { client: entity }),
            ClientCommandAction::StartFlyingWithElytra => events
                .2
                .start_flying_with_elytra
                .send(StartFlyingWithElytra { client: entity }),
        },
        C2sPlayPacket::PlayerInputC2s(p) => {
            events.2.player_input.send(PlayerInput {
                client: entity,
                sideways: p.sideways,
                forward: p.forward,
                jump: p.flags.jump(),
                unmount: p.flags.unmount(),
            });
        }
        C2sPlayPacket::PlayPongC2s(p) => {
            events.2.play_pong.send(PlayPong {
                client: entity,
                id: p.id,
            });
        }
        C2sPlayPacket::PlayerSessionC2s(p) => {
            events.3.player_session.send(PlayerSession {
                client: entity,
                session_id: p.session_id,
                expires_at: p.expires_at,
                public_key_data: p.public_key_data.into(),
                key_signature: p.key_signature.into(),
            });
        }
        C2sPlayPacket::RecipeCategoryOptionsC2s(p) => {
            events
                .3
                .recipe_category_options
                .send(RecipeCategoryOptions {
                    client: entity,
                    book_id: p.book_id,
                    book_open: p.book_open,
                    filter_active: p.filter_active,
                });
        }
        C2sPlayPacket::RecipeBookDataC2s(p) => {
            events.3.recipe_book_data.send(RecipeBookData {
                client: entity,
                recipe_id: p.recipe_id.into(),
            });
        }
        C2sPlayPacket::RenameItemC2s(p) => {
            events.3.rename_item.send(RenameItem {
                client: entity,
                name: p.item_name.into(),
            });
        }
        C2sPlayPacket::ResourcePackStatusC2s(p) => {
            events
                .3
                .resource_pack_status_change
                .send(ResourcePackStatusChange {
                    client: entity,
                    status: p.into(),
                })
        }
        C2sPlayPacket::AdvancementTabC2s(p) => match p {
            AdvancementTabC2s::OpenedTab { tab_id } => {
                events.3.open_advancement_tab.send(OpenAdvancementTab {
                    client: entity,
                    tab_id: tab_id.into(),
                })
            }
            AdvancementTabC2s::ClosedScreen => events
                .3
                .close_advancement_screen
                .send(CloseAdvancementScreen { client: entity }),
        },
        C2sPlayPacket::SelectMerchantTradeC2s(p) => {
            events.3.select_merchant_trade.send(SelectMerchantTrade {
                client: entity,
                slot: p.selected_slot.0,
            });
        }
        C2sPlayPacket::UpdateBeaconC2s(p) => {
            events.3.update_beacon.send(UpdateBeacon {
                client: entity,
                primary_effect: p.primary_effect.map(|i| i.0),
                secondary_effect: p.secondary_effect.map(|i| i.0),
            });
        }
        C2sPlayPacket::UpdateSelectedSlotC2s(p) => {
            events.3.update_selected_slot.send(UpdateSelectedSlot {
                client: entity,
                slot: p.slot,
            })
        }
        C2sPlayPacket::UpdateCommandBlockC2s(p) => {
            events.3.update_command_block.send(UpdateCommandBlock {
                client: entity,
                position: p.position,
                command: p.command.into(),
                mode: p.mode,
                track_output: p.flags.track_output(),
                conditional: p.flags.conditional(),
                automatic: p.flags.automatic(),
            });
        }
        C2sPlayPacket::UpdateCommandBlockMinecartC2s(p) => {
            events
                .3
                .update_command_block_minecart
                .send(UpdateCommandBlockMinecart {
                    client: entity,
                    entity_id: p.entity_id.0,
                    command: p.command.into(),
                    track_output: p.track_output,
                });
        }
        C2sPlayPacket::CreativeInventoryActionC2s(p) => {
            if p.slot == -1 {
                if let Some(stack) = p.clicked_item.as_ref() {
                    events.2.drop_item_stack.send(DropItemStack {
                        client: entity,
                        from_slot: None,
                        stack: stack.clone(),
                    });
                }
            }
            events
                .3
                .creative_inventory_action
                .send(CreativeInventoryAction {
                    client: entity,
                    slot: p.slot,
                    clicked_item: p.clicked_item,
                });
        }
        C2sPlayPacket::UpdateJigsawC2s(p) => {
            events.4.update_jigsaw.send(UpdateJigsaw {
                client: entity,
                position: p.position,
                name: p.name.into(),
                target: p.target.into(),
                pool: p.pool.into(),
                final_state: p.final_state.into(),
                joint_type: p.joint_type.into(),
            });
        }
        C2sPlayPacket::UpdateStructureBlockC2s(p) => {
            events.4.update_structure_block.send(UpdateStructureBlock {
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
        C2sPlayPacket::UpdateSignC2s(p) => {
            events.4.update_sign.send(UpdateSign {
                client: entity,
                position: p.position,
                lines: p.lines.map(Into::into),
            });
        }
        C2sPlayPacket::HandSwingC2s(p) => {
            events.4.hand_swing.send(HandSwing {
                client: entity,
                hand: p.hand,
            });
        }
        C2sPlayPacket::SpectatorTeleportC2s(p) => {
            events.4.spectator_teleport.send(SpectatorTeleport {
                client: entity,
                target: p.target,
            });
        }
        C2sPlayPacket::PlayerInteractBlockC2s(p) => {
            if p.sequence.0 != 0 {
                q.player_action_sequence.0 = cmp::max(p.sequence.0, q.player_action_sequence.0);
            }

            events.4.player_interact_block.send(PlayerInteractBlock {
                client: entity,
                hand: p.hand,
                position: p.position,
                direction: p.face,
                cursor_pos: p.cursor_pos.into(),
                head_inside_block: false,
                sequence: 0,
            })
        }
        C2sPlayPacket::PlayerInteractItemC2s(p) => {
            if p.sequence.0 != 0 {
                q.player_action_sequence.0 = cmp::max(p.sequence.0, q.player_action_sequence.0);
            }

            events.4.player_interact_item.send(PlayerInteractItem {
                client: entity,
                hand: p.hand,
                sequence: p.sequence.0,
            });
        }
    }

    Ok(true)
}

// TODO: fix this up.

/*
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
/// [`EventLoopSchedule`]. Otherwise, it may
/// not function correctly.
#[allow(clippy::too_many_arguments)]
pub fn default_event_handler(
    mut clients: Query<(&mut Client, Option<&mut McEntity>, &mut ViewDistance)>,
    mut update_settings: EventReader<ClientSettings>,
    mut player_move: EventReader<PlayerMove>,
    mut start_sneaking: EventReader<StartSneaking>,
    mut stop_sneaking: EventReader<StopSneaking>,
    mut start_sprinting: EventReader<StartSprinting>,
    mut stop_sprinting: EventReader<StopSprinting>,
    mut swing_arm: EventReader<HandSwing>,
) {
    for ClientSettings {
        client,
        view_distance,
        displayed_skin_parts,
        main_hand,
        ..
    } in update_settings.iter()
    {
        if let Ok((_, mcentity, mut view_dist)) = clients.get_mut(*client) {
            view_dist.set(*view_distance);

            if let Some(mut entity) = mcentity {
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
    }

    for PlayerMove {
        client,
        position,
        yaw,
        pitch,
        on_ground,
        ..
    } in player_move.iter()
    {
        if let Ok((_, Some(mut mcentity), _)) = clients.get_mut(*client) {
            mcentity.set_position(*position);
            mcentity.set_yaw(*yaw);
            mcentity.set_head_yaw(*yaw);
            mcentity.set_pitch(*pitch);
            mcentity.set_on_ground(*on_ground);
        }
    }

    for StartSneaking { client } in start_sneaking.iter() {
        if let Ok((_, Some(mut entity), _)) = clients.get_mut(*client) {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_pose(Pose::Sneaking);
            }
        };
    }

    for StopSneaking { client } in stop_sneaking.iter() {
        if let Ok((_, Some(mut entity), _)) = clients.get_mut(*client) {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_pose(Pose::Standing);
            }
        };
    }

    for StartSprinting { client } in start_sprinting.iter() {
        if let Ok((_, Some(mut entity), _)) = clients.get_mut(*client) {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(true);
            }
        };
    }

    for StopSprinting { client } in stop_sprinting.iter() {
        if let Ok((_, Some(mut entity), _)) = clients.get_mut(*client) {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(false);
            }
        };
    }

    for HandSwing { client, hand } in swing_arm.iter() {
        if let Ok((_, Some(mut entity), _)) = clients.get_mut(*client) {
            if entity.kind() == EntityKind::Player {
                entity.trigger_animation(match hand {
                    Hand::Main => EntityAnimation::SwingMainHand,
                    Hand::Off => EntityAnimation::SwingOffHand,
                });
            }
        };
    }
}
*/
