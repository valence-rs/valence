pub mod status {
    pub use query_pong::QueryPongS2c;
    pub use query_response::QueryResponseS2c;

    pub mod query_pong;
    pub mod query_response;

    packet_group! {
        #[derive(Clone)]
        S2cStatusPacket<'a> {
            QueryPongS2c,
            QueryResponseS2c<'a>,
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

    packet_group! {
        #[derive(Clone)]
        S2cLoginPacket<'a> {
            LoginCompressionS2c,
            LoginDisconnectS2c<'a>,
            LoginHelloS2c<'a>,
            LoginQueryRequestS2c<'a>,
            LoginSuccessS2c<'a>,
        }
    }
}

pub mod play {
    pub use advancement_update::AdvancementUpdateS2c;
    pub use block_breaking_progress::BlockBreakingProgressS2c;
    pub use block_entity_update::BlockEntityUpdateS2c;
    pub use block_event::BlockEventS2c;
    pub use block_update::BlockUpdateS2c;
    pub use boss_bar::BossBarS2c;
    pub use bundle_splitter::BundleSplitter;
    pub use chat_message::ChatMessageS2c;
    pub use chat_suggestions::ChatSuggestionsS2c;
    pub use chunk_biome_data::ChunkBiomeDataS2c;
    pub use chunk_data::ChunkDataS2c;
    pub use chunk_delta_update::ChunkDeltaUpdateS2c;
    pub use chunk_load_distance::ChunkLoadDistanceS2c;
    pub use chunk_render_distance_center::ChunkRenderDistanceCenterS2c;
    pub use clear_title::ClearTitleS2c;
    pub use close_screen::CloseScreenS2c;
    pub use command_suggestions::CommandSuggestionsS2c;
    pub use command_tree::CommandTreeS2c;
    pub use cooldown_update::CooldownUpdateS2c;
    pub use craft_failed_response::CraftFailedResponseS2c;
    pub use custom_payload::CustomPayloadS2c;
    pub use damage_tilt::DamageTiltS2c;
    pub use death_message::DeathMessageS2c;
    pub use difficulty::DifficultyS2c;
    pub use disconnect::DisconnectS2c;
    pub use end_combat::EndCombatS2c;
    pub use enter_combat::EnterCombatS2c;
    pub use entities_destroy::EntitiesDestroyS2c;
    pub use entity_animation::EntityAnimationS2c;
    pub use entity_attach::EntityAttachS2c;
    pub use entity_attributes::EntityAttributesS2c;
    pub use entity_damage::EntityDamageS2c;
    pub use entity_equipment_update::EntityEquipmentUpdateS2c;
    pub use entity_move::{MoveRelative, Rotate, RotateAndMoveRelative};
    pub use entity_passengers_set::EntityPassengersSetS2c;
    pub use entity_position::EntityPositionS2c;
    pub use entity_set_head_yaw::EntitySetHeadYawS2c;
    pub use entity_spawn::EntitySpawnS2c;
    pub use entity_status::EntityStatusS2c;
    pub use entity_status_effect::EntityStatusEffectS2c;
    pub use entity_tracker_update::EntityTrackerUpdateS2c;
    pub use entity_velocity_update::EntityVelocityUpdateS2c;
    pub use experience_bar_update::ExperienceBarUpdateS2c;
    pub use experience_orb_spawn::ExperienceOrbSpawnS2c;
    pub use explosion::ExplosionS2c;
    pub use features::FeaturesS2c;
    pub use game_join::GameJoinS2c;
    pub use game_message::GameMessageS2c;
    pub use game_state_change::GameStateChangeS2c;
    pub use health_update::HealthUpdateS2c;
    pub use inventory::InventoryS2c;
    pub use item_pickup_animation::ItemPickupAnimationS2c;
    pub use keep_alive::KeepAliveS2c;
    pub use light_update::LightUpdateS2c;
    pub use look_at::LookAtS2c;
    pub use map_update::MapUpdateS2c;
    pub use nbt_query_response::NbtQueryResponseS2c;
    pub use open_horse_screen::OpenHorseScreenS2c;
    pub use open_screen::OpenScreenS2c;
    pub use open_written_book::OpenWrittenBookS2c;
    pub use overlay_message::OverlayMessageS2c;
    pub use particle::ParticleS2c;
    pub use play_ping::PlayPingS2c;
    pub use play_sound::PlaySoundS2c;
    pub use play_sound_from_entity::PlaySoundFromEntityS2c;
    pub use player_abilities::PlayerAbilitiesS2c;
    pub use player_action_response::PlayerActionResponseS2c;
    pub use player_list::PlayerListS2c;
    pub use player_list_header::PlayerListHeaderS2c;
    pub use player_position_look::PlayerPositionLookS2c;
    pub use player_remove::PlayerRemoveS2c;
    pub use player_respawn::PlayerRespawnS2c;
    pub use player_spawn::PlayerSpawnS2c;
    pub use player_spawn_position::PlayerSpawnPositionS2c;
    pub use profileless_chat_message::ProfilelessChatMessageS2c;
    pub use remove_entity_status_effect::RemoveEntityStatusEffectS2c;
    pub use remove_message::RemoveMessageS2c;
    pub use resource_pack_send::ResourcePackSendS2c;
    pub use scoreboard_display::ScoreboardDisplayS2c;
    pub use scoreboard_objective_update::ScoreboardObjectiveUpdateS2c;
    pub use scoreboard_player_update::ScoreboardPlayerUpdateS2c;
    pub use screen_handler_property_update::ScreenHandlerPropertyUpdateS2c;
    pub use screen_handler_slot_update::ScreenHandlerSlotUpdateS2c;
    pub use select_advancement_tab::SelectAdvancementTabS2c;
    pub use server_metadata::ServerMetadataS2c;
    pub use set_camera_entity::SetCameraEntityS2c;
    pub use set_trade_offers::SetTradeOffersS2c;
    pub use sign_editor_open::SignEditorOpenS2c;
    pub use simulation_distance::SimulationDistanceS2c;
    pub use statistics::StatisticsS2c;
    pub use stop_sound::StopSoundS2c;
    pub use subtitle::SubtitleS2c;
    pub use synchronize_recipes::SynchronizeRecipesS2c;
    pub use synchronize_tags::SynchronizeTagsS2c;
    pub use team::TeamS2c;
    pub use title::TitleS2c;
    pub use title_fade::TitleFadeS2c;
    pub use unload_chunk::UnloadChunkS2c;
    pub use unlock_recipes::UnlockRecipesS2c;
    pub use update_selected_slot::UpdateSelectedSlotS2c;
    pub use vehicle_move::VehicleMoveS2c;
    pub use world_border_center_changed::WorldBorderCenterChangedS2c;
    pub use world_border_initialize::WorldBorderInitializeS2c;
    pub use world_border_interpolate_size::WorldBorderInterpolateSizeS2c;
    pub use world_border_size_changed::WorldBorderSizeChangedS2c;
    pub use world_border_warning_blocks_changed::WorldBorderWarningBlocksChangedS2c;
    pub use world_border_warning_time_changed::WorldBorderWarningTimeChangedS2c;
    pub use world_event::WorldEventS2c;
    pub use world_time_update::WorldTimeUpdateS2c;

    pub mod advancement_update;
    pub mod block_breaking_progress;
    pub mod block_entity_update;
    pub mod block_event;
    pub mod block_update;
    pub mod boss_bar;
    pub mod bundle_splitter;
    pub mod chat_message;
    pub mod chat_suggestions;
    pub mod chunk_biome_data;
    pub mod chunk_data;
    pub mod chunk_delta_update;
    pub mod chunk_load_distance;
    pub mod chunk_render_distance_center;
    pub mod clear_title;
    pub mod close_screen;
    pub mod command_suggestions;
    pub mod command_tree;
    pub mod cooldown_update;
    pub mod craft_failed_response;
    pub mod custom_payload;
    pub mod damage_tilt;
    pub mod death_message;
    pub mod difficulty;
    pub mod disconnect;
    pub mod end_combat;
    pub mod enter_combat;
    pub mod entities_destroy;
    pub mod entity_animation;
    pub mod entity_attach;
    pub mod entity_attributes;
    pub mod entity_damage;
    pub mod entity_equipment_update;
    pub mod entity_move;
    pub mod entity_passengers_set;
    pub mod entity_position;
    pub mod entity_set_head_yaw;
    pub mod entity_spawn;
    pub mod entity_status;
    pub mod entity_status_effect;
    pub mod entity_tracker_update;
    pub mod entity_velocity_update;
    pub mod experience_bar_update;
    pub mod experience_orb_spawn;
    pub mod explosion;
    pub mod features;
    pub mod game_join;
    pub mod game_message;
    pub mod game_state_change;
    pub mod health_update;
    pub mod inventory;
    pub mod item_pickup_animation;
    pub mod keep_alive;
    pub mod light_update;
    pub mod look_at;
    pub mod map_update;
    pub mod nbt_query_response;
    pub mod open_horse_screen;
    pub mod open_screen;
    pub mod open_written_book;
    pub mod overlay_message;
    pub mod particle;
    pub mod play_ping;
    pub mod play_sound;
    pub mod play_sound_from_entity;
    pub mod player_abilities;
    pub mod player_action_response;
    pub mod player_list;
    pub mod player_list_header;
    pub mod player_position_look;
    pub mod player_remove;
    pub mod player_respawn;
    pub mod player_spawn;
    pub mod player_spawn_position;
    pub mod profileless_chat_message;
    pub mod remove_entity_status_effect;
    pub mod remove_message;
    pub mod resource_pack_send;
    pub mod scoreboard_display;
    pub mod scoreboard_objective_update;
    pub mod scoreboard_player_update;
    pub mod screen_handler_property_update;
    pub mod screen_handler_slot_update;
    pub mod select_advancement_tab;
    pub mod server_metadata;
    pub mod set_camera_entity;
    pub mod set_trade_offers;
    pub mod sign_editor_open;
    pub mod simulation_distance;
    pub mod statistics;
    pub mod stop_sound;
    pub mod subtitle;
    pub mod synchronize_recipes;
    pub mod synchronize_tags;
    pub mod team;
    pub mod title;
    pub mod title_fade;
    pub mod unload_chunk;
    pub mod unlock_recipes;
    pub mod update_selected_slot;
    pub mod vehicle_move;
    pub mod world_border_center_changed;
    pub mod world_border_initialize;
    pub mod world_border_interpolate_size;
    pub mod world_border_size_changed;
    pub mod world_border_warning_blocks_changed;
    pub mod world_border_warning_time_changed;
    pub mod world_event;
    pub mod world_time_update;

    packet_group! {
        #[derive(Clone)]
        S2cPlayPacket<'a> {
            AdvancementUpdateS2c<'a>,
            BlockBreakingProgressS2c,
            BlockEntityUpdateS2c<'a>,
            BlockEventS2c,
            BlockUpdateS2c,
            BossBarS2c,
            BundleSplitter,
            ChatMessageS2c<'a>,
            ChatSuggestionsS2c<'a>,
            ChunkBiomeDataS2c<'a>,
            ChunkDataS2c<'a>,
            ChunkDeltaUpdateS2c<'a>,
            ChunkLoadDistanceS2c,
            ChunkRenderDistanceCenterS2c,
            ClearTitleS2c,
            CloseScreenS2c,
            CommandSuggestionsS2c<'a>,
            CommandTreeS2c<'a>,
            CooldownUpdateS2c,
            CraftFailedResponseS2c<'a>,
            CustomPayloadS2c<'a>,
            DamageTiltS2c,
            DeathMessageS2c<'a>,
            DifficultyS2c,
            DisconnectS2c<'a>,
            EndCombatS2c,
            EnterCombatS2c,
            EntitiesDestroyS2c<'a>,
            EntityAnimationS2c,
            EntityAttachS2c,
            EntityAttributesS2c<'a>,
            EntityDamageS2c,
            EntityEquipmentUpdateS2c,
            EntityPassengersSetS2c,
            EntityPositionS2c,
            EntitySetHeadYawS2c,
            EntitySpawnS2c,
            EntityStatusEffectS2c,
            EntityStatusS2c,
            EntityTrackerUpdateS2c<'a>,
            EntityVelocityUpdateS2c,
            ExperienceBarUpdateS2c,
            ExperienceOrbSpawnS2c,
            ExplosionS2c<'a>,
            FeaturesS2c<'a>,
            GameJoinS2c<'a>,
            GameMessageS2c<'a>,
            GameStateChangeS2c,
            HealthUpdateS2c,
            InventoryS2c<'a>,
            ItemPickupAnimationS2c,
            KeepAliveS2c,
            LightUpdateS2c,
            LookAtS2c,
            MapUpdateS2c<'a>,
            MoveRelative,
            NbtQueryResponseS2c,
            OpenHorseScreenS2c,
            OpenScreenS2c<'a>,
            OpenWrittenBookS2c,
            OverlayMessageS2c<'a>,
            ParticleS2c<'a>,
            PlayerAbilitiesS2c,
            PlayerActionResponseS2c,
            PlayerListHeaderS2c<'a>,
            PlayerListS2c<'a>,
            PlayerPositionLookS2c,
            PlayerRemoveS2c<'a>,
            PlayerRespawnS2c<'a>,
            PlayerSpawnPositionS2c,
            PlayerSpawnS2c,
            PlayPingS2c,
            PlaySoundFromEntityS2c,
            PlaySoundS2c<'a>,
            ProfilelessChatMessageS2c<'a>,
            RemoveEntityStatusEffectS2c,
            RemoveMessageS2c<'a>,
            ResourcePackSendS2c<'a>,
            Rotate,
            RotateAndMoveRelative,
            ScoreboardDisplayS2c<'a>,
            ScoreboardObjectiveUpdateS2c<'a>,
            ScoreboardPlayerUpdateS2c<'a>,
            ScreenHandlerPropertyUpdateS2c,
            ScreenHandlerSlotUpdateS2c<'a>,
            SelectAdvancementTabS2c<'a>,
            ServerMetadataS2c<'a>,
            SetCameraEntityS2c,
            SetTradeOffersS2c,
            SignEditorOpenS2c,
            SimulationDistanceS2c,
            StatisticsS2c,
            StopSoundS2c<'a>,
            SubtitleS2c<'a>,
            SynchronizeRecipesS2c<'a>,
            SynchronizeTagsS2c<'a>,
            TeamS2c<'a>,
            TitleFadeS2c,
            TitleS2c<'a>,
            UnloadChunkS2c,
            UnlockRecipesS2c<'a>,
            UpdateSelectedSlotS2c,
            VehicleMoveS2c,
            WorldBorderCenterChangedS2c,
            WorldBorderInitializeS2c,
            WorldBorderInterpolateSizeS2c,
            WorldBorderSizeChangedS2c,
            WorldBorderWarningBlocksChangedS2c,
            WorldBorderWarningTimeChangedS2c,
            WorldEventS2c,
            WorldTimeUpdateS2c,
        }
    }
}
