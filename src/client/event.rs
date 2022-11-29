use std::cmp;

use anyhow::bail;
use uuid::Uuid;
use valence_protocol::entity_meta::Pose;
use valence_protocol::packets::c2s::play::{
    ClientCommand, PlayerAbilitiesC2s, ResourcePackC2s, SeenAdvancements,
};
use valence_protocol::packets::C2sPlayPacket;
use valence_protocol::types::{
    Action, ChatMode, ClickContainerMode, CommandBlockMode, Difficulty, DiggingStatus,
    DisplayedSkinParts, EntityInteraction, Hand, MainHand, RecipeBookId, StructureBlockAction,
    StructureBlockFlags, StructureBlockMirror, StructureBlockMode, StructureBlockRotation,
};
use valence_protocol::{BlockFace, BlockPos, Ident, ItemStack, VarLong};

use crate::client::Client;
use crate::config::Config;
use crate::entity::{Entity, EntityEvent, TrackedData};

/// A discrete action performed by a client.
///
/// Client events are a more convenient representation of the data contained in
/// a [`C2sPlayPacket`].
///
/// [`C2sPlayPacket`]: crate::protocol::packets::C2sPlayPacket
#[derive(Clone, Debug)]
pub enum ClientEvent {
    QueryBlockEntity {
        position: BlockPos,
        transaction_id: i32,
    },
    ChangeDifficulty(Difficulty),
    MessageAcknowledgment {
        last_seen: Vec<(Uuid, Box<[u8]>)>,
        last_received: Option<(Uuid, Box<[u8]>)>,
    },
    ChatCommand {
        command: Box<str>,
        timestamp: u64,
    },
    ChatMessage {
        message: Box<str>,
        timestamp: u64,
    },
    ChatPreview,
    PerformRespawn,
    RequestStats,
    UpdateSettings {
        /// e.g. en_US
        locale: Box<str>,
        /// The client side render distance, in chunks.
        ///
        /// The value is always in `2..=32`.
        view_distance: u8,
        chat_mode: ChatMode,
        /// `true` if the client has chat colors enabled, `false` otherwise.
        chat_colors: bool,
        displayed_skin_parts: DisplayedSkinParts,
        main_hand: MainHand,
        enable_text_filtering: bool,
        allow_server_listings: bool,
    },
    CommandSuggestionsRequest {
        transaction_id: i32,
        text: Box<str>,
    },
    ClickContainerButton {
        window_id: i8,
        button_id: i8,
    },
    ClickContainer {
        window_id: u8,
        state_id: i32,
        slot_id: i16,
        button: i8,
        mode: ClickContainerMode,
        slot_changes: Vec<(i16, Option<ItemStack>)>,
        carried_item: Option<ItemStack>,
    },
    CloseContainer {
        window_id: i8,
    },
    PluginMessage {
        channel: Ident<Box<str>>,
        data: Box<[u8]>,
    },
    EditBook {
        slot: i32,
        entries: Vec<Box<str>>,
        title: Option<Box<str>>,
    },
    QueryEntity {
        transaction_id: i32,
        entity_id: i32,
    },
    /// Left or right click interaction with an entity's hitbox.
    InteractWithEntity {
        /// The raw ID of the entity being interacted with.
        entity_id: i32,
        /// If the client was sneaking during the interaction.
        sneaking: bool,
        /// The kind of interaction that occurred.
        interact: EntityInteraction,
    },
    JigsawGenerate {
        position: BlockPos,
        levels: i32,
        keep_jigsaws: bool,
    },
    LockDifficulty(bool),
    // TODO: combine movement events?
    SetPlayerPosition {
        position: [f64; 3],
        on_ground: bool,
    },
    SetPlayerPositionAndRotation {
        position: [f64; 3],
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    SetPlayerRotation {
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    SetPlayerOnGround(bool),
    MoveVehicle {
        position: [f64; 3],
        yaw: f32,
        pitch: f32,
    },
    StartSneaking,
    StopSneaking,
    LeaveBed,
    StartSprinting,
    StopSprinting,
    StartJumpWithHorse {
        /// The power of the horse jump in `0..=100`.
        jump_boost: u8,
    },
    /// A jump while on a horse stopped.
    StopJumpWithHorse,
    /// The inventory was opened while on a horse.
    OpenHorseInventory,
    StartFlyingWithElytra,
    PaddleBoat {
        left_paddle_turning: bool,
        right_paddle_turning: bool,
    },
    PickItem {
        slot_to_use: i32,
    },
    PlaceRecipe {
        window_id: i8,
        recipe: Ident<Box<str>>,
        make_all: bool,
    },
    StopFlying,
    StartFlying,
    StartDigging {
        position: BlockPos,
        face: BlockFace,
        sequence: i32,
    },
    CancelDigging {
        position: BlockPos,
        face: BlockFace,
        sequence: i32,
    },
    FinishDigging {
        position: BlockPos,
        face: BlockFace,
        sequence: i32,
    },
    DropItem,
    DropItemStack,
    /// Eating food, pulling back bows, using buckets, etc.
    UpdateHeldItemState,
    SwapItemInHand,
    PlayerInput {
        sideways: f32,
        forward: f32,
        jump: bool,
        unmount: bool,
    },
    Pong {
        id: i32,
    },
    ChangeRecipeBookSettings {
        book_id: RecipeBookId,
        book_open: bool,
        filter_active: bool,
    },
    SetSeenRecipe {
        recipe_id: Ident<Box<str>>,
    },
    RenameItem {
        name: Box<str>,
    },
    ResourcePackLoaded,
    ResourcePackDeclined,
    ResourcePackFailedDownload,
    ResourcePackAccepted,
    OpenAdvancementTab {
        tab_id: Ident<Box<str>>,
    },
    CloseAdvancementScreen,
    SelectTrade {
        slot: i32,
    },
    SetBeaconEffect {
        primary_effect: Option<i32>,
        secondary_effect: Option<i32>,
    },
    SetHeldItem {
        slot: i16,
    },
    ProgramCommandBlock {
        position: BlockPos,
        command: Box<str>,
        mode: CommandBlockMode,
        track_output: bool,
        conditional: bool,
        automatic: bool,
    },
    ProgramCommandBlockMinecart {
        entity_id: i32,
        command: Box<str>,
        track_output: bool,
    },
    SetCreativeModeSlot {
        slot: i16,
        clicked_item: Option<ItemStack>,
    },
    ProgramJigsawBlock {
        position: BlockPos,
        name: Ident<Box<str>>,
        target: Ident<Box<str>>,
        pool: Ident<Box<str>>,
        final_state: Box<str>,
        joint_type: Box<str>,
    },
    ProgramStructureBlock {
        position: BlockPos,
        action: StructureBlockAction,
        mode: StructureBlockMode,
        name: Box<str>,
        offset_xyz: [i8; 3],
        size_xyz: [i8; 3],
        mirror: StructureBlockMirror,
        rotation: StructureBlockRotation,
        metadata: Box<str>,
        integrity: f32,
        seed: VarLong,
        flags: StructureBlockFlags,
    },
    UpdateSign {
        position: BlockPos,
        lines: [Box<str>; 4],
    },
    SwingArm(Hand),
    TeleportToEntity {
        target: Uuid,
    },
    UseItemOnBlock {
        /// The hand that was used
        hand: Hand,
        /// The location of the block that was interacted with
        position: BlockPos,
        /// The face of the block that was clicked
        face: BlockFace,
        /// The position inside of the block that was clicked on
        cursor_pos: [f32; 3],
        /// Whether or not the player's head is inside a block
        head_inside_block: bool,
        /// Sequence number for synchronization
        sequence: i32,
    },
    UseItem {
        hand: Hand,
        sequence: i32,
    },
}

pub(super) fn next_event_fallible<C: Config>(
    client: &mut Client<C>,
) -> anyhow::Result<Option<ClientEvent>> {
    loop {
        let Some(pkt) = client.recv.try_next_packet::<C2sPlayPacket>()? else {
            return Ok(None)
        };

        return Ok(Some(match pkt {
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

                continue;
            }
            C2sPlayPacket::QueryBlockEntityTag(p) => ClientEvent::QueryBlockEntity {
                position: p.position,
                transaction_id: p.transaction_id.0,
            },
            C2sPlayPacket::ChangeDifficulty(p) => ClientEvent::ChangeDifficulty(p.0),
            C2sPlayPacket::MessageAcknowledgmentC2s(p) => ClientEvent::MessageAcknowledgment {
                last_seen: p
                    .0
                    .last_seen
                    .into_iter()
                    .map(|entry| (entry.profile_id, entry.signature.into()))
                    .collect(),
                last_received: p
                    .0
                    .last_received
                    .map(|entry| (entry.profile_id, entry.signature.into())),
            },
            C2sPlayPacket::ChatCommand(p) => ClientEvent::ChatCommand {
                command: p.command.into(),
                timestamp: p.timestamp,
            },
            C2sPlayPacket::ChatMessage(p) => ClientEvent::ChatMessage {
                message: p.message.into(),
                timestamp: p.timestamp,
            },
            C2sPlayPacket::ChatPreviewC2s(_) => ClientEvent::ChatPreview,
            C2sPlayPacket::ClientCommand(p) => match p {
                ClientCommand::PerformRespawn => ClientEvent::PerformRespawn,
                ClientCommand::RequestStats => ClientEvent::RequestStats,
            },
            C2sPlayPacket::ClientInformation(p) => ClientEvent::UpdateSettings {
                locale: p.locale.into(),
                view_distance: p.view_distance,
                chat_mode: p.chat_mode,
                chat_colors: p.chat_colors,
                displayed_skin_parts: p.displayed_skin_parts,
                main_hand: p.main_hand,
                enable_text_filtering: p.enable_text_filtering,
                allow_server_listings: p.allow_server_listings,
            },
            C2sPlayPacket::CommandSuggestionsRequest(p) => ClientEvent::CommandSuggestionsRequest {
                transaction_id: p.transaction_id.0,
                text: p.text.into(),
            },
            C2sPlayPacket::ClickContainerButton(p) => ClientEvent::ClickContainerButton {
                window_id: p.window_id,
                button_id: p.button_id,
            },
            C2sPlayPacket::ClickContainer(p) => {
                // TODO: check that the slot modifications are legal.
                // TODO: update cursor item.

                for (idx, item) in &p.slots {
                    // TODO: check bounds on indices.
                    client.slots[*idx as usize] = item.clone();
                }

                ClientEvent::ClickContainer {
                    window_id: p.window_id,
                    state_id: p.state_id.0,
                    slot_id: p.slot_idx,
                    button: p.button,
                    mode: p.mode,
                    slot_changes: p.slots,
                    carried_item: p.carried_item,
                }
            }
            C2sPlayPacket::CloseContainerC2s(p) => ClientEvent::CloseContainer {
                window_id: p.window_id,
            },
            C2sPlayPacket::PluginMessageC2s(p) => ClientEvent::PluginMessage {
                channel: p.channel.into(),
                data: p.data.0.into(),
            },
            C2sPlayPacket::EditBook(p) => ClientEvent::EditBook {
                slot: p.slot.0,
                entries: p.entries.into_iter().map(From::from).collect(),
                title: p.title.map(From::from),
            },
            C2sPlayPacket::QueryEntityTag(p) => ClientEvent::QueryEntity {
                transaction_id: p.transaction_id.0,
                entity_id: p.entity_id.0,
            },
            C2sPlayPacket::Interact(p) => ClientEvent::InteractWithEntity {
                entity_id: p.entity_id.0,
                sneaking: p.sneaking,
                interact: p.interact,
            },
            C2sPlayPacket::JigsawGenerate(p) => ClientEvent::JigsawGenerate {
                position: p.position,
                levels: p.levels.0,
                keep_jigsaws: p.keep_jigsaws,
            },
            C2sPlayPacket::KeepAliveC2s(p) => {
                if client.bits.got_keepalive() {
                    bail!("unexpected keepalive");
                } else if p.id != client.last_keepalive_id {
                    bail!(
                        "keepalive IDs don't match (expected {}, got {})",
                        client.last_keepalive_id,
                        p.id
                    );
                } else {
                    client.bits.set_got_keepalive(true);
                }

                continue;
            }
            C2sPlayPacket::LockDifficulty(p) => ClientEvent::LockDifficulty(p.0),
            C2sPlayPacket::SetPlayerPosition(p) => {
                if client.pending_teleports != 0 {
                    continue;
                }

                client.position = p.position.into();

                ClientEvent::SetPlayerPosition {
                    position: p.position,
                    on_ground: p.on_ground,
                }
            }
            C2sPlayPacket::SetPlayerPositionAndRotation(p) => {
                if client.pending_teleports != 0 {
                    continue;
                }

                client.position = p.position.into();
                client.yaw = p.yaw;
                client.pitch = p.pitch;

                ClientEvent::SetPlayerPositionAndRotation {
                    position: p.position,
                    yaw: p.yaw,
                    pitch: p.pitch,
                    on_ground: p.on_ground,
                }
            }
            C2sPlayPacket::SetPlayerRotation(p) => {
                if client.pending_teleports != 0 {
                    continue;
                }

                client.yaw = p.yaw;
                client.pitch = p.pitch;

                ClientEvent::SetPlayerRotation {
                    yaw: p.yaw,
                    pitch: p.pitch,
                    on_ground: false,
                }
            }
            C2sPlayPacket::SetPlayerOnGround(p) => {
                if client.pending_teleports != 0 {
                    continue;
                }

                ClientEvent::SetPlayerOnGround(p.0)
            }
            C2sPlayPacket::MoveVehicleC2s(p) => {
                if client.pending_teleports != 0 {
                    continue;
                }

                client.position = p.position.into();
                client.yaw = p.yaw;
                client.pitch = p.pitch;

                ClientEvent::MoveVehicle {
                    position: p.position,
                    yaw: p.yaw,
                    pitch: p.pitch,
                }
            }
            C2sPlayPacket::PlayerCommand(p) => match p.action_id {
                Action::StartSneaking => ClientEvent::StartSneaking,
                Action::StopSneaking => ClientEvent::StopSneaking,
                Action::LeaveBed => ClientEvent::LeaveBed,
                Action::StartSprinting => ClientEvent::StartSprinting,
                Action::StopSprinting => ClientEvent::StopSprinting,
                Action::StartJumpWithHorse => ClientEvent::StartJumpWithHorse {
                    jump_boost: p.jump_boost.0.clamp(0, 100) as u8,
                },
                Action::StopJumpWithHorse => ClientEvent::StopJumpWithHorse,
                Action::OpenHorseInventory => ClientEvent::OpenHorseInventory,
                Action::StartFlyingWithElytra => ClientEvent::StartFlyingWithElytra,
            },
            C2sPlayPacket::PaddleBoat(p) => ClientEvent::PaddleBoat {
                left_paddle_turning: p.left_paddle_turning,
                right_paddle_turning: p.right_paddle_turning,
            },
            C2sPlayPacket::PickItem(p) => ClientEvent::PickItem {
                slot_to_use: p.slot_to_use.0,
            },
            C2sPlayPacket::PlaceRecipe(p) => ClientEvent::PlaceRecipe {
                window_id: p.window_id,
                recipe: p.recipe.into(),
                make_all: p.make_all,
            },
            C2sPlayPacket::PlayerAbilitiesC2s(p) => match p {
                PlayerAbilitiesC2s::StopFlying => ClientEvent::StopFlying,
                PlayerAbilitiesC2s::StartFlying => ClientEvent::StartFlying,
            },
            C2sPlayPacket::PlayerAction(p) => {
                if p.sequence.0 != 0 {
                    client.block_change_sequence =
                        cmp::max(p.sequence.0, client.block_change_sequence);
                }

                match p.status {
                    DiggingStatus::StartedDigging => ClientEvent::StartDigging {
                        position: p.position,
                        face: p.face,
                        sequence: p.sequence.0,
                    },
                    DiggingStatus::CancelledDigging => ClientEvent::CancelDigging {
                        position: p.position,
                        face: p.face,
                        sequence: p.sequence.0,
                    },
                    DiggingStatus::FinishedDigging => ClientEvent::FinishDigging {
                        position: p.position,
                        face: p.face,
                        sequence: p.sequence.0,
                    },
                    DiggingStatus::DropItemStack => ClientEvent::DropItemStack,
                    DiggingStatus::DropItem => ClientEvent::DropItem,
                    DiggingStatus::UpdateHeldItemState => ClientEvent::UpdateHeldItemState,
                    DiggingStatus::SwapItemInHand => ClientEvent::SwapItemInHand,
                }
            }
            C2sPlayPacket::PlayerInput(p) => ClientEvent::PlayerInput {
                sideways: p.sideways,
                forward: p.forward,
                jump: p.flags.jump(),
                unmount: p.flags.unmount(),
            },
            C2sPlayPacket::PongPlay(p) => ClientEvent::Pong { id: p.id },
            C2sPlayPacket::ChangeRecipeBookSettings(p) => ClientEvent::ChangeRecipeBookSettings {
                book_id: p.book_id,
                book_open: p.book_open,
                filter_active: p.filter_active,
            },
            C2sPlayPacket::SetSeenRecipe(p) => ClientEvent::SetSeenRecipe {
                recipe_id: p.recipe_id.into(),
            },
            C2sPlayPacket::RenameItem(p) => ClientEvent::RenameItem {
                name: p.item_name.into(),
            },
            C2sPlayPacket::ResourcePackC2s(p) => match p {
                ResourcePackC2s::SuccessfullyLoaded => ClientEvent::ResourcePackLoaded,
                ResourcePackC2s::Declined => ClientEvent::ResourcePackDeclined,
                ResourcePackC2s::FailedDownload => ClientEvent::ResourcePackFailedDownload,
                ResourcePackC2s::Accepted => ClientEvent::ResourcePackAccepted,
            },
            C2sPlayPacket::SeenAdvancements(p) => match p {
                SeenAdvancements::OpenedTab { tab_id } => ClientEvent::OpenAdvancementTab {
                    tab_id: tab_id.into(),
                },
                SeenAdvancements::ClosedScreen => ClientEvent::CloseAdvancementScreen,
            },
            C2sPlayPacket::SelectTrade(p) => ClientEvent::SelectTrade {
                slot: p.selected_slot.0,
            },
            C2sPlayPacket::SetBeaconEffect(p) => ClientEvent::SetBeaconEffect {
                primary_effect: p.primary_effect.map(|i| i.0),
                secondary_effect: p.secondary_effect.map(|i| i.0),
            },
            C2sPlayPacket::SetHeldItemC2s(p) => ClientEvent::SetHeldItem { slot: p.slot },
            C2sPlayPacket::ProgramCommandBlock(p) => ClientEvent::ProgramCommandBlock {
                position: p.position,
                command: p.command.into(),
                mode: p.mode,
                track_output: p.flags.track_output(),
                conditional: p.flags.conditional(),
                automatic: p.flags.automatic(),
            },
            C2sPlayPacket::ProgramCommandBlockMinecart(p) => {
                ClientEvent::ProgramCommandBlockMinecart {
                    entity_id: p.entity_id.0,
                    command: p.command.into(),
                    track_output: p.track_output,
                }
            }
            C2sPlayPacket::SetCreativeModeSlot(p) => ClientEvent::SetCreativeModeSlot {
                slot: p.slot,
                clicked_item: p.clicked_item,
            },
            C2sPlayPacket::ProgramJigsawBlock(p) => ClientEvent::ProgramJigsawBlock {
                position: p.position,
                name: p.name.into(),
                target: p.target.into(),
                pool: p.pool.into(),
                final_state: p.final_state.into(),
                joint_type: p.joint_type.into(),
            },
            C2sPlayPacket::ProgramStructureBlock(p) => ClientEvent::ProgramStructureBlock {
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
                seed: p.seed,
                flags: p.flags,
            },
            C2sPlayPacket::UpdateSign(p) => ClientEvent::UpdateSign {
                position: p.position,
                lines: p.lines.map(From::from),
            },
            C2sPlayPacket::SwingArm(p) => ClientEvent::SwingArm(p.0),
            C2sPlayPacket::TeleportToEntity(p) => {
                ClientEvent::TeleportToEntity { target: p.target }
            }
            C2sPlayPacket::UseItemOn(p) => {
                if p.sequence.0 != 0 {
                    client.block_change_sequence =
                        cmp::max(p.sequence.0, client.block_change_sequence);
                }

                ClientEvent::UseItemOnBlock {
                    hand: p.hand,
                    position: p.position,
                    face: p.face,
                    cursor_pos: p.cursor_pos,
                    head_inside_block: p.head_inside_block,
                    sequence: p.sequence.0,
                }
            }
            C2sPlayPacket::UseItem(p) => {
                if p.sequence.0 != 0 {
                    client.block_change_sequence =
                        cmp::max(p.sequence.0, client.block_change_sequence);
                }

                ClientEvent::UseItem {
                    hand: p.hand,
                    sequence: p.sequence.0,
                }
            }
        }));
    }
}

impl ClientEvent {
    /// Takes a client event, a client, and an entity representing the client
    /// and expresses the event in a reasonable way.
    ///
    /// For instance, movement events are expressed by changing the entity's
    /// position/rotation to match the received movement, crouching makes the
    /// entity crouch, etc.
    ///
    /// This function's primary purpose is to reduce boilerplate code in the
    /// examples, but it can be used as a quick way to get started in your own
    /// code. The precise behavior of this function is left unspecified and
    /// is subject to change.
    pub fn handle_default<C: Config>(&self, client: &mut Client<C>, entity: &mut Entity<C>) {
        match self {
            ClientEvent::RequestStats => {
                // TODO: award empty statistics
            }
            ClientEvent::UpdateSettings {
                view_distance,
                displayed_skin_parts,
                main_hand,
                ..
            } => {
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
            ClientEvent::CommandSuggestionsRequest { .. } => {}
            ClientEvent::SetPlayerPosition {
                position,
                on_ground,
            } => {
                entity.set_position(*position);
                entity.set_on_ground(*on_ground);
            }
            ClientEvent::SetPlayerPositionAndRotation {
                position,
                yaw,
                pitch,
                on_ground,
            } => {
                entity.set_position(*position);
                entity.set_yaw(*yaw);
                entity.set_head_yaw(*yaw);
                entity.set_pitch(*pitch);
                entity.set_on_ground(*on_ground);
            }
            ClientEvent::SetPlayerRotation {
                yaw,
                pitch,
                on_ground,
            } => {
                entity.set_yaw(*yaw);
                entity.set_head_yaw(*yaw);
                entity.set_pitch(*pitch);
                entity.set_on_ground(*on_ground);
            }
            ClientEvent::SetPlayerOnGround(on_ground) => entity.set_on_ground(*on_ground),
            ClientEvent::MoveVehicle {
                position,
                yaw,
                pitch,
            } => {
                entity.set_position(*position);
                entity.set_yaw(*yaw);
                entity.set_pitch(*pitch);
            }
            ClientEvent::StartSneaking => {
                if let TrackedData::Player(player) = entity.data_mut() {
                    if player.get_pose() == Pose::Standing {
                        player.set_pose(Pose::Sneaking);
                    }
                }
            }
            ClientEvent::StopSneaking => {
                if let TrackedData::Player(player) = entity.data_mut() {
                    if player.get_pose() == Pose::Sneaking {
                        player.set_pose(Pose::Standing);
                    }
                }
            }
            ClientEvent::StartSprinting => {
                if let TrackedData::Player(player) = entity.data_mut() {
                    player.set_sprinting(true);
                }
            }
            ClientEvent::StopSprinting => {
                if let TrackedData::Player(player) = entity.data_mut() {
                    player.set_sprinting(false);
                }
            }
            ClientEvent::SwingArm(hand) => {
                entity.push_event(match hand {
                    Hand::Main => EntityEvent::SwingMainHand,
                    Hand::Off => EntityEvent::SwingOffHand,
                });
            }
            _ => {}
        }
    }
}
