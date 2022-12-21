use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use valence_nbt::{compound, List};
use valence_protocol::block::{BlockKind, BlockState, PropName, PropValue};
use valence_protocol::packets::s2c::play::{
    ChunkDataAndUpdateLight, ChunkDataAndUpdateLightEncode, SetTabListHeaderAndFooter,
};
use valence_protocol::text::Color;
use valence_protocol::{
    write_packet, write_packet_compressed, ItemKind, LengthPrefixedArray, PacketDecoder,
    PacketEncoder, TextFormat,
};

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(5)).confidence_level(0.99);
    targets = blocks, packets
}
criterion_main!(benches);

fn blocks(c: &mut Criterion) {
    let states = BlockKind::ALL.map(BlockKind::to_state);

    c.bench_function("BlockState::from_kind", |b| {
        b.iter(|| {
            for kind in black_box(BlockKind::ALL) {
                black_box(BlockState::from_kind(kind));
            }
        });
    });

    c.bench_function("BlockState::to_kind", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.to_kind());
            }
        });
    });

    c.bench_function("BlockState::get", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.get(PropName::Note));
            }
        });
    });

    c.bench_function("BlockState::set", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.set(PropName::Note, PropValue::Didgeridoo));
            }
        });
    });

    c.bench_function("BlockState::is_liquid", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.is_liquid());
            }
        });
    });

    c.bench_function("BlockState::is_opaque", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.is_opaque());
            }
        })
    });

    c.bench_function("BlockState::is_replaceable", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.is_replaceable());
            }
        })
    });

    c.bench_function("BlockState::luminance", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.luminance());
            }
        })
    });

    c.bench_function("BlockKind::to_item_kind", |b| {
        b.iter(|| {
            for kind in black_box(BlockKind::ALL) {
                black_box(kind.to_item_kind());
            }
        });
    });

    c.bench_function("BlockKind::from_item_kind", |b| {
        b.iter(|| {
            for kind in black_box(ItemKind::ALL) {
                black_box(BlockKind::from_item_kind(kind));
            }
        });
    });
}

fn packets(c: &mut Criterion) {
    let mut encoder = PacketEncoder::new();

    const BLOCKS_AND_BIOMES: [u8; 2000] = [0x80; 2000];
    const SKY_LIGHT_ARRAYS: [LengthPrefixedArray<u8, 2048>; 26] =
        [LengthPrefixedArray([0xff; 2048]); 26];

    let chunk_data_packet = ChunkDataAndUpdateLightEncode {
        chunk_x: 123,
        chunk_z: 456,
        heightmaps: &compound! {
            "MOTION_BLOCKING" => List::Long(vec![123; 256]),
        },
        blocks_and_biomes: BLOCKS_AND_BIOMES.as_slice(),
        block_entities: &[],
        trust_edges: false,
        sky_light_mask: &[],
        block_light_mask: &[],
        empty_sky_light_mask: &[],
        empty_block_light_mask: &[],
        sky_light_arrays: SKY_LIGHT_ARRAYS.as_slice(),
        block_light_arrays: &[],
    };

    let tab_list_header_footer_packet = SetTabListHeaderAndFooter {
        header: "this".italic() + " is the " + "header".bold().color(Color::RED),
        footer: "this".italic()
            + " is the "
            + "footer".bold().color(Color::BLUE)
            + ". I am appending some extra text so that the packet goes over the compression \
               threshold.",
    };

    c.bench_function("encode_chunk_data", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder.append_packet(&chunk_data_packet).unwrap();

            black_box(encoder);
        });
    });

    c.bench_function("encode_tab_list_header_footer", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder
                .append_packet(&tab_list_header_footer_packet)
                .unwrap();

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

    c.bench_function("encode_tab_list_header_footer_compressed", |b| {
        b.iter(|| {
            let encoder = black_box(&mut encoder);

            encoder.clear();
            encoder
                .append_packet(&tab_list_header_footer_packet)
                .unwrap();

            black_box(encoder);
        });
    });

    let mut chunk_data = vec![];
    let mut tab_list_header_footer = vec![];

    write_packet(&mut chunk_data, &chunk_data_packet).unwrap();
    write_packet(&mut tab_list_header_footer, &tab_list_header_footer_packet).unwrap();

    let mut decoder = PacketDecoder::new();

    c.bench_function("decode_chunk_data", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&chunk_data);
            decoder
                .try_next_packet::<ChunkDataAndUpdateLight>()
                .unwrap();

            black_box(decoder);
        });
    });

    c.bench_function("decode_tab_list_header_footer", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&tab_list_header_footer);
            decoder
                .try_next_packet::<SetTabListHeaderAndFooter>()
                .unwrap();

            black_box(decoder);
        });
    });

    decoder.set_compression(true);

    let mut scratch = vec![];

    chunk_data.clear();
    write_packet_compressed(&mut chunk_data, 256, &mut scratch, &chunk_data_packet).unwrap();

    tab_list_header_footer.clear();
    write_packet_compressed(
        &mut tab_list_header_footer,
        256,
        &mut scratch,
        &tab_list_header_footer_packet,
    )
    .unwrap();

    c.bench_function("decode_chunk_data_compressed", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&chunk_data);
            decoder
                .try_next_packet::<ChunkDataAndUpdateLight>()
                .unwrap();

            black_box(decoder);
        });
    });

    c.bench_function("decode_tab_list_header_footer_compressed", |b| {
        b.iter(|| {
            let decoder = black_box(&mut decoder);

            decoder.queue_slice(&tab_list_header_footer);
            decoder
                .try_next_packet::<SetTabListHeaderAndFooter>()
                .unwrap();

            black_box(decoder);
        });
    });
}
