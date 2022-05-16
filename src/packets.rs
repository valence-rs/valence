//! Contains packet definitions and some types contained within them.
//!
//! See <https://wiki.vg/Protocol> for up to date protocol information.

#![allow(dead_code)] // TODO: remove this

use std::fmt;
use std::io::{Read, Write};

use anyhow::{bail, ensure, Context};
use bitvec::prelude::BitVec;
use num::{One, Zero};
use paste::paste;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::block_pos::BlockPos;
use crate::byte_angle::ByteAngle;
use crate::glm::{DVec3, I16Vec3, Vec3};
use crate::identifier::Identifier;
use crate::protocol::{BoundedArray, BoundedInt, BoundedString, Decode, Encode, Nbt, ReadToEnd};
use crate::var_int::VarInt;
use crate::var_long::VarLong;
use crate::Text;

/// Trait for types that can be written to the Minecraft protocol as a complete
/// packet.
///
/// A complete packet is one that starts with a `VarInt` packet ID, followed by
/// the body of the packet.
pub trait EncodePacket: fmt::Debug + private::Sealed {
    /// Writes a packet to the Minecraft protocol, including its packet ID.
    fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()>;
}

/// Trait for types that can be read from the Minecraft protocol as a complete
/// packet.
///
/// A complete packet is one that starts with a `VarInt` packet ID, followed by
/// the body of the packet.
pub trait DecodePacket: Sized + fmt::Debug + private::Sealed {
    /// Reads a packet from the Minecraft protocol, including its packet ID.
    fn decode_packet(r: &mut impl Read) -> anyhow::Result<Self>;
}

/// Defines a struct which implements [`Encode`] and [`Decode`].
///
/// The fields of the struct are encoded and decoded in the order they are
/// defined.
///
/// If a packet ID is provided after the struct name, then this struct will
/// implement [`EncodePacket`] and [`DecodePacket`].
macro_rules! def_struct {
    (
        $(#[$struct_attrs:meta])*
        $name:ident $($id:literal)? {
            $(
                $(#[$field_attrs:meta])*
                $field:ident: $typ:ty
            ),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug)]
        $(#[$struct_attrs])*
        pub struct $name {
            $(
                $(#[$field_attrs])*
                pub $field: $typ,
            )*
        }

        impl Encode for $name {
            fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
                $(
                    Encode::encode(&self.$field, _w)
                        .context(concat!("failed to write field `", stringify!($field), "` from struct `", stringify!($name), "`"))?;
                )*
                Ok(())
            }
        }

        impl Decode for $name {
            fn decode(_r: &mut impl Read) -> anyhow::Result<Self> {
                $(
                    let $field: $typ = Decode::decode(_r)
                        .context(concat!("failed to read field `", stringify!($field), "` from struct `", stringify!($name), "`"))?;
                )*

                Ok(Self {
                    $(
                        $field,
                    )*
                })
            }
        }

        $(
            impl private::Sealed for $name {}

            impl EncodePacket for $name {
                fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
                    VarInt($id)
                        .encode(w)
                        .context(concat!("failed to write packet ID for `", stringify!($name), "`"))?;
                    self.encode(w)
                }
            }

            impl DecodePacket for $name {
                fn decode_packet(r: &mut impl Read) -> anyhow::Result<Self> {
                    let VarInt(packet_id) = VarInt::decode(r)
                        .context(concat!("failed to read packet ID for `", stringify!($name), "`"))?;

                    ensure!(
                        $id == packet_id,
                        concat!("bad packet ID for `", stringify!($name), "` (expected {:#04x}, got {:#04x})"),
                        $id,
                        packet_id
                    );
                    Self::decode(r)
                }
            }

            impl $name {
                #[allow(unused)]
                const PACKET_ID: i32 = $id;
            }
        )*

        // TODO: https://github.com/rust-lang/rust/issues/48214
        //impl Copy for $name
        //where
        //    $(
        //        $typ: Copy
        //    )*
        //{}
    }
}

/// Defines an enum which implements [`Encode`] and [`Decode`].
///
/// The enum tag is encoded and decoded first, followed by the appropriate
/// variant.
///
/// If a packet ID is provided after the struct name, then this struct will
/// implement [`EncodePacket`] and [`DecodePacket`].
macro_rules! def_enum {
    (
        $(#[$enum_attrs:meta])*
        $name:ident $($id:literal)?: $tag_ty:ty {
            $(
                $(#[$variant_attrs:meta])*
                $variant:ident$(: $typ:ty)? = $lit:literal
            ),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug)]
        $(#[$enum_attrs])*
        pub enum $name {
            $(
                $(#[$variant_attrs])*
                $variant$(($typ))?,
            )*
        }

        impl Encode for $name {
            fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
                match self {
                    $(
                        if_typ_is_empty_pat!($($typ)?, $name::$variant, $name::$variant(val)) => {
                            <$tag_ty>::encode(&$lit.into(), _w)
                                .context(concat!("failed to write enum tag for `", stringify!($name), "`"))?;

                            if_typ_is_empty_expr!($($typ)?, Ok(()), {
                                Encode::encode(val, _w)
                                    .context(concat!("failed to write variant `", stringify!($variant), "` from enum `", stringify!($name), "`"))
                            })
                        },
                    )*

                    // Need this because references to uninhabited enums are considered inhabited.
                    #[allow(unreachable_patterns)]
                    _ => unreachable!("uninhabited enum?")
                }
            }
        }

        impl Decode for $name {
            fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
                let tag_ctx = concat!("failed to read enum tag for `", stringify!($name), "`");
                let tag = <$tag_ty>::decode(r).context(tag_ctx)?.into();
                match tag {
                    $(
                        $lit => {
                            if_typ_is_empty_expr!($($typ)?, Ok($name::$variant), {
                                $(
                                    let res: $typ = Decode::decode(r)
                                        .context(concat!("failed to read variant `", stringify!($variant), "` from enum `", stringify!($name), "`"))?;
                                    Ok($name::$variant(res))
                                )?
                            })
                        }
                    )*
                    _ => bail!(concat!("bad tag value for enum `", stringify!($name), "`"))
                }
            }
        }

        $(
            impl private::Sealed for $name {}

            impl EncodePacket for $name {
                fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
                    VarInt($id)
                        .encode(w)
                        .context(concat!("failed to write packet ID for `", stringify!($name), "`"))?;
                    self.encode(w)
                }
            }

            impl DecodePacket for $name {
                fn decode_packet(r: &mut impl Read) -> anyhow::Result<Self> {
                    let VarInt(packet_id) = VarInt::decode(r)
                        .context(concat!("failed to read packet ID for `", stringify!($name), "`"))?;

                    ensure!(
                        $id == packet_id,
                        concat!("bad packet ID for `", stringify!($name), "` (expected {:#04X}, got {:#04X})"),
                        $id,
                        packet_id
                    );
                    Self::decode(r)
                }
            }

            impl $name {
                #[allow(unused)]
                const PACKET_ID: i32 = $id;
            }
        )*
    }
}

macro_rules! if_typ_is_empty_expr {
    (, $t:expr, $f:expr) => {
        $t
    };
    ($typ:ty, $t:expr, $f:expr) => {
        $f
    };
}

macro_rules! if_typ_is_empty_pat {
    (, $t:pat, $f:pat) => {
        $t
    };
    ($typ:ty, $t:pat, $f:pat) => {
        $f
    };
}

macro_rules! def_bitfield {
    (
        $(#[$struct_attrs:meta])*
        $name:ident: $inner_ty:ty {
            $(
                $(#[$bit_attrs:meta])*
                $bit:ident = $offset:literal
            ),* $(,)?
        }
    ) => {
        // TODO: custom Debug impl.
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        $(#[$struct_attrs])*
        pub struct $name($inner_ty);

        impl $name {
            pub fn new(
                $(
                    $bit: bool,
                )*
            ) -> Self {
                let mut res = Self(Default::default());
                paste! {
                    $(
                        res = res.[<set_ $bit:snake>]($bit);
                    )*
                }
                res
            }

            paste! {
                $(
                    #[doc = "Gets the " $bit " bit on this bitfield.\n"]
                    $(#[$bit_attrs])*
                    pub fn [<get_ $bit:snake>](self) -> bool {
                        self.0 & <$inner_ty>::one() << <$inner_ty>::from($offset) != <$inner_ty>::zero()
                    }

                    #[doc = "Sets the " $bit " bit on this bitfield.\n"]
                    $(#[$bit_attrs])*
                    #[must_use]
                    pub fn [<set_ $bit:snake>](self, $bit: bool) -> Self {
                        let mask = <$inner_ty>::one() << <$inner_ty>::from($offset);
                        if $bit {
                            Self(self.0 | mask)
                        } else {
                            Self(self.0 & !mask)
                        }
                    }
                )*
            }
        }

        impl $crate::protocol::Encode for $name {
            fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
                self.0.encode(w)
            }
        }

        impl $crate::protocol::Decode for $name {
            fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
                <$inner_ty>::decode(r).map(Self)
            }
        }
    }
}

mod private {
    pub trait Sealed {}
}

/// Packets and types used during the handshaking state.
pub mod handshake {
    use super::*;

    def_struct! {
        Handshake 0x00 {
            protocol_version: VarInt,
            server_adddress: BoundedString<0, 255>,
            server_port: u16,
            next_state: HandshakeNextState,
        }
    }

    def_enum! {
        HandshakeNextState: VarInt {
            Status = 1,
            Login = 2,
        }
    }
}

/// Packets and types used during the status state.
pub mod status {
    use super::*;

    // ==== Clientbound ====

    def_struct! {
        Response 0x00 {
            json_response: String
        }
    }

    def_struct! {
        Pong 0x01 {
            /// Should be the same as the payload from [`Ping`].
            payload: u64
        }
    }

    // ==== Serverbound ====

    def_struct! {
        Request 0x00 {}
    }

    def_struct! {
        Ping 0x01 {
            payload: u64
        }
    }
}

/// Packets and types used during the play state.
pub mod login {
    use super::*;

    // ==== Clientbound ====

    def_struct! {
        Disconnect 0x00 {
            reason: Text,
        }
    }

    def_struct! {
        EncryptionRequest 0x01 {
            /// Currently unused
            server_id: BoundedString<0, 20>,
            /// The RSA public key
            public_key: Vec<u8>,
            verify_token: BoundedArray<u8, 16, 16>,
        }
    }

    def_struct! {
        LoginSuccess 0x02 {
            uuid: Uuid,
            username: BoundedString<3, 16>,
        }
    }

    def_struct! {
        SetCompression 0x03 {
            threshold: VarInt
        }
    }

    // ==== Serverbound ====

    def_struct! {
        LoginStart 0x00 {
            username: BoundedString<3, 16>,
        }
    }

    def_struct! {
        EncryptionResponse 0x01 {
            shared_secret: BoundedArray<u8, 16, 128>,
            verify_token: BoundedArray<u8, 16, 128>,
        }
    }
}

/// Packets and types used during the play state.
pub mod play {
    use super::*;

    // ==== Clientbound ====

    def_struct! {
        SpawnEntity 0x00 {
            entity_id: VarInt,
            object_uuid: Uuid,
            typ: VarInt,
            position: DVec3,
            pitch: ByteAngle,
            yaw: ByteAngle,
            data: i32,
            velocity: I16Vec3,
        }
    }

    def_struct! {
        SpawnExperienceOrb 0x01 {
            entity_id: VarInt,
            position: DVec3,
            count: i16,
        }
    }

    def_struct! {
        SpawnLivingEntity 0x02 {
            entity_id: VarInt,
            entity_uuid: Uuid,
            typ: VarInt,
            position: DVec3,
            yaw: ByteAngle,
            pitch: ByteAngle,
            head_pitch: ByteAngle,
            velocity: I16Vec3,
        }
    }

    def_struct! {
        SpawnPainting 0x03 {
            entity_id: VarInt,
            entity_uuid: Uuid,
            variant: VarInt, // TODO: painting ID enum
            location: BlockPos,
            direction: PaintingDirection,
        }
    }

    def_enum! {
        PaintingDirection: u8 {
            South = 0,
            West = 1,
            North = 2,
            East = 3,
        }
    }

    def_struct! {
        SpawnPlayer 0x04 {
            entity_id: VarInt,
            player_uuid: Uuid,
            position: DVec3,
            yaw: ByteAngle,
            pitch: ByteAngle,
        }
    }

    def_struct! {
        SculkVibrationSignal 0x05 {
            source_position: BlockPos,
            destination_identifier: Identifier, // TODO: destination codec type?
            destination: BlockPos, // TODO: this type varies depending on destination_identifier
            arrival_ticks: VarInt,
        }
    }

    def_struct! {
        EntityAnimation 0x06 {
            entity_id: VarInt,
            animation: Animation,
        }
    }

    def_enum! {
        Animation: u8 {
            SwingMainArm = 0,
            TakeDamage = 1,
            LeaveBed = 2,
            SwingOffhand = 3,
            CriticalEffect = 4,
            MagicCriticalEffect = 5,
        }
    }

    def_struct! {
        AcknoledgePlayerDigging 0x08 {
            location: BlockPos,
            block: VarInt, // TODO: block state ID type.
            status: VarInt, // TODO: VarInt enum here.
            sucessful: bool,
        }
    }

    def_struct! {
        BlockBreakAnimation 0x09 {
            entity_id: VarInt,
            location: BlockPos,
            destroy_stage: BoundedInt<u8, 0, 10>,
        }
    }

    def_struct! {
        BlockEntityData 0x0a {
            location: BlockPos,
            typ: VarInt, // TODO: use enum here
            data: nbt::Blob,
        }
    }

    def_struct! {
        BlockAction 0x0b {
            location: BlockPos,
            action_id: u8,
            action_param: u8,
            block_type: VarInt,
        }
    }

    def_struct! {
        BlockChange 0x0c {
            location: BlockPos,
            block_id: VarInt,
        }
    }

    def_struct! {
        BossBar 0x0d {
            uuid: Uuid,
            action: BossBarAction,
        }
    }

    def_enum! {
        BossBarAction: VarInt {
            Add: BossBarActionAdd = 0,
        }
    }

    def_struct! {
        BossBarActionAdd {
            title: Text,
            health: f32,
            color: BossBarColor,
            division: BossBarDivision,
            /// TODO: bitmask
            flags: u8,
        }
    }

    def_enum! {
        BossBarColor: VarInt {
            Pink = 0,
            Blue = 1,
            Red = 2,
            Green = 3,
            Yellow = 4,
            Purple = 5,
            White = 6,
        }
    }

    def_enum! {
        BossBarDivision: VarInt {
            NoDivision = 0,
            SixNotches = 1,
            TenNotches = 2,
            TwelveNotches = 3,
            TwentyNotches = 4,
        }
    }

    def_struct! {
        ServerDifficulty 0x0e {
            difficulty: Difficulty,
            locked: bool,
        }
    }

    def_enum! {
        Difficulty: u8 {
            Peaceful = 0,
            Easy = 1,
            Normal = 2,
            Hard = 3,
        }
    }

    def_struct! {
        ChatMessageClientbound 0x0f {
            message: Text,
            position: ChatMessagePosition,
            sender: Uuid,
        }
    }

    def_enum! {
        ChatMessagePosition: u8 {
            Chat = 0,
            SystemMessage = 1,
            GameInfo = 2,
        }
    }

    def_struct! {
        ClearTitles 0x10 {
            reset: bool,
        }
    }

    def_struct! {
        TabComplete 0x11 {
            id: VarInt,
            start: VarInt,
            length: VarInt,
            matches: Vec<TabCompleteMatch>,
        }
    }

    def_struct! {
        TabCompleteMatch {
            value: String,
            tooltip: TabCompleteTooltip,
        }
    }

    def_enum! {
        TabCompleteTooltip: u8 {
            NoTooltip = 0,
            Tooltip: Text = 1,
        }
    }

    def_struct! {
        WindowProperty 0x15 {
            // TODO: use enums
            window_id: u8,
            property: i16,
            value: i16,
        }
    }

    def_struct! {
        SetCooldown 0x17 {
            item_id: VarInt,
            cooldown_ticks: VarInt,
        }
    }

    def_struct! {
        Disconnect 0x1a {
            reason: Text,
        }
    }

    def_struct! {
        EntityStatus 0x1b {
            entity_id: i32,
            /// TODO: enum
            entity_status: u8,
        }
    }

    def_struct! {
        UnloadChunk 0x1d {
            chunk_x: i32,
            chunk_z: i32
        }
    }

    def_struct! {
        ChangeGameState 0x1e {
            reason: ChangeGameStateReason,
            value: f32,
        }
    }

    def_enum! {
        ChangeGameStateReason: u8 {
            NoRespawnBlockAvailable = 0,
            EndRaining = 1,
            BeginRaining = 2,
            ChangeGameMode = 3,
            WinGame = 4,
            DemoEvent = 5,
            ArrowHitPlayer = 6,
            RainLevelChange = 7,
            ThunderLevelChange = 8,
            PlayPufferfishStingSound = 9,
            PlayElderGuardianMobAppearance = 10,
            EnableRespawnScreen = 11,
        }
    }

    def_struct! {
        OpenHorseWindow 0x1f {
            window_id: u8,
            slot_count: VarInt,
            entity_id: i32,
        }
    }

    def_struct! {
        InitializeWorldBorder 0x20 {
            x: f64,
            z: f64,
            old_diameter: f64,
            new_diameter: f64,
            speed: VarLong,
            portal_teleport_boundary: VarInt,
            warning_blocks: VarInt,
            warning_time: VarInt,
        }
    }

    def_struct! {
        KeepAliveClientbound 0x21 {
            id: i64,
        }
    }

    def_struct! {
        ChunkDataAndUpdateLight 0x22 {
            chunk_x: i32,
            chunk_z: i32,
            heightmaps: Nbt<ChunkDataHeightmaps>,
            blocks_and_biomes: Vec<u8>,
            block_entities: Vec<ChunkDataBlockEntity>,
            trust_edges: bool,
            sky_light_mask: BitVec<u64>,
            block_light_mask: BitVec<u64>,
            empty_sky_light_mask: BitVec<u64>,
            empty_block_light_mask: BitVec<u64>,
            sky_light_arrays: Vec<[u8; 2048]>,
            block_light_arrays: Vec<[u8; 2048]>,
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ChunkDataHeightmaps {
        #[serde(rename = "MOTION_BLOCKING", serialize_with = "nbt::i64_array")]
        pub motion_blocking: Vec<i64>,
    }

    def_struct! {
        ChunkDataBlockEntity {
            packed_xz: i8,
            y: i16,
            typ: VarInt,
            data: nbt::Blob,
        }
    }

    def_struct! {
        JoinGame 0x26 {
            /// Entity ID of the joining player
            entity_id: i32,
            is_hardcore: bool,
            gamemode: GameMode,
            /// The previous gamemode for the purpose of the F3+F4 gamemode switcher. (TODO: verify)
            /// Is `-1` if there was no previous gamemode.
            previous_gamemode: GameMode,
            dimension_names: Vec<Identifier>,
            dimension_codec: Nbt<DimensionCodec>,
            /// The specification of the dimension being spawned into.
            dimension: Nbt<DimensionType>,
            /// The identifier of the dimension being spawned into.
            dimension_name: Identifier,
            /// Hash of the world's seed used for client biome noise.
            hashed_seed: i64,
            /// No longer used by the client.
            max_players: VarInt,
            view_distance: BoundedInt<VarInt, 2, 32>,
            simulation_distance: VarInt,
            /// If reduced debug info should be shown on the F3 screen.
            reduced_debug_info: bool,
            /// If player respawns should be instant or not.
            enable_respawn_screen: bool,
            is_debug: bool,
            /// If this is a superflat world.
            /// Superflat worlds have different void fog and horizon levels.
            is_flat: bool,
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DimensionCodec {
        #[serde(rename = "minecraft:dimension_type")]
        pub dimension_type_registry: DimensionTypeRegistry,
        #[serde(rename = "minecraft:worldgen/biome")]
        pub biome_registry: BiomeRegistry,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DimensionTypeRegistry {
        #[serde(rename = "type")]
        pub typ: Identifier,
        pub value: Vec<DimensionTypeRegistryEntry>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DimensionTypeRegistryEntry {
        pub name: Identifier,
        pub id: i32,
        pub element: DimensionType,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DimensionType {
        pub piglin_safe: bool,
        pub natural: bool,
        pub ambient_light: f32,
        pub fixed_time: Option<i64>,
        pub infiniburn: String, // TODO: tag type?
        pub respawn_anchor_works: bool,
        pub has_skylight: bool,
        pub bed_works: bool,
        pub effects: Identifier,
        pub has_raids: bool,
        pub min_y: i32,
        pub height: i32,
        pub logical_height: i32,
        pub coordinate_scale: f64,
        pub ultrawarm: bool,
        pub has_ceiling: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeRegistry {
        #[serde(rename = "type")]
        pub typ: Identifier,
        pub value: Vec<Biome>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Biome {
        pub name: Identifier,
        pub id: i32,
        pub element: BiomeProperty,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeProperty {
        pub precipitation: String,
        pub depth: f32,
        pub temperature: f32,
        pub scale: f32,
        pub downfall: f32,
        pub category: String,
        pub temperature_modifier: Option<String>,
        pub effects: BiomeEffects,
        pub particle: Option<BiomeParticle>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeEffects {
        pub sky_color: i32,
        pub water_fog_color: i32,
        pub fog_color: i32,
        pub water_color: i32,
        pub foliage_color: Option<i32>,
        pub grass_color: Option<i32>,
        pub grass_color_modifier: Option<String>,
        pub music: Option<BiomeMusic>,
        pub ambient_sound: Option<Identifier>,
        pub additions_sound: Option<BiomeAdditionsSound>,
        pub mood_sound: Option<BiomeMoodSound>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeMusic {
        pub replace_current_music: bool,
        pub sound: Identifier,
        pub max_delay: i32,
        pub min_delay: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeAdditionsSound {
        pub sound: Identifier,
        pub tick_chance: f64,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeMoodSound {
        pub sound: Identifier,
        pub tick_delay: i32,
        pub offset: f64,
        pub block_search_extent: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeParticle {
        pub probability: f32,
        pub options: BiomeParticleOptions,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeParticleOptions {
        #[serde(rename = "type")]
        pub typ: Identifier,
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        GameMode: u8 {
            Survival = 0,
            Creative = 1,
            Adventure = 2,
            Spectator = 3,
        }
    }

    impl Default for GameMode {
        fn default() -> Self {
            GameMode::Survival
        }
    }

    def_struct! {
        PlayerPositionAndLook 0x38 {
            position: DVec3,
            yaw: f32,
            pitch: f32,
            flags: PlayerPositionAndLookFlags,
            teleport_id: VarInt,
            dismount_vehicle: bool,
        }
    }

    def_bitfield! {
        PlayerPositionAndLookFlags: u8 {
            x = 0,
            y = 1,
            z = 2,
            y_rot = 3,
            x_rot = 4,
        }
    }

    def_struct! {
        DestroyEntities 0x3a {
            entities: Vec<VarInt>,
        }
    }

    def_struct! {
        MultiBlockChange 0x3f {
            chunk_section_position: u64,
            invert_trust_edges: bool,
            blocks: Vec<u64>,
        }
    }

    def_struct! {
        HeldItemChangeClientbound 0x48 {
            slot: BoundedInt<u8, 0, 9>,
        }
    }

    def_struct! {
        UpdateViewPosition 0x49 {
            chunk_x: VarInt,
            chunk_z: VarInt,
        }
    }

    def_struct! {
        UpdateViewDistance 0x4a {
            view_distance: BoundedInt<VarInt, 2, 32>,
        }
    }

    def_struct! {
        SpawnPosition 0x4b {
            location: BlockPos,
            angle: f32,
        }
    }

    def_struct! {
        EntityMetadata 0x4d {
            entity_id: VarInt,
            metadata: ReadToEnd,
        }
    }

    def_struct! {
        TimeUpdate 0x59 {
            /// The age of the world in 1/20ths of a second.
            world_age: i64,
            /// The current time of day in 1/20ths of a second.
            /// The value should be in the range \[0, 24000].
            /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
            time_of_day: i64,
        }
    }

    macro_rules! def_client_play_packet_enum {
        {
            $($packet:ident),* $(,)?
        } => {
            /// An enum of all clientbound play packets.
            #[derive(Clone, Debug)]
            pub enum ClientPlayPacket {
                $($packet($packet)),*
            }

            impl private::Sealed for ClientPlayPacket {}

            $(
                impl From<$packet> for ClientPlayPacket {
                    fn from(p: $packet) -> ClientPlayPacket {
                        ClientPlayPacket::$packet(p)
                    }
                }
            )*

            impl EncodePacket for ClientPlayPacket {
                fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
                    match self {
                        $(
                            Self::$packet(p) => {
                                VarInt($packet::PACKET_ID)
                                    .encode(w)
                                    .context(concat!("failed to write play packet ID for `", stringify!($packet), "`"))?;
                                p.encode(w)
                            }
                        )*
                    }
                }
            }

            #[cfg(test)]
            #[test]
            fn test_client_play_packet_order() {
                let ids = [
                    $(
                        (stringify!($packet), $packet::PACKET_ID),
                    )*
                ];

                if let Some(w) = ids.windows(2).find(|w| w[0].1 >= w[1].1) {
                    panic!("the {} and {} variants of the client play packet enum are not properly sorted by their packet ID", w[0].0, w[1].0);
                }
            }
        }
    }

    def_client_play_packet_enum! {
        SpawnEntity,
        SpawnExperienceOrb,
        SpawnLivingEntity,
        SpawnPainting,
        SpawnPlayer,
        SculkVibrationSignal,
        EntityAnimation,
        AcknoledgePlayerDigging,
        BlockBreakAnimation,
        BlockEntityData,
        BlockAction,
        BlockChange,
        BossBar,
        Disconnect,
        EntityStatus,
        UnloadChunk,
        ChangeGameState,
        KeepAliveClientbound,
        ChunkDataAndUpdateLight,
        JoinGame,
        PlayerPositionAndLook,
        DestroyEntities,
        MultiBlockChange,
        HeldItemChangeClientbound,
        UpdateViewPosition,
        UpdateViewDistance,
        SpawnPosition,
        EntityMetadata,
        TimeUpdate,
    }

    // ==== Serverbound ====

    def_struct! {
        TeleportConfirm 0x00 {
            teleport_id: VarInt
        }
    }

    def_struct! {
        QueryBlockNbt 0x01 {
            transaction_id: VarInt,
            location: BlockPos,
        }
    }

    def_enum! {
        SetDifficulty 0x02: i8 {
            Peaceful = 0,
            Easy = 1,
            Normal = 2,
            Hard = 3,
        }
    }

    def_struct! {
        ChatMessageServerbound 0x03 {
            message: BoundedString<0, 256>
        }
    }

    def_enum! {
        ClientStatus 0x04: VarInt {
            /// Sent when ready to complete login and ready to respawn after death.
            PerformRespawn = 0,
            /// Sent when the statistics menu is opened.
            RequestStatus = 1,
        }
    }

    def_struct! {
        ClientSettings 0x05 {
            /// e.g. en_US
            locale: BoundedString<0, 16>,
            /// Client-side render distance in chunks.
            view_distance: BoundedInt<u8, 2, 32>,
            chat_mode: ChatMode,
            chat_colors: bool,
            displayed_skin_parts: DisplayedSkinParts,
            main_hand: MainHand,
            /// Currently always false
            enable_text_filtering: bool,
            /// False if the client should not show up in the hover preview.
            allow_server_listings: bool,
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        ChatMode: VarInt {
            Enabled = 0,
            CommandsOnly = 1,
            Hidden = 2,
        }
    }

    def_bitfield! {
        DisplayedSkinParts: u8 {
            cape = 0,
            jacket = 1,
            left_sleeve = 2,
            right_sleeve = 3,
            left_pants_leg = 4,
            right_pants_leg = 5,
            hat = 6,
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        MainHand: VarInt {
            Left = 0,
            Right = 1,
        }
    }

    def_struct! {
        TabCompleteServerbound 0x06 {
            transaction_id: VarInt,
            /// Text behind the cursor without the '/'.
            text: BoundedString<0, 32500>
        }
    }

    def_struct! {
        ClickWindowButton 0x07 {
            window_id: i8,
            button_id: i8,
        }
    }

    def_struct! {
        ClickWindow 0x08 {
            window_id: u8,
            state_id: VarInt,
            slot: i16,
            button: i8,
            mode: VarInt, // TODO: enum
            // TODO
        }
    }

    def_struct! {
        CloseWindow 0x09 {
            window_id: u8,
        }
    }

    def_struct! {
        PluginMessageServerbound 0x0a {
            channel: Identifier,
            data: ReadToEnd,
        }
    }

    def_struct! {
        EditBook 0x0b {
            hand: Hand,
            entries: Vec<String>,
            title: Option<String>,
        }
    }

    def_enum! {
        Hand: VarInt {
            Main = 0,
            Off = 1,
        }
    }

    def_struct! {
        QueryEntityNbt 0x0c {
            transaction_id: VarInt,
            entity_id: VarInt,
        }
    }

    def_struct! {
        InteractEntity 0x0d {
            entity_id: VarInt,
            typ: InteractType,
            sneaking: bool,
        }
    }

    def_enum! {
        InteractType: VarInt {
            Interact: Hand = 0,
            Attack = 1,
            InteractAt: InteractAtData = 2
        }
    }

    def_struct! {
        InteractAtData {
            target: Vec3,
            hand: Hand,
        }
    }

    def_struct! {
        GenerateStructure 0x0e {
            location: BlockPos,
            levels: VarInt,
            keep_jigsaws: bool,
        }
    }

    def_struct! {
        KeepAliveServerbound 0x0f {
            id: i64,
        }
    }

    def_struct! {
        LockDifficulty 0x10 {
            locked: bool
        }
    }

    def_struct! {
        PlayerPosition 0x11 {
            position: DVec3,
            on_ground: bool,
        }
    }

    def_struct! {
        PlayerPositionAndRotation 0x12 {
            // Absolute position
            position: DVec3,
            /// Absolute rotation on X axis in degrees.
            yaw: f32,
            /// Absolute rotation on Y axis in degrees.
            pitch: f32,
            on_ground: bool,
        }
    }

    def_struct! {
        PlayerRotation 0x13 {
            /// Absolute rotation on X axis in degrees.
            yaw: f32,
            /// Absolute rotation on Y axis in degrees.
            pitch: f32,
            on_ground: bool,
        }
    }

    def_struct! {
        PlayerMovement 0x14 {
            on_ground: bool
        }
    }

    def_struct! {
        VehicleMoveServerbound 0x15 {
            /// Absolute position
            position: DVec3,
            /// Degrees
            yaw: f32,
            /// Degrees
            pitch: f32,
        }
    }

    def_struct! {
        SteerBoat 0x16 {
            left_paddle_turning: bool,
            right_paddle_turning: bool,
        }
    }

    def_struct! {
        PickItem 0x17 {
            slot_to_use: VarInt,
        }
    }

    def_struct! {
        CraftRecipeRequest 0x18 {
            window_id: i8,
            recipe: Identifier,
            make_all: bool,
        }
    }

    def_enum! {
        PlayerAbilitiesServerbound 0x19: i8 {
            NotFlying = 0,
            Flying = 0b10,
        }
    }

    def_struct! {
        PlayerDigging 0x1a {
            status: DiggingStatus,
            location: BlockPos,
            face: BlockFace,
        }
    }

    def_enum! {
        DiggingStatus: VarInt {
            StartedDigging = 0,
            CancelledDigging = 1,
            FinishedDigging = 2,
            DropItemStack = 3,
            DropItem = 4,
            ShootArrowOrFinishEating = 5,
            SwapItemInHand = 6,
        }
    }

    def_enum! {
        BlockFace: i8 {
            /// -Y
            Bottom = 0,
            /// +Y
            Top = 1,
            /// -Z
            North = 2,
            /// +Z
            South = 3,
            /// -X
            West = 4,
            /// +X
            East = 5,
        }
    }

    def_struct! {
        EntityAction 0x1b {
            entity_id: VarInt,
            action_id: EntityActionId,
            jump_boost: BoundedInt<VarInt, 0, 100>,
        }
    }

    def_enum! {
        EntityActionId: VarInt {
            StartSneaking = 0,
            StopSneaking = 1,
            LeaveBed = 2,
            StartSprinting = 3,
            StopSprinting = 4,
            StartJumpWithHorse = 5,
            StopJumpWithHorse = 6,
            OpenHorseInventory = 7,
            StartFlyingWithElytra = 8,
        }
    }

    def_struct! {
        SteerVehicle 0x1c {
            sideways: f32,
            forward: f32,
            flags: SteerVehicleFlags,
        }
    }

    def_bitfield! {
        SteerVehicleFlags: u8 {
            jump = 0,
            unmount = 1,
        }
    }

    def_struct! {
        Pong 0x1d {
            id: i32,
        }
    }

    def_struct! {
        SetRecipeBookState 0x1e {
            book_id: RecipeBookId,
            book_open: bool,
            filter_active: bool,
        }
    }

    def_enum! {
        RecipeBookId: VarInt {
            Crafting = 0,
            Furnace = 1,
            BlastFurnace = 2,
            Smoker = 3,
        }
    }

    def_struct! {
        SetDisplayedRecipe 0x1f {
            recipe_id: Identifier,
        }
    }

    def_struct! {
        NameItem 0x20 {
            item_name: BoundedString<0, 50>,
        }
    }

    def_enum! {
        ResourcePackStatus 0x21: VarInt {
            SuccessfullyLoaded = 0,
            Declined = 1,
            FailedDownload = 2,
            Accepted = 3,
        }
    }

    def_enum! {
        AdvancementTab 0x22: VarInt {
            OpenedTab: Identifier = 0,
            ClosedScreen = 1,
        }
    }

    def_struct! {
        SelectTrade 0x23 {
            selected_slot: VarInt,
        }
    }

    def_struct! {
        SetBeaconEffect 0x24 {
            // TODO: potion ids?
            primary_effect: VarInt,
            secondary_effect: VarInt,
        }
    }

    def_struct! {
        HeldItemChangeServerbound 0x25 {
            slot: BoundedInt<i16, 0, 8>,
        }
    }

    def_struct! {
        UpdateCommandBlock 0x26 {
            location: BlockPos,
            command: String,
            mode: CommandBlockMode,
            flags: CommandBlockFlags,
        }
    }

    def_enum! {
        CommandBlockMode: VarInt {
            Sequence = 0,
            Auto = 1,
            Redstone = 2,
        }
    }

    def_bitfield! {
        CommandBlockFlags: i8 {
            track_output = 0,
            is_conditional = 1,
            automatic = 2,
        }
    }

    def_struct! {
        UpdateCommandBlockMinecart 0x27 {
            entity_id: VarInt,
            command: String,
            track_output: bool,
        }
    }

    def_struct! {
        CreativeInventoryAction 0x28 {
            slot: i16,
            // TODO: clicked_item: Slot,
        }
    }

    def_struct! {
        UpdateJigsawBlock 0x29 {
            location: BlockPos,
            name: Identifier,
            target: Identifier,
            pool: Identifier,
            final_state: String,
            joint_type: String,
        }
    }

    def_struct! {
        UpdateStructureBlock 0x2a {
            location: BlockPos,
            action: StructureBlockAction,
            mode: StructureBlockMode,
            name: String,
            offset_xyz: [BoundedInt<i8, -32, 32>; 3],
            size_xyz: [BoundedInt<i8, 0, 32>; 3],
            mirror: StructureBlockMirror,
            rotation: StructureBlockRotation,
            metadata: String,
            integrity: f32, // TODO: bounded float between 0 and 1.
            seed: VarLong,
            flags: StructureBlockFlags,
        }
    }

    def_enum! {
        StructureBlockAction: VarInt {
            UpdateData = 0,
            SaveStructure = 1,
            LoadStructure = 2,
            DetectSize = 3,
        }
    }

    def_enum! {
        StructureBlockMode: VarInt {
            Save = 0,
            Load = 1,
            Corner = 2,
            Data = 3,
        }
    }

    def_enum! {
        StructureBlockMirror: VarInt {
            None = 0,
            LeftRight = 1,
            FrontBack = 2,
        }
    }

    def_enum! {
        StructureBlockRotation: VarInt {
            None = 0,
            Clockwise90 = 1,
            Clockwise180 = 2,
            Counterclockwise90 = 3,
        }
    }

    def_bitfield! {
        StructureBlockFlags: i8 {
            ignore_entities = 0,
            show_air = 1,
            show_bounding_box = 2,
        }
    }

    def_struct! {
        UpdateSign 0x2b {
            location: BlockPos,
            lines: [BoundedString<0, 384>; 4],
        }
    }

    def_struct! {
        PlayerArmSwing 0x2c {
            hand: Hand,
        }
    }

    def_struct! {
        Spectate 0x2d {
            target: Uuid,
        }
    }

    def_struct! {
        PlayerBlockPlacement 0x2e {
            hand: Hand,
            location: BlockPos,
            face: BlockFace,
            cursor_pos: Vec3,
            head_inside_block: bool,
        }
    }

    def_struct! {
        UseItem 0x2f {
            hand: Hand,
        }
    }

    macro_rules! def_server_play_packet_enum {
        {
            $($packet:ident),* $(,)?
        } => {
            /// An enum of all serverbound play packets.
            #[derive(Clone, Debug)]
            pub enum ServerPlayPacket {
                $($packet($packet)),*
            }

            impl private::Sealed for ServerPlayPacket {}

            impl DecodePacket for ServerPlayPacket {
                fn decode_packet(r: &mut impl Read) -> anyhow::Result<ServerPlayPacket> {
                    let packet_id = VarInt::decode(r).context("failed to read play packet ID")?.0;
                    match packet_id {
                        $(
                            $packet::PACKET_ID => {
                                let pkt = $packet::decode(r)?;
                                Ok(ServerPlayPacket::$packet(pkt))
                            }
                        )*
                        id => bail!("unknown play packet ID {:#04x}", id)
                    }
                }
            }


            #[cfg(test)]
            #[test]
            fn test_server_play_packet_order() {
                let ids = [
                    $(
                        (stringify!($packet), $packet::PACKET_ID),
                    )*
                ];

                if let Some(w) = ids.windows(2).find(|w| w[0].1 >= w[1].1) {
                    panic!("the {} and {} variants of the server play packet enum are not properly sorted by their packet ID", w[0].0, w[1].0);
                }
            }
        }
    }

    def_server_play_packet_enum! {
        TeleportConfirm,
        QueryBlockNbt,
        SetDifficulty,
        ChatMessageServerbound,
        ClientStatus,
        ClientSettings,
        TabCompleteServerbound,
        ClickWindowButton,
        ClickWindow,
        CloseWindow,
        PluginMessageServerbound,
        EditBook,
        QueryEntityNbt,
        InteractEntity,
        GenerateStructure,
        KeepAliveServerbound,
        LockDifficulty,
        PlayerPosition,
        PlayerPositionAndRotation,
        PlayerRotation,
        PlayerMovement,
        VehicleMoveServerbound,
        SteerBoat,
        PickItem,
        CraftRecipeRequest,
        PlayerAbilitiesServerbound,
        PlayerDigging,
        EntityAction,
        SteerVehicle,
        Pong,
        SetRecipeBookState,
        SetDisplayedRecipe,
        NameItem,
        ResourcePackStatus,
        AdvancementTab,
        SelectTrade,
        SetBeaconEffect,
        HeldItemChangeServerbound,
        UpdateCommandBlock,
        UpdateCommandBlockMinecart,
        CreativeInventoryAction,
        UpdateJigsawBlock,
        UpdateStructureBlock,
        UpdateSign,
        PlayerArmSwing,
        Spectate,
        PlayerBlockPlacement,
        UseItem,
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    def_struct! {
        TestPacket 0xfff {
            first: String,
            second: Vec<u16>,
            third: u64
        }
    }
}
