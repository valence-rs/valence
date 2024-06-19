use std::borrow::Cow;
use std::hint::black_box;

use divan::Bencher;
use valence::nbt::{compound, List};
use valence::prelude::*;
use valence::protocol::decode::PacketDecoder;
use valence::protocol::encode::{PacketEncoder, PacketWriter, WritePacket};
use valence::protocol::packets::play::{ChunkDataS2c, EntitySpawnS2c, PlayerListHeaderS2c};
use valence::protocol::{ByteAngle, FixedArray, VarInt};
use valence::text::IntoText;
use valence_server::protocol::Velocity;
use valence_server::CompressionThreshold;

pub fn setup<'a>() -> (PacketEncoder, ChunkDataS2c<'a>, PlayerListHeaderS2c<'a>, EntitySpawnS2c) {
    let encoder = PacketEncoder::new();

    const BLOCKS_AND_BIOMES: [u8; 2000] = [0x80; 2000];
    const SKY_LIGHT_ARRAYS: [FixedArray<u8, 2048>; 26] = [FixedArray([0xff; 2048]); 26];

    let chunk_data_packet = ChunkDataS2c {
        pos: ChunkPos::new(123, 456),
        heightmaps: Cow::Owned(compound! {
            "MOTION_BLOCKING" => List::Long(vec![123; 256]),
        }),
        blocks_and_biomes: BLOCKS_AND_BIOMES.as_slice(),
        block_entities: Cow::Borrowed(&[]),
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
        velocity: Velocity([12, 34, 56]),
    };

    (encoder, chunk_data_packet, player_list_header_packet, spawn_entity_packet)
}
#[divan::bench]
fn encode_chunk_data(bencher: Bencher) {
    let (mut encoder, chunk_data_packet, _, _) = setup();
    bencher.bench_local(|| { let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&chunk_data_packet).unwrap();

            black_box(encoder);
    });
}

#[divan::bench]
fn encode_player_list_header(bencher: Bencher) {
    let (mut encoder, _, player_list_header_packet, _) = setup();
    bencher.bench_local(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&player_list_header_packet).unwrap();

            black_box(encoder);
    });
}

#[divan::bench]
fn encode_spawn_entity(bencher: Bencher) {
    let (mut encoder, _, _, spawn_entity_packet) = setup();
    bencher.bench_local(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&spawn_entity_packet).unwrap();

            black_box(encoder);
    });
}

#[divan::bench]
fn encode_chunk_data_compressed(bencher: Bencher) {
    let (mut encoder, chunk_data_packet, _, _) = setup();
    encoder.set_compression(CompressionThreshold(-1));

    bencher.bench_local(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&chunk_data_packet).unwrap();

            black_box(encoder);
    });
}

#[divan::bench]
fn encode_player_list_header_compressed(bencher: Bencher) {
    let (mut encoder, _, player_list_header_packet, _) = setup();
    encoder.set_compression(CompressionThreshold(-1));

    bencher.bench_local(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&player_list_header_packet).unwrap();

            black_box(encoder);
    });
    }

#[divan::bench]
fn encode_spawn_entity_compressed(bencher: Bencher) {
    let (mut encoder, _, _, spawn_entity_packet) = setup();
    encoder.set_compression(CompressionThreshold(-1));

    bencher.bench_local(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&spawn_entity_packet).unwrap();

            black_box(encoder);
        });
    }

#[divan::bench]
fn decode_chunk_data(bencher: Bencher) {
    let (_, chunk_data_packet, _, _) = setup();

    let mut decoder = PacketDecoder::new();
    let mut packet_buf = vec![];

    PacketWriter::new(&mut packet_buf, CompressionThreshold(-1)).write_packet(&chunk_data_packet);
    bencher.bench_local(|| {
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
}

#[divan::bench]
fn decode_player_list_header(bencher: Bencher) {
    let (_, _, player_list_header_packet, _) = setup();

    let mut decoder = PacketDecoder::new();
    let mut packet_buf = vec![];

    PacketWriter::new(&mut packet_buf, CompressionThreshold(-1)).write_packet(&player_list_header_packet);
    bencher.bench_local(move || {
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
}

#[divan::bench]
fn decode_entity_spawn(bencher: Bencher) {
    let (_, _, _, spawn_entity_packet) = setup();

    let mut decoder = PacketDecoder::new();
    let mut packet_buf = vec![];

    PacketWriter::new(&mut packet_buf, CompressionThreshold(-1)).write_packet(&spawn_entity_packet);
    bencher.bench_local(|| {
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
}

#[divan::bench]
fn decode_chunk_data_compressed(bencher: Bencher) {
    let (_, chunk_data_packet, _, _) = setup();

    let mut decoder = PacketDecoder::new();
    let mut packet_buf = vec![];

    decoder.set_compression(256.into());

    PacketWriter::new(&mut packet_buf, 256.into()).write_packet(&chunk_data_packet);

    bencher.bench_local(|| {
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
}

#[divan::bench]
fn decode_player_list_header_compressed(bencher: Bencher) {
    let (_, _, player_list_header_packet, _) = setup();

    let mut decoder = PacketDecoder::new();
    let mut packet_buf = vec![];

    decoder.set_compression(256.into());

    PacketWriter::new(&mut packet_buf, 256.into()).write_packet(&player_list_header_packet);

    bencher.bench_local(|| {
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
}

#[divan::bench]
fn decode_spawn_data_compressed(bencher: Bencher) {
    let (_, _, _, spawn_entity_packet) = setup();

    let mut decoder = PacketDecoder::new();
    let mut packet_buf = vec![];

    decoder.set_compression(256.into());
    PacketWriter::new(&mut packet_buf, 256.into()).write_packet(&spawn_entity_packet);

    bencher.bench_local(|| {
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
}
