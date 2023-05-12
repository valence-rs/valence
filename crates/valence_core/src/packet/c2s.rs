pub mod handshake {
    pub use handshake::HandshakeC2s;

    #[allow(clippy::module_inception)]
    pub mod handshake;

    packet_group! {
        #[derive(Clone)]
        C2sHandshakePacket<'a> {
            HandshakeC2s<'a>
        }
    }
}

pub mod status {
    pub use query_ping::QueryPingC2s;
    pub use query_request::QueryRequestC2s;

    pub mod query_ping;
    pub mod query_request;

    packet_group! {
        #[derive(Clone)]
        C2sStatusPacket {
            QueryPingC2s,
            QueryRequestC2s,
        }
    }
}

pub mod login {
    pub use login_hello::LoginHelloC2s;
    pub use login_key::LoginKeyC2s;
    pub use login_query_response::LoginQueryResponseC2s;

    pub mod login_hello;
    pub mod login_key;
    pub mod login_query_response;

    packet_group! {
        #[derive(Clone)]
        C2sLoginPacket<'a> {
            LoginHelloC2s<'a>,
            LoginKeyC2s<'a>,
            LoginQueryResponseC2s<'a>,
        }
    }
}

pub mod play {
    pub use advancement_tab::AdvancementTabC2s;
    pub use boat_paddle::BoatPaddleStateC2s;
    pub use book_update::BookUpdateC2s;
    pub use button_click::ButtonClickC2s;
    pub use chat_message::ChatMessageC2s;
    pub use click_slot::ClickSlotC2s;
    pub use client_command::ClientCommandC2s;
    pub use client_settings::ClientSettingsC2s;
    pub use client_status::ClientStatusC2s;
    pub use close_handled_screen::CloseHandledScreenC2s;
    pub use command_execution::CommandExecutionC2s;
    pub use craft_request::CraftRequestC2s;
    pub use creative_inventory_action::CreativeInventoryActionC2s;
    pub use custom_payload::CustomPayloadC2s;
    pub use hand_swing::HandSwingC2s;
    pub use jigsaw_generating::JigsawGeneratingC2s;
    pub use keep_alive::KeepAliveC2s;
    pub use message_acknowledgment::MessageAcknowledgmentC2s;
    pub use pick_from_inventory::PickFromInventoryC2s;
    pub use play_pong::PlayPongC2s;
    pub use player_action::PlayerActionC2s;
    pub use player_input::PlayerInputC2s;
    pub use player_interact_block::PlayerInteractBlockC2s;
    pub use player_interact_entity::PlayerInteractEntityC2s;
    pub use player_interact_item::PlayerInteractItemC2s;
    pub use player_move::{Full, LookAndOnGround, OnGroundOnly, PositionAndOnGround};
    pub use player_session::PlayerSessionC2s;
    pub use query_block_nbt::QueryBlockNbtC2s;
    pub use query_entity_nbt::QueryEntityNbtC2s;
    pub use recipe_book_data::RecipeBookDataC2s;
    pub use recipe_category_options::RecipeCategoryOptionsC2s;
    pub use rename_item::RenameItemC2s;
    pub use request_command_completions::RequestCommandCompletionsC2s;
    pub use resource_pack_status::ResourcePackStatusC2s;
    pub use select_merchant_trade::SelectMerchantTradeC2s;
    pub use spectator_teleport::SpectatorTeleportC2s;
    pub use teleport_confirm::TeleportConfirmC2s;
    pub use update_beacon::UpdateBeaconC2s;
    pub use update_command_block::UpdateCommandBlockC2s;
    pub use update_command_block_minecart::UpdateCommandBlockMinecartC2s;
    pub use update_difficulty::UpdateDifficultyC2s;
    pub use update_difficulty_lock::UpdateDifficultyLockC2s;
    pub use update_jigsaw::UpdateJigsawC2s;
    pub use update_player_abilities::UpdatePlayerAbilitiesC2s;
    pub use update_selected_slot::UpdateSelectedSlotC2s;
    pub use update_sign::UpdateSignC2s;
    pub use update_structure_block::UpdateStructureBlockC2s;
    pub use vehicle_move::VehicleMoveC2s;

    pub mod advancement_tab;
    pub mod boat_paddle;
    pub mod book_update;
    pub mod button_click;
    pub mod chat_message;
    pub mod click_slot;
    pub mod client_command;
    pub mod client_settings;
    pub mod client_status;
    pub mod close_handled_screen;
    pub mod command_execution;
    pub mod craft_request;
    pub mod creative_inventory_action;
    pub mod custom_payload;
    pub mod hand_swing;
    pub mod jigsaw_generating;
    pub mod keep_alive;
    pub mod message_acknowledgment;
    pub mod pick_from_inventory;
    pub mod play_pong;
    pub mod player_action;
    pub mod player_input;
    pub mod player_interact_block;
    pub mod player_interact_entity;
    pub mod player_interact_item;
    pub mod player_move;
    pub mod player_session;
    pub mod query_block_nbt;
    pub mod query_entity_nbt;
    pub mod recipe_book_data;
    pub mod recipe_category_options;
    pub mod rename_item;
    pub mod request_command_completions;
    pub mod resource_pack_status;
    pub mod select_merchant_trade;
    pub mod spectator_teleport;
    pub mod teleport_confirm;
    pub mod update_beacon;
    pub mod update_command_block;
    pub mod update_command_block_minecart;
    pub mod update_difficulty;
    pub mod update_difficulty_lock;
    pub mod update_jigsaw;
    pub mod update_player_abilities;
    pub mod update_selected_slot;
    pub mod update_sign;
    pub mod update_structure_block;
    pub mod vehicle_move;

    packet_group! {
        #[derive(Clone)]
        C2sPlayPacket<'a> {
            AdvancementTabC2s<'a>,
            BoatPaddleStateC2s,
            BookUpdateC2s<'a>,
            ButtonClickC2s,
            ChatMessageC2s<'a>,
            ClickSlotC2s,
            ClientCommandC2s,
            ClientSettingsC2s<'a>,
            ClientStatusC2s,
            CloseHandledScreenC2s,
            CommandExecutionC2s<'a>,
            CraftRequestC2s<'a>,
            CreativeInventoryActionC2s,
            CustomPayloadC2s<'a>,
            Full,
            HandSwingC2s,
            JigsawGeneratingC2s,
            KeepAliveC2s,
            LookAndOnGround,
            MessageAcknowledgmentC2s,
            OnGroundOnly,
            PickFromInventoryC2s,
            PlayerActionC2s,
            PlayerInputC2s,
            PlayerInteractBlockC2s,
            PlayerInteractEntityC2s,
            PlayerInteractItemC2s,
            PlayerSessionC2s<'a>,
            PlayPongC2s,
            PositionAndOnGround,
            QueryBlockNbtC2s,
            QueryEntityNbtC2s,
            RecipeBookDataC2s<'a>,
            RecipeCategoryOptionsC2s,
            RenameItemC2s<'a>,
            RequestCommandCompletionsC2s<'a>,
            ResourcePackStatusC2s,
            SelectMerchantTradeC2s,
            SpectatorTeleportC2s,
            TeleportConfirmC2s,
            UpdateBeaconC2s,
            UpdateCommandBlockC2s<'a>,
            UpdateCommandBlockMinecartC2s<'a>,
            UpdateDifficultyC2s,
            UpdateDifficultyLockC2s,
            UpdateJigsawC2s<'a>,
            UpdatePlayerAbilitiesC2s,
            UpdateSelectedSlotC2s,
            UpdateSignC2s<'a>,
            UpdateStructureBlockC2s<'a>,
            VehicleMoveC2s,
        }
    }
}
