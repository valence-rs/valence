//! Contains packet definitions and some types contained within them.
//!
//! See <https://wiki.vg/Protocol> for up to date protocol information.

#![allow(dead_code)]

use std::fmt;
use std::io::{Read, Write};

use anyhow::{bail, ensure, Context};
use bitvec::prelude::BitVec;
use num::{One, Zero};
use paste::paste;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::Vec3;

use crate::block_pos::BlockPos;
use crate::protocol::{
    BoundedArray, BoundedInt, BoundedString, ByteAngle, Decode, Encode, Nbt, RawBytes, VarInt,
    VarLong,
};
use crate::{Ident, Text};

/// Trait for types that can be written to the Minecraft protocol as a complete
/// packet.
///
/// A complete packet is one that starts with a `VarInt` packet ID, followed by
/// the body of the packet.
pub trait EncodePacket: fmt::Debug {
    /// Writes a packet to the Minecraft protocol, including its packet ID.
    fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()>;
}

/// Trait for types that can be read from the Minecraft protocol as a complete
/// packet.
///
/// A complete packet is one that starts with a `VarInt` packet ID, followed by
/// the body of the packet.
pub trait DecodePacket: Sized + fmt::Debug {
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

def_struct! {
    #[derive(PartialEq, Serialize, Deserialize)]
    Property {
        name: String,
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>
    }
}

def_struct! {
    SignatureData {
        timestamp: u64,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    }
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
    pub mod s2c {
        use super::super::*;

        def_struct! {
            Response 0x00 {
                json_response: String
            }
        }

        def_struct! {
            PongResponse 0x01 {
                /// Should be the same as the payload from [`Ping`].
                payload: u64
            }
        }
    }

    pub mod c2s {
        use super::super::*;

        def_struct! {
            StatusRequest 0x00 {}
        }

        def_struct! {
            PingRequest 0x01 {
                payload: u64
            }
        }
    }
}

/// Packets and types used during the play state.
pub mod login {
    pub mod s2c {
        use super::super::*;

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
                properties: Vec<Property>,
            }
        }

        def_struct! {
            SetCompression 0x03 {
                threshold: VarInt
            }
        }
    }

    pub mod c2s {
        use super::super::*;

        def_struct! {
            LoginStart 0x00 {
                username: BoundedString<3, 16>,
                sig_data: Option<SignatureData>,
            }
        }

        def_struct! {
            EncryptionResponse 0x01 {
                shared_secret: BoundedArray<u8, 16, 128>,
                token_or_sig: VerifyTokenOrMsgSig,
            }
        }

        def_enum! {
            VerifyTokenOrMsgSig: u8 {
                VerifyToken: BoundedArray<u8, 16, 128> = 1,
                MsgSig: MessageSignature = 0,
            }
        }

        def_struct! {
            MessageSignature {
                salt: u64,
                sig: Vec<u8>, // TODO: bounds?
            }
        }
    }
}

/// Packets and types used during the play state.
pub mod play {
    pub mod s2c {
        use super::super::*;

        def_struct! {
            AddEntity 0x00 {
                entity_id: VarInt,
                object_uuid: Uuid,
                typ: VarInt,
                position: Vec3<f64>,
                pitch: ByteAngle,
                yaw: ByteAngle,
                head_yaw: ByteAngle,
                data: VarInt,
                velocity: Vec3<i16>,
            }
        }

        def_struct! {
            AddExperienceOrb 0x01 {
                entity_id: VarInt,
                position: Vec3<f64>,
                count: i16,
            }
        }

        def_struct! {
            AddPlayer 0x02 {
                entity_id: VarInt,
                player_uuid: Uuid,
                position: Vec3<f64>,
                yaw: ByteAngle,
                pitch: ByteAngle,
            }
        }

        def_struct! {
            Animate 0x03 {
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
            BlockChangeAck 0x05 {
                sequence: VarInt,
            }
        }

        def_struct! {
            BlockDestruction 0x06 {
                entity_id: VarInt,
                location: BlockPos,
                destroy_stage: BoundedInt<u8, 0, 10>,
            }
        }

        def_struct! {
            BlockEntityData 0x07 {
                location: BlockPos,
                typ: VarInt, // TODO: use enum here
                data: nbt::Blob,
            }
        }

        def_struct! {
            BlockEvent 0x08 {
                location: BlockPos,
                action_id: u8,
                action_param: u8,
                block_type: VarInt, // TODO: use BlockType type.
            }
        }

        def_struct! {
            BlockUpdate 0x09 {
                location: BlockPos,
                block_id: VarInt,
            }
        }

        def_struct! {
            BossEvent 0x0a {
                uuid: Uuid,
                action: BossEventAction,
            }
        }

        def_enum! {
            BossEventAction: VarInt {
                Add: BossEventActionAdd = 0,
                // TODO
            }
        }

        def_struct! {
            BossEventActionAdd {
                title: Text,
                health: f32,
                color: BossEventColor,
                division: BossEventDivision,
                /// TODO: bitmask
                flags: u8,
            }
        }

        def_enum! {
            BossEventColor: VarInt {
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
            BossEventDivision: VarInt {
                NoDivision = 0,
                SixNotches = 1,
                TenNotches = 2,
                TwelveNotches = 3,
                TwentyNotches = 4,
            }
        }

        def_struct! {
            ChangeDifficulty 0x0b {
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
            Disconnect 0x17 {
                reason: Text,
            }
        }

        def_struct! {
            EntityEvent 0x18 {
                entity_id: i32,
                /// TODO: enum
                entity_status: u8,
            }
        }

        def_struct! {
            ForgetLevelChunk 0x1a {
                chunk_x: i32,
                chunk_z: i32
            }
        }

        def_struct! {
            GameEvent 0x1b {
                reason: GameEventReason,
                value: f32,
            }
        }

        def_enum! {
            GameEventReason: u8 {
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
            InitializeWorldBorder 0x1d {
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
            KeepAlive 0x1e {
                id: i64,
            }
        }

        def_struct! {
            LevelChunkWithLight 0x1f {
                chunk_x: i32,
                chunk_z: i32,
                heightmaps: Nbt<LevelChunkHeightmaps>,
                blocks_and_biomes: Vec<u8>,
                block_entities: Vec<LevelChunkBlockEntity>,
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
        pub struct LevelChunkHeightmaps {
            #[serde(rename = "MOTION_BLOCKING", serialize_with = "nbt::i64_array")]
            pub motion_blocking: Vec<i64>,
        }

        def_struct! {
            LevelChunkBlockEntity {
                packed_xz: i8,
                y: i16,
                typ: VarInt,
                data: nbt::Blob,
            }
        }

        def_struct! {
            Login 0x23 {
                /// Entity ID of the joining player
                entity_id: i32,
                is_hardcore: bool,
                gamemode: GameMode,
                previous_gamemode: GameMode,
                dimension_names: Vec<Ident>,
                registry_codec: Nbt<RegistryCodec>,
                /// The name of the dimension type being spawned into.
                dimension_type_name: Ident,
                /// The name of the dimension being spawned into.
                dimension_name: Ident,
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
                last_death_location: Option<(Ident, BlockPos)>,
            }
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct RegistryCodec {
            #[serde(rename = "minecraft:dimension_type")]
            pub dimension_type_registry: DimensionTypeRegistry,
            #[serde(rename = "minecraft:worldgen/biome")]
            pub biome_registry: BiomeRegistry,
            #[serde(rename = "minecraft:chat_type")]
            pub chat_type_registry: ChatTypeRegistry,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct DimensionTypeRegistry {
            #[serde(rename = "type")]
            pub typ: Ident,
            pub value: Vec<DimensionTypeRegistryEntry>,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct DimensionTypeRegistryEntry {
            pub name: Ident,
            pub id: i32,
            pub element: DimensionType,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct DimensionType {
            pub piglin_safe: bool,
            pub has_raids: bool,
            pub monster_spawn_light_level: i32,
            pub monster_spawn_block_light_limit: i32,
            pub natural: bool,
            pub ambient_light: f32,
            pub fixed_time: Option<i64>,
            pub infiniburn: String, // TODO: tag type?
            pub respawn_anchor_works: bool,
            pub has_skylight: bool,
            pub bed_works: bool,
            pub effects: Ident,
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
            pub typ: Ident,
            pub value: Vec<Biome>,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct Biome {
            pub name: Ident,
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
            pub ambient_sound: Option<Ident>,
            pub additions_sound: Option<BiomeAdditionsSound>,
            pub mood_sound: Option<BiomeMoodSound>,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct BiomeMusic {
            pub replace_current_music: bool,
            pub sound: Ident,
            pub max_delay: i32,
            pub min_delay: i32,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct BiomeAdditionsSound {
            pub sound: Ident,
            pub tick_chance: f64,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct BiomeMoodSound {
            pub sound: Ident,
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
            pub typ: Ident,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct ChatTypeRegistry {
            #[serde(rename = "type")]
            pub typ: Ident,
            pub value: Vec<ChatTypeRegistryEntry>,
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct ChatTypeRegistryEntry {
            // TODO
        }

        def_enum! {
            #[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
            GameMode: u8 {
                #[default]
                Survival = 0,
                Creative = 1,
                Adventure = 2,
                Spectator = 3,
            }
        }

        def_struct! {
            MoveEntityPosition 0x26 {
                entity_id: VarInt,
                delta: Vec3<i16>,
                on_ground: bool,
            }
        }

        def_struct! {
            MoveEntityPositionAndRotation 0x27 {
                entity_id: VarInt,
                delta: Vec3<i16>,
                yaw: ByteAngle,
                pitch: ByteAngle,
                on_ground: bool,
            }
        }

        def_struct! {
            MoveEntityRotation 0x28 {
                entity_id: VarInt,
                yaw: ByteAngle,
                pitch: ByteAngle,
                on_ground: bool,
            }
        }

        def_struct! {
            PlayerChat 0x30 {
                message: Text,
                typ: PlayerChatType,
                sender: Uuid,
                // TODO more fields
            }
        }

        def_enum! {
            #[derive(Copy, PartialEq, Eq, Default)]
            PlayerChatType: VarInt {
                #[default]
                Chat = 0,
                SystemMessage = 1,
                GameInfo = 2,
                SayCommand = 3,
                MsgCommand = 4,
                TeamMsgCommand = 5,
                EmoteCommand = 6,
                TellrawCommand = 7,
            }
        }

        def_enum! {
            PlayerInfo 0x34: VarInt {
                AddPlayer: Vec<PlayerInfoAddPlayer> = 0,
                UpdateGameMode: Vec<(Uuid, GameMode)> = 1,
                UpdateLatency: Vec<(Uuid, VarInt)> = 2,
                UpdateDisplayName: Vec<(Uuid, Option<Text>)> = 3,
                RemovePlayer: Vec<Uuid> = 4,
            }
        }

        def_struct! {
            PlayerInfoAddPlayer {
                uuid: Uuid,
                username: BoundedString<3, 16>,
                properties: Vec<Property>,
                game_mode: GameMode,
                ping: VarInt,
                display_name: Option<Text>,
                sig_data: Option<SignatureData>,
            }
        }

        def_struct! {
            PlayerPosition 0x36 {
                position: Vec3<f64>,
                yaw: f32,
                pitch: f32,
                flags: PlayerPositionFlags,
                teleport_id: VarInt,
                dismount_vehicle: bool,
            }
        }

        def_bitfield! {
            PlayerPositionFlags: u8 {
                x = 0,
                y = 1,
                z = 2,
                y_rot = 3,
                x_rot = 4,
            }
        }

        def_struct! {
            RemoveEntities 0x38 {
                entities: Vec<VarInt>,
            }
        }

        def_struct! {
            RotateHead 0x3c {
                entity_id: VarInt,
                head_yaw: ByteAngle,
            }
        }

        def_struct! {
            SectionBlocksUpdate 0x3d {
                chunk_section_position: i64,
                invert_trust_edges: bool,
                blocks: Vec<VarLong>,
            }
        }

        def_struct! {
            SetCarriedItem 0x47 {
                slot: BoundedInt<u8, 0, 9>,
            }
        }

        def_struct! {
            SetChunkCacheCenter 0x48 {
                chunk_x: VarInt,
                chunk_z: VarInt,
            }
        }

        def_struct! {
            SetChunkCacheRadius 0x49 {
                view_distance: BoundedInt<VarInt, 2, 32>,
            }
        }

        def_struct! {
            SpawnPosition 0x4a {
                location: BlockPos,
                angle: f32,
            }
        }

        def_struct! {
            SetEntityMetadata 0x4d {
                entity_id: VarInt,
                metadata: RawBytes,
            }
        }

        def_struct! {
            SetEntityMotion 0x4f {
                entity_id: VarInt,
                velocity: Vec3<i16>,
            }
        }

        def_struct! {
            SetTime 0x59 {
                /// The age of the world in 1/20ths of a second.
                world_age: i64,
                /// The current time of day in 1/20ths of a second.
                /// The value should be in the range \[0, 24000].
                /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
                time_of_day: i64,
            }
        }

        def_struct! {
            SystemChat 0x5f {
                chat: Text,
                typ: PlayerChatType,
            }
        }

        def_struct! {
            TabList 0x60 {
                header: Text,
                footer: Text,
            }
        }

        def_struct! {
            TeleportEntity 0x63 {
                entity_id: VarInt,
                position: Vec3<f64>,
                yaw: ByteAngle,
                pitch: ByteAngle,
                on_ground: bool,
            }
        }

        macro_rules! def_s2c_play_packet_enum {
            {
                $($packet:ident),* $(,)?
            } => {
                /// An enum of all s2c play packets.
                #[derive(Clone, Debug)]
                pub enum S2cPlayPacket {
                    $($packet($packet)),*
                }

                $(
                    impl From<$packet> for S2cPlayPacket {
                        fn from(p: $packet) -> S2cPlayPacket {
                            S2cPlayPacket::$packet(p)
                        }
                    }
                )*

                impl EncodePacket for S2cPlayPacket {
                    fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
                        match self {
                            $(
                                Self::$packet(p) => {
                                    VarInt($packet::PACKET_ID)
                                        .encode(w)
                                        .context(concat!("failed to write s2c play packet ID for `", stringify!($packet), "`"))?;
                                    p.encode(w)
                                }
                            )*
                        }
                    }
                }

                #[cfg(test)]
                #[test]
                fn s2c_play_packet_order() {
                    let ids = [
                        $(
                            (stringify!($packet), $packet::PACKET_ID),
                        )*
                    ];

                    if let Some(w) = ids.windows(2).find(|w| w[0].1 >= w[1].1) {
                        panic!(
                            "the {} (ID {:#x}) and {} (ID {:#x}) variants of the s2c play packet enum are not properly sorted by their packet ID",
                            w[0].0,
                            w[0].1,
                            w[1].0,
                            w[1].1
                        );
                    }
                }
            }
        }

        def_s2c_play_packet_enum! {
            AddEntity,
            AddExperienceOrb,
            AddPlayer,
            Animate,
            BlockChangeAck,
            BlockDestruction,
            BlockEntityData,
            BlockEvent,
            BlockUpdate,
            BossEvent,
            Disconnect,
            EntityEvent,
            ForgetLevelChunk,
            GameEvent,
            KeepAlive,
            LevelChunkWithLight,
            Login,
            MoveEntityPosition,
            MoveEntityPositionAndRotation,
            MoveEntityRotation,
            PlayerChat,
            PlayerInfo,
            PlayerPosition,
            RemoveEntities,
            RotateHead,
            SectionBlocksUpdate,
            SetCarriedItem,
            SetChunkCacheCenter,
            SetChunkCacheRadius,
            SpawnPosition,
            SetEntityMetadata,
            SetEntityMotion,
            SetTime,
            SystemChat,
            TabList,
            TeleportEntity,
        }
    }

    pub mod c2s {
        use super::super::*;

        def_struct! {
            AcceptTeleportation 0x00 {
                teleport_id: VarInt
            }
        }

        def_struct! {
            BlockEntityTagQuery 0x01 {
                transaction_id: VarInt,
                location: BlockPos,
            }
        }

        def_enum! {
            ChangeDifficulty 0x02: i8 {
                Peaceful = 0,
                Easy = 1,
                Normal = 2,
                Hard = 3,
            }
        }

        def_struct! {
            ChatCommand 0x03 {
                command: String, // TODO: bounded?
                // TODO: timestamp, arg signatures
                signed_preview: bool,
            }
        }

        def_struct! {
            Chat 0x04 {
                message: BoundedString<0, 256>,
                timestamp: u64,
                salt: u64,
                signature: Vec<u8>,
                signed_preview: bool,
            }
        }

        def_struct! {
            ChatPreview 0x05 {
                query: i32, // TODO: is this an i32 or a varint?
                message: BoundedString<0, 256>,
            }
        }

        def_enum! {
            ClientCommand 0x06: VarInt {
                /// Sent when ready to complete login and ready to respawn after death.
                PerformRespawn = 0,
                /// Sent when the statistics menu is opened.
                RequestStatus = 1,
            }
        }

        def_struct! {
            ClientInformation 0x07 {
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
            CommandSuggestion 0x08 {
                transaction_id: VarInt,
                /// Text behind the cursor without the '/'.
                text: BoundedString<0, 32500>
            }
        }

        def_struct! {
            ContainerButtonClick 0x09 {
                window_id: i8,
                button_id: i8,
            }
        }

        def_struct! {
            ContainerClose 0x0b {
                window_id: u8,
            }
        }

        def_struct! {
            CustomPayload 0x0c {
                channel: Ident,
                data: RawBytes,
            }
        }

        def_struct! {
            EditBook 0x0d {
                slot: VarInt,
                entries: Vec<String>,
                title: Option<String>,
            }
        }

        def_struct! {
            EntityTagQuery 0x0e {
                transaction_id: VarInt,
                entity_id: VarInt,
            }
        }

        def_struct! {
            Interact 0x0f {
                entity_id: VarInt,
                typ: InteractType,
                sneaking: bool,
            }
        }

        def_enum! {
            InteractType: VarInt {
                Interact: Hand = 0,
                Attack = 1,
                InteractAt: (Vec3<f32>, Hand) = 2
            }
        }

        def_enum! {
            #[derive(Copy, PartialEq, Eq)]
            Hand: VarInt {
                Main = 0,
                Off = 1,
            }
        }

        def_struct! {
            JigsawGenerate 0x10 {
                location: BlockPos,
                levels: VarInt,
                keep_jigsaws: bool,
            }
        }

        def_struct! {
            KeepAlive 0x11 {
                id: i64,
            }
        }

        def_struct! {
            LockDifficulty 0x12 {
                locked: bool
            }
        }

        def_struct! {
            MovePlayerPosition 0x13 {
                position: Vec3<f64>,
                on_ground: bool,
            }
        }

        def_struct! {
            MovePlayerPositionAndRotation 0x14 {
                // Absolute position
                position: Vec3<f64>,
                /// Absolute rotation on X axis in degrees.
                yaw: f32,
                /// Absolute rotation on Y axis in degrees.
                pitch: f32,
                on_ground: bool,
            }
        }

        def_struct! {
            MovePlayerRotation 0x15 {
                /// Absolute rotation on X axis in degrees.
                yaw: f32,
                /// Absolute rotation on Y axis in degrees.
                pitch: f32,
                on_ground: bool,
            }
        }

        def_struct! {
            MovePlayerStatusOnly 0x16 {
                on_ground: bool
            }
        }

        def_struct! {
            MoveVehicle 0x17 {
                /// Absolute position
                position: Vec3<f64>,
                /// Degrees
                yaw: f32,
                /// Degrees
                pitch: f32,
            }
        }

        def_struct! {
            PaddleBoat 0x18 {
                left_paddle_turning: bool,
                right_paddle_turning: bool,
            }
        }

        def_struct! {
            PickItem 0x19 {
                slot_to_use: VarInt,
            }
        }

        def_struct! {
            PlaceRecipe 0x1a {
                window_id: i8,
                recipe: Ident,
                make_all: bool,
            }
        }

        def_enum! {
            PlayerAbilities 0x1b: i8 {
                NotFlying = 0,
                Flying = 0b10,
            }
        }

        def_struct! {
            PlayerAction 0x1c {
                status: DiggingStatus,
                location: BlockPos,
                face: BlockFace,
                sequence: VarInt,
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
            #[derive(Copy, PartialEq, Eq)]
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
            PlayerCommand 0x1d {
                entity_id: VarInt,
                action_id: PlayerCommandId,
                jump_boost: BoundedInt<VarInt, 0, 100>,
            }
        }

        def_enum! {
            PlayerCommandId: VarInt {
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
            PlayerInput 0x1e {
                sideways: f32,
                forward: f32,
                flags: PlayerInputFlags,
            }
        }

        def_bitfield! {
            PlayerInputFlags: u8 {
                jump = 0,
                unmount = 1,
            }
        }

        def_struct! {
            Pong 0x1f {
                id: i32,
            }
        }

        def_struct! {
            RecipeBookChangeSettings 0x20 {
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
            RecipeBookSeenRecipe 0x21 {
                recipe_id: Ident,
            }
        }

        def_struct! {
            RenameItem 0x22 {
                item_name: BoundedString<0, 50>,
            }
        }

        def_enum! {
            ResourcePack 0x23: VarInt {
                SuccessfullyLoaded = 0,
                Declined = 1,
                FailedDownload = 2,
                Accepted = 3,
            }
        }

        def_enum! {
            SeenAdvancements 0x24: VarInt {
                OpenedTab: Ident = 0,
                ClosedScreen = 1,
            }
        }

        def_struct! {
            SelectTrade 0x25 {
                selected_slot: VarInt,
            }
        }

        def_struct! {
            SetBeacon 0x26 {
                // TODO: potion ids
                primary_effect: Option<VarInt>,
                secondary_effect: Option<VarInt>,
            }
        }

        def_struct! {
            SetCarriedItem 0x27 {
                slot: BoundedInt<i16, 0, 8>,
            }
        }

        def_struct! {
            SetCommandBlock 0x28 {
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
            SetCommandBlockMinecart 0x29 {
                entity_id: VarInt,
                command: String,
                track_output: bool,
            }
        }

        def_struct! {
            SetCreativeModeSlot 0x2a {
                slot: i16,
                // TODO: clicked_item: Slot,
            }
        }

        def_struct! {
            SetJigsawBlock 0x2b {
                location: BlockPos,
                name: Ident,
                target: Ident,
                pool: Ident,
                final_state: String,
                joint_type: String,
            }
        }

        def_struct! {
            SetStructureBlock 0x2c {
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
            SignUpdate 0x2d {
                location: BlockPos,
                lines: [BoundedString<0, 384>; 4],
            }
        }

        def_struct! {
            Swing 0x2e {
                hand: Hand,
            }
        }

        def_struct! {
            TeleportToEntity 0x2f {
                target: Uuid,
            }
        }

        def_struct! {
            UseItemOn 0x30 {
                hand: Hand,
                location: BlockPos,
                face: BlockFace,
                cursor_pos: Vec3<f32>,
                head_inside_block: bool,
                sequence: VarInt,
            }
        }

        def_struct! {
            UseItem 0x31 {
                hand: Hand,
                sequence: VarInt,
            }
        }

        macro_rules! def_c2s_play_packet_enum {
        {
            $($packet:ident),* $(,)?
        } => {
            /// An enum of all client-to-server play packets.
            #[derive(Clone, Debug)]
            pub enum C2sPlayPacket {
                $($packet($packet)),*
            }

            impl DecodePacket for C2sPlayPacket {
                fn decode_packet(r: &mut impl Read) -> anyhow::Result<C2sPlayPacket> {
                    let packet_id = VarInt::decode(r).context("failed to read c2s play packet ID")?.0;
                    match packet_id {
                        $(
                            $packet::PACKET_ID => {
                                let pkt = $packet::decode(r)?;
                                Ok(C2sPlayPacket::$packet(pkt))
                            }
                        )*
                        id => bail!("unknown c2s play packet ID {:#04x}", id)
                    }
                }
            }


            #[cfg(test)]
            #[test]
            fn c2s_play_packet_order() {
                let ids = [
                    $(
                        (stringify!($packet), $packet::PACKET_ID),
                    )*
                ];

                if let Some(w) = ids.windows(2).find(|w| w[0].1 >= w[1].1) {
                    panic!(
                        "the {} (ID {:#x}) and {} (ID {:#x}) variants of the c2s play packet enum are not properly sorted by their packet ID",
                        w[0].0,
                        w[0].1,
                        w[1].0,
                        w[1].1
                    );
                }
            }
        }
    }

        def_c2s_play_packet_enum! {
            AcceptTeleportation,
            BlockEntityTagQuery,
            ChangeDifficulty,
            ChatCommand,
            Chat,
            ChatPreview,
            ClientCommand,
            ClientInformation,
            CommandSuggestion,
            ContainerButtonClick,
            ContainerClose,
            CustomPayload,
            EditBook,
            EntityTagQuery,
            Interact,
            JigsawGenerate,
            KeepAlive,
            LockDifficulty,
            MovePlayerPosition,
            MovePlayerPositionAndRotation,
            MovePlayerRotation,
            MovePlayerStatusOnly,
            MoveVehicle,
            PaddleBoat,
            PickItem,
            PlaceRecipe,
            PlayerAbilities,
            PlayerAction,
            PlayerCommand,
            PlayerInput,
            Pong,
            RecipeBookChangeSettings,
            RecipeBookSeenRecipe,
            RenameItem,
            ResourcePack,
            SeenAdvancements,
            SelectTrade,
            SetBeacon,
            SetCarriedItem,
            SetCommandBlock,
            SetCommandBlockMinecart,
            SetCreativeModeSlot,
            SetJigsawBlock,
            SetStructureBlock,
            SignUpdate,
            Swing,
            TeleportToEntity,
            UseItemOn,
            UseItem,
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    def_struct! {
        TestPacket 0xfff {
            first: String,
            second: Vec<u16>,
            third: u64
        }
    }
}
