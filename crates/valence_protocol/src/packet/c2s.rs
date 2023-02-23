pub mod handshake {
    pub use handshake::HandshakeC2s;

    #[allow(clippy::module_inception)]
    pub mod handshake;

    packet_group! {
        #[derive(Clone)]
        C2sHandshakePacket<'a> {
            0 = HandshakeC2s<'a>
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
            0 = QueryRequestC2s,
            1 = QueryPingC2s,
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
            0 = LoginHelloC2s<'a>,
            1 = LoginKeyC2s<'a>,
            2 = LoginQueryResponseC2s<'a>,
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
    pub use player_interact::PlayerInteractC2s;
    pub use player_interact_block::PlayerInteractBlockC2s;
    pub use player_interact_item::PlayerInteractItemC2s;
    pub use player_move::{FullC2s, LookAndOnGroundC2s, OnGroundOnlyC2s, PositionAndOnGroundC2s};
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
    pub mod player_interact;
    pub mod player_interact_block;
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
            0 = TeleportConfirmC2s,
            1 = QueryBlockNbtC2s,
            2 = UpdateDifficultyC2s,
            3 = MessageAcknowledgmentC2s,
            4 = CommandExecutionC2s<'a>,
            5 = ChatMessageC2s<'a>,
            6 = ClientStatusC2s,
            7 = ClientSettingsC2s<'a>,
            8 = RequestCommandCompletionsC2s<'a>,
            9 = ButtonClickC2s,
            10 = ClickSlotC2s,
            11 = CloseHandledScreenC2s,
            12 = CustomPayloadC2s<'a>,
            13 = BookUpdateC2s<'a>,
            14 = QueryEntityNbtC2s,
            15 = PlayerInteractC2s,
            16 = JigsawGeneratingC2s,
            17 = KeepAliveC2s,
            18 = UpdateDifficultyLockC2s,
            19 = PositionAndOnGroundC2s,
            20 = FullC2s,
            21 = LookAndOnGroundC2s,
            22 = OnGroundOnlyC2s,
            23 = VehicleMoveC2s,
            24 = BoatPaddleStateC2s,
            25 = PickFromInventoryC2s,
            26 = CraftRequestC2s<'a>,
            27 = UpdatePlayerAbilitiesC2s,
            28 = PlayerActionC2s,
            29 = ClientCommandC2s,
            30 = PlayerInputC2s,
            31 = PlayPongC2s,
            32 = PlayerSessionC2s<'a>,
            33 = RecipeCategoryOptionsC2s,
            34 = RecipeBookDataC2s<'a>,
            35 = RenameItemC2s<'a>,
            36 = ResourcePackStatusC2s,
            37 = AdvancementTabC2s<'a>,
            38 = SelectMerchantTradeC2s,
            39 = UpdateBeaconC2s,
            40 = UpdateSelectedSlotC2s,
            41 = UpdateCommandBlockC2s<'a>,
            42 = UpdateCommandBlockMinecartC2s<'a>,
            43 = CreativeInventoryActionC2s,
            44 = UpdateJigsawC2s<'a>,
            45 = UpdateStructureBlockC2s<'a>,
            46 = UpdateSignC2s<'a>,
            47 = HandSwingC2s,
            48 = SpectatorTeleportC2s,
            49 = PlayerInteractBlockC2s,
            50 = PlayerInteractItemC2s
        }
    }
}
