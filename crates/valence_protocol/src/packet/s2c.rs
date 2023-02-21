pub mod status {
    pub use query_pong::QueryPongS2c;
    pub use query_response::QueryResponseS2c;

    pub mod query_pong;
    pub mod query_response;
    packet_enum! {
        #[derive(Clone)]
        S2cStatusPacket<'a> {
            QueryResponseS2c<'a>,
            QueryPongS2c,
        }
    }
}

pub mod login {
    pub use login_compression::LoginCompressionS2c;
    pub use login_disconnect::LoginDisconnectS2c;
    pub use login_hello::LoginHelloS2c;
    pub use login_query_request::LoginQueryRequestS2c;
    pub use login_success::LoginSuccessS2c;

    pub mod login_compression;
    pub mod login_disconnect;
    pub mod login_hello;
    pub mod login_query_request;
    pub mod login_success;
    packet_enum! {
        #[derive(Clone)]
        S2cLoginPacket<'a> {
            LoginDisconnectS2c<'a>,
            LoginHelloS2c<'a>,
            LoginSuccessS2c<'a>,
            LoginCompressionS2c,
            LoginQueryRequestS2c<'a>,
        }
    }
}

pub mod play {

    pub mod entity_spawn;

    use std::borrow::Cow;

    pub use entity_spawn::EntitySpawnS2c;
    pub mod experience_orb_spawn;
    pub use experience_orb_spawn::ExperienceOrbSpawnS2c;
    pub mod player_spawn;
    pub use player_spawn::PlayerSpawnS2c;
    pub mod entity_animation;
    pub use entity_animation::EntityAnimationS2c;
    pub mod statistics;
    pub use statistics::StatisticsS2c;
    pub mod player_action_response;
    pub use player_action_response::PlayerActionResponseS2c;
    pub mod block_breaking_progress;
    pub use block_breaking_progress::BlockBreakingProgressS2c;
    pub mod block_entity_update;
    pub use block_entity_update::BlockEntityUpdateS2c;
    pub mod block_event;
    pub use block_event::BlockEventS2c;
    pub mod block_update;
    pub use block_update::BlockUpdateS2c;
    pub mod boss_bar;
    pub use boss_bar::BossBarS2c;
    pub mod difficulty;
    pub use difficulty::DifficultyS2c;
    pub mod clear_titles;
    pub use clear_titles::ClearTitlesS2c;
    pub mod command_suggestions;
    pub use command_suggestions::CommandSuggestionsS2c;
    pub mod command_tree;
    pub use command_tree::CommandTreeS2c;
    pub mod close_screen;
    pub use close_screen::CloseScreenS2c;
    pub mod inventory;
    pub use inventory::InventoryS2c;
    pub mod screen_handler_property_update;
    pub use screen_handler_property_update::ScreenHandlerPropertyUpdateS2c;
    pub mod screen_handler_slot_update;
    pub use screen_handler_slot_update::ScreenHandlerSlotUpdateS2c;
    pub mod cooldown_update;
    pub use cooldown_update::CooldownUpdateS2c;
    pub mod chat_suggestions;
    pub use chat_suggestions::ChatSuggestionsS2c;
    pub mod custom_payload;
    pub use custom_payload::CustomPayloadS2c;
    pub mod remove_message;
    pub use remove_message::RemoveMessageS2c;
    pub mod disconnect;
    pub use disconnect::DisconnectS2c;
    pub mod profileless_chat_message;
    pub use profileless_chat_message::ProfilelessChatMessageS2c;
    pub mod entity_status;
    pub use entity_status::EntityStatusS2c;
    pub mod explosion;
    pub use explosion::ExplosionS2c;
    pub mod unload_chunk;
    pub use unload_chunk::UnloadChunkS2c;
    pub mod game_state_change;
    pub use game_state_change::GameStateChangeS2c;
    pub mod open_horse_screen;
    pub use open_horse_screen::OpenHorseScreenS2c;
    pub mod world_border_initialize;
    pub use world_border_initialize::WorldBorderInitializeS2c;
    pub mod keep_alive;
    pub use keep_alive::KeepAliveS2c;
    pub mod chunk_data;
    pub use chunk_data::ChunkDataS2c;
    pub mod world_event;
    pub use world_event::WorldEventS2c;
    pub mod light_update;
    pub use light_update::LightUpdateS2c;
    pub mod particle;
    pub use particle::ParticleS2c;
    pub mod game_join;
    pub use game_join::GameJoinS2c;
    pub mod map_update;
    pub use map_update::MapUpdateS2c;
    pub mod set_trade_offers;
    pub use set_trade_offers::SetTradeOffersS2c;
    pub mod entity;
    pub use entity::{MoveRelativeS2c, RotateAndMoveRelativeS2c, RotateS2c};
    pub mod vehicle_move;
    pub use vehicle_move::VehicleMoveS2c;
    pub mod open_written_book;
    pub use open_written_book::OpenWrittenBookS2c;
    pub mod open_screen;
    pub use open_screen::OpenScreenS2c;
    pub mod sign_editor_open;
    pub use sign_editor_open::SignEditorOpen;
    pub mod play_ping;
    pub use play_ping::PlayPingS2c;
    pub mod craft_failed_response;
    pub use craft_failed_response::CraftFailedResponseS2c;
    pub mod player_abilities;
    pub use player_abilities::PlayerAbilitiesS2c;
    pub mod chat_message;
    pub use chat_message::ChatMessageS2c;
    pub mod end_combat;
    pub use end_combat::EndCombatS2c;
    pub mod enter_combat;
    pub use enter_combat::EnterCombatS2c;
    pub mod death_message;
    pub use death_message::DeathMessageS2c;
    pub mod player_remove;
    pub use player_remove::PlayerRemoveS2c;
    pub mod player_list;
    pub use player_list::PlayerListS2c;
    pub mod look_at;
    pub use look_at::LookAtS2c;
    pub mod player_position_look;
    pub use player_position_look::PlayerPositionLookS2c;
    pub mod unlock_recipes;
    pub use unlock_recipes::UnlockRecipesS2c;
    pub mod entities_destroy;
    pub use entities_destroy::EntitiesDestroyS2c;
    pub mod remove_entity_status_effect;
    pub use remove_entity_status_effect::RemoveEntityStatusEffectS2c;
    pub mod resource_pack_send;
    pub use resource_pack_send::ResourcePackSendS2c;
    pub mod player_respawn;
    pub use player_respawn::PlayerRespawnS2c;
    pub mod entity_set_head_yaw;
    pub use entity_set_head_yaw::EntitySetHeadYawS2c;
    pub mod chunk_delta_update;
    pub use chunk_delta_update::ChunkDeltaUpdateS2c;
    pub mod select_advancements_tab;
    pub use select_advancements_tab::SelectAdvancementsTabS2c;
    pub mod server_metadata;
    pub use server_metadata::ServerMetadataS2c;
    pub mod overlay_message;
    pub use overlay_message::OverlayMessageS2c;
    pub mod world_border_center_changed;
    pub use world_border_center_changed::WorldBorderCenterChangedS2c;
    pub mod world_border_interpolate_size;
    pub use world_border_interpolate_size::WorldBorderInterpolateSizeS2c;
    pub mod world_border_size_changed;
    pub use world_border_size_changed::WorldBorderSizeChangedS2c;
    pub mod world_border_warning_time_changed;
    pub use world_border_warning_time_changed::WorldBorderWarningTimeChangedS2c;
    pub mod world_border_warning_blocks_changed;
    pub use world_border_warning_blocks_changed::WorldBorderWarningBlocksChangedS2c;
    pub mod set_camera_entity;
    pub use set_camera_entity::SetCameraEntityS2c;
    pub mod update_selected_slot;
    pub use update_selected_slot::UpdateSelectedSlotS2c;
    pub mod chunk_render_distance_center;
    pub use chunk_render_distance_center::ChunkRenderDistanceCenterS2c;
    pub mod chunk_load_distance;
    pub use chunk_load_distance::ChunkLoadDistanceS2c;
    pub mod player_spawn_position;
    pub use player_spawn_position::PlayerSpawnPositionS2c;
    pub mod scoreboard_display;
    pub use scoreboard_display::ScoreboardDisplayS2c;
    pub mod entity_tracker_update;
    pub use entity_tracker_update::EntityTrackerUpdateS2c;
    pub mod entity_attach;
    pub use entity_attach::EntityAttachS2c;
    pub mod entity_velocity_update;
    pub use entity_velocity_update::EntityVelocityUpdateS2c;
    pub mod entity_equipment_update;
    pub use entity_equipment_update::EntityEquipmentUpdateS2c;
    pub mod experience_bar_update;
    pub use experience_bar_update::ExperienceBarUpdateS2c;
    pub mod health_update;
    pub use health_update::HealthUpdateS2c;
    pub mod scoreboard_objective_update;
    pub use scoreboard_objective_update::ScoreboardObjectiveUpdateS2c;
    pub mod entity_passengers_set;
    pub use entity_passengers_set::EntityPassengersSetS2c;
    pub mod team;
    pub use team::TeamS2c;
    pub mod scoreboard_player_update;
    pub use scoreboard_player_update::ScoreboardPlayerUpdateS2c;
    pub mod simulation_distance;
    pub use simulation_distance::SimulationDistanceS2c;
    pub mod subtitle;
    pub use subtitle::SubtitleS2c;
    pub mod world_time_update;
    pub use world_time_update::WorldTimeUpdateS2c;
    pub mod title;
    pub use title::TitleS2c;
    pub mod title_fade;
    pub use title_fade::TitleFadeS2c;
    pub mod play_sound_from_entity;
    pub use play_sound_from_entity::PlaySoundFromEntityS2c;
    pub mod play_sound;
    pub use play_sound::PlaySoundS2c;
    pub mod stop_sound;
    pub use stop_sound::StopSoundS2c;
    pub mod game_message;
    pub use game_message::GameMessageS2c;
    pub mod player_list_header;
    pub use player_list_header::PlayerListHeaderS2c;
    pub mod nbt_query_response;
    pub use nbt_query_response::NbtQueryResponseS2c;
    pub mod item_pickup_animation;
    pub use item_pickup_animation::ItemPickupAnimationS2c;
    pub mod entity_position;
    pub use entity_position::EntityPositionS2c;
    pub mod advancement_update;
    pub use advancement_update::AdvancementUpdateS2c;
    pub mod entity_attributes;
    pub use entity_attributes::EntityAttributesS2c;
    pub mod features;
    pub use features::FeaturesS2c;
    pub mod entity_status_effect;
    pub use entity_status_effect::EntityStatusEffectS2c;
    pub mod synchronize_recipes;
    pub use synchronize_recipes::SynchronizeRecipesS2c;
    pub mod synchronize_tags;
    pub use synchronize_tags::SynchronizeTagsS2c;

    packet_enum! {
        #[derive(Clone)]
        S2cPlayPacket<'a> {
            EntitySpawnS2c,
            ExperienceOrbSpawnS2c,
            PlayerSpawnS2c,
            EntityAnimationS2c,
            StatisticsS2c,
            PlayerActionResponseS2c,
            BlockBreakingProgressS2c,
            BlockEntityUpdateS2c<'a>,
            BlockEventS2c,
            BlockUpdateS2c,
            BossBarS2c,
            DifficultyS2c,
            ClearTitlesS2c,
            CommandSuggestionsS2c<'a>,
            CommandTreeS2c<'a>,
            CloseScreenS2c,
            InventoryS2c<'a>,
            ScreenHandlerPropertyUpdateS2c,
            ScreenHandlerSlotUpdateS2c<'a>,
            CooldownUpdateS2c,
            ChatSuggestionsS2c<'a>,
            CustomPayloadS2c<'a>,
            RemoveMessageS2c<'a>,
            DisconnectS2c<'a>,
            ProfilelessChatMessageS2c<'a>,
            EntityStatusS2c,
            ExplosionS2c<'a>,
            UnloadChunkS2c,
            GameStateChangeS2c,
            OpenHorseScreenS2c,
            WorldBorderInitializeS2c,
            KeepAliveS2c,
            ChunkDataS2c<'a>,
            WorldEventS2c,
            LightUpdateS2c,
            ParticleS2c,
            GameJoinS2c<'a>,
            MapUpdateS2c<'a>,
            SetTradeOffersS2c,
            MoveRelativeS2c,
            RotateAndMoveRelativeS2c,
            RotateS2c,
            VehicleMoveS2c,
            OpenWrittenBookS2c,
            OpenScreenS2c<'a>,
            SignEditorOpen,
            PlayPingS2c,
            CraftFailedResponseS2c<'a>,
            PlayerAbilitiesS2c,
            ChatMessageS2c<'a>,
            EndCombatS2c,
            EnterCombatS2c,
            DeathMessageS2c<'a>,
            PlayerRemoveS2c<'a>,
            PlayerListS2c<'a>,
            LookAtS2c,
            PlayerPositionLookS2c,
            UnlockRecipesS2c<'a>,
            EntitiesDestroyS2c<'a>,
            RemoveEntityStatusEffectS2c,
            ResourcePackSendS2c<'a>,
            PlayerRespawnS2c<'a>,
            EntitySetHeadYawS2c,
            ChunkDeltaUpdateS2c<'a>,
            SelectAdvancementsTabS2c<'a>,
            ServerMetadataS2c<'a>,
            OverlayMessageS2c<'a>,
            WorldBorderCenterChangedS2c,
            WorldBorderInterpolateSizeS2c,
            WorldBorderSizeChangedS2c,
            WorldBorderWarningTimeChangedS2c,
            WorldBorderWarningBlocksChangedS2c,
            SetCameraEntityS2c,
            UpdateSelectedSlotS2c,
            ChunkRenderDistanceCenterS2c,
            ChunkLoadDistanceS2c,
            PlayerSpawnPositionS2c,
            ScoreboardDisplayS2c<'a>,
            EntityTrackerUpdateS2c<'a>,
            EntityAttachS2c,
            EntityVelocityUpdateS2c,
            EntityEquipmentUpdateS2c,
            ExperienceBarUpdateS2c,
            HealthUpdateS2c,
            ScoreboardObjectiveUpdateS2c<'a>,
            EntityPassengersSetS2c,
            TeamS2c<'a>,
            ScoreboardPlayerUpdateS2c<'a>,
            SimulationDistanceS2c,
            SubtitleS2c<'a>,
            WorldTimeUpdateS2c,
            TitleS2c<'a>,
            TitleFadeS2c,
            PlaySoundFromEntityS2c,
            PlaySoundS2c<'a>,
            StopSoundS2c<'a>,
            GameMessageS2c<'a>,
            PlayerListHeaderS2c<'a>,
            NbtQueryResponseS2c,
            ItemPickupAnimationS2c,
            EntityPositionS2c,
            AdvancementUpdateS2c<'a>,
            EntityAttributesS2c<'a>,
            FeaturesS2c<'a>,
            EntityStatusEffectS2c,
            SynchronizeRecipesS2c<'a>,
            SynchronizeTagsS2c<'a>,
        }
    }
}
