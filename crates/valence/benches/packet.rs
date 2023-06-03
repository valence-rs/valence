use std::borrow::Cow;
use std::hint::black_box;

use criterion::Criterion;
use valence::nbt::{compound, List};
use valence::prelude::*;
use valence::protocol::array::LengthPrefixedArray;
use valence::protocol::byte_angle::ByteAngle;
use valence::protocol::decode::PacketDecoder;
use valence::protocol::encode::{encode_packet, encode_packet_compressed, PacketEncoder};
use valence::protocol::var_int::VarInt;
use valence::text::TextFormat;
use valence_entity::packet::EntitySpawnS2c;
use valence_instance::packet::ChunkDataS2c;
use valence_player_list::packet::PlayerListHeaderS2c;

pub fn packet(c: &mut Criterion) {
    let mut encoder = PacketEncoder::new();

    const BLOCKS_AND_BIOMES: [u8; 2000] = [0x80; 2000];
    const SKY_LIGHT_ARRAYS: [LengthPrefixedArray<u8, 2048>; 26] =
        [LengthPrefixedArray([0xff; 2048]); 26];

    let chunk_data_packet = ChunkDataS2c {
        pos: ChunkPos::new(123, 456),
        heightmaps: Cow::Owned(compound! {
            "MOTION_BLOCKING" => List::Long(vec![123; 256]),
        }),
        blocks_and_biomes: BLOCKS_AND_BIOMES.as_slice(),
        block_entities: Cow::Borrowed(&[]),
        trust_edges: false,
        sky_light_mask: Cow::Borrowed(&[]),
        block_light_mask: Cow::Borrowed(&[]),
        empty_sky_light_mask: Cow::Borrowed(&[]),
        empty_block_light_mask: Cow::Borrowed(&[]),
        sky_light_arrays: Cow::Borrowed(SKY_LIGHT_ARRAYS.as_slice()),
        block_light_arrays: Cow::Borrowed(&[]),
    };

    let player_list_header_packet = PlayerListHeaderS2c {
        header: ("this".italic() + " is the " + "header".bold().color(Color::RED)).into(),
        footer: ("this".italic()
            + " is the "
            + "footer".bold().color(Color::BLUE)
            + ". I am appending some extra text so that the packet goes over the compression \
               threshold.")
            .into(),
    };

    let spawn_entity_packet = EntitySpawnS2c {
        entity_id: VarInt(1234),
        object_uuid: Default::default(),
        kind: VarInt(5),
        position: DVec3::new(123.0, 456.0, 789.0),
        pitch: ByteAngle(200),
        yaw: ByteAngle(100),
        head_yaw: ByteAngle(50),
        data: VarInt(i32::MIN),
        velocity: [12, 34, 56],
    };

    c.bench_function("encode_chunk_data", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&chunk_data_packet).unwrap();

            black_box(encoder);
        });
    });

    c.bench_function("encode_player_list_header", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&player_list_header_packet).unwrap();

            black_box(encoder);
        });
    });

    c.bench_function("encode_spawn_entity", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&spawn_entity_packet).unwrap();

            black_box(encoder);
        });
    });

    encoder.set_compression(Some(256));

    c.bench_function("encode_chunk_data_compressed", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&chunk_data_packet).unwrap();

            black_box(encoder);
        });
    });

    c.bench_function("encode_player_list_header_compressed", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&player_list_header_packet).unwrap();

            black_box(encoder);
        });
    });

    c.bench_function("encode_spawn_entity_compressed", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&spawn_entity_packet).unwrap();

            black_box(encoder);
        });
    });

    let mut decoder = PacketDecoder::new();
    let mut packet_buf = vec![];

    encode_packet(&mut packet_buf, &chunk_data_packet).unwrap();

    c.bench_function("decode_chunk_data", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&packet_buf);
            decoder
                .try_next_packet()
                .unwrap()
                .unwrap()
                .decode::<ChunkDataS2c>()
                .unwrap();

            black_box(decoder);
        });
    });

    packet_buf.clear();
    encode_packet(&mut packet_buf, &player_list_header_packet).unwrap();

    c.bench_function("decode_player_list_header", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&packet_buf);
            decoder
                .try_next_packet()
                .unwrap()
                .unwrap()
                .decode::<PlayerListHeaderS2c>()
                .unwrap();

            black_box(decoder);
        });
    });

    packet_buf.clear();
    encode_packet(&mut packet_buf, &spawn_entity_packet).unwrap();

    c.bench_function("decode_entity_spawn", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&packet_buf);
            decoder
                .try_next_packet()
                .unwrap()
                .unwrap()
                .decode::<EntitySpawnS2c>()
                .unwrap();

            black_box(decoder);
        });
    });

    decoder.set_compression(Some(256));

    let mut scratch = vec![];

    packet_buf.clear();
    encode_packet_compressed(&mut packet_buf, &chunk_data_packet, 256, &mut scratch).unwrap();

    c.bench_function("decode_chunk_data_compressed", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&packet_buf);
            decoder
                .try_next_packet()
                .unwrap()
                .unwrap()
                .decode::<ChunkDataS2c>()
                .unwrap();

            black_box(decoder);
        });
    });

    packet_buf.clear();
    encode_packet_compressed(
        &mut packet_buf,
        &player_list_header_packet,
        256,
        &mut scratch,
    )
    .unwrap();

    c.bench_function("decode_player_list_header_compressed", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&packet_buf);
            decoder
                .try_next_packet()
                .unwrap()
                .unwrap()
                .decode::<PlayerListHeaderS2c>()
                .unwrap();

            black_box(decoder);
        });
    });

    packet_buf.clear();
    encode_packet_compressed(&mut packet_buf, &spawn_entity_packet, 256, &mut scratch).unwrap();

    c.bench_function("decode_spawn_entity_compressed", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&packet_buf);
            decoder
                .try_next_packet()
                .unwrap()
                .unwrap()
                .decode::<EntitySpawnS2c>()
                .unwrap();

            black_box(decoder);
        });
    });
}
