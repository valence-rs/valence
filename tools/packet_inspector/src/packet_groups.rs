use valence::advancement::packet::*;
use valence::client::action::*;
use valence::client::command::*;
use valence::client::custom_payload::*;
use valence::client::hand_swing::*;
use valence::client::interact_block::*;
use valence::client::interact_entity::*;
use valence::client::interact_item::*;
use valence::client::keepalive::*;
use valence::client::movement::*;
use valence::client::packet::structure_block::*;
use valence::client::packet::*;
use valence::client::resource_pack::*;
use valence::client::settings::*;
use valence::client::status::*;
use valence::client::teleport::*;
use valence::client::title::*;
use valence::entity::packet::*;
use valence::instance::packet::*;
use valence::inventory::packet::synchronize_recipes::*;
use valence::inventory::packet::*;
use valence::network::packet::*;
use valence::packet_group;
use valence::particle::*;
use valence::player_list::packet::*;
use valence::protocol::packet::boss_bar::*;
use valence::protocol::packet::chat::*;
use valence::protocol::packet::command::*;
use valence::protocol::packet::map::*;
use valence::protocol::packet::scoreboard::*;
use valence::protocol::packet::sound::*;
use valence::protocol::packet::synchronize_tags::*;

packet_group! {
    #[derive(Clone)]
    pub C2sHandshakePacket<'a> {
        HandshakeC2s<'a>
    }
}

packet_group! {
    #[derive(Clone)]
    pub S2cStatusPacket<'a> {
        QueryPongS2c,
        QueryResponseS2c<'a>,
    }
}

packet_group! {
    #[derive(Clone)]
    pub C2sStatusPacket {
        QueryPingC2s,
        QueryRequestC2s,
    }
}

packet_group! {
    #[derive(Clone)]
    pub S2cLoginPacket<'a> {
        LoginCompressionS2c,
        LoginDisconnectS2c<'a>,
        LoginHelloS2c<'a>,
        LoginQueryRequestS2c<'a>,
        LoginSuccessS2c<'a>,
    }
}

packet_group! {
    #[derive(Clone)]
    pub C2sLoginPacket<'a> {
        LoginHelloC2s<'a>,
        LoginKeyC2s<'a>,
        LoginQueryResponseC2s<'a>,
    }
}

packet_group! {
    #[derive(Clone)]
    pub S2cPlayPacket<'a> {
        AdvancementUpdateS2c<'a>,
        BlockBreakingProgressS2c,
        BlockEntityUpdateS2c<'a>,
        BlockEventS2c,
        BlockUpdateS2c,
        BossBarS2c,
        BundleSplitterS2c,
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
        MoveRelativeS2c,
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
        RotateS2c,
        RotateAndMoveRelativeS2c,
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

packet_group! {
    #[derive(Clone)]
    pub C2sPlayPacket<'a> {
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
        FullC2s,
        HandSwingC2s,
        JigsawGeneratingC2s,
        KeepAliveC2s,
        LookAndOnGroundC2s,
        MessageAcknowledgmentC2s,
        OnGroundOnlyC2s,
        PickFromInventoryC2s,
        PlayerActionC2s,
        PlayerInputC2s,
        PlayerInteractBlockC2s,
        PlayerInteractEntityC2s,
        PlayerInteractItemC2s,
        PlayerSessionC2s<'a>,
        PlayPongC2s,
        PositionAndOnGroundC2s,
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
