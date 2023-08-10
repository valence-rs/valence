use std::hint::black_box;

use criterion::Criterion;
use valence::block::{BlockKind, BlockState, PropName, PropValue};
use valence::ItemKind;

pub fn block(c: &mut Criterion) {
    let mut group = c.benchmark_group("block");

    let states = BlockKind::ALL.map(BlockKind::to_state);

    group.bench_function("BlockState::from_kind", |b| {
        b.iter(|| {
            for kind in black_box(BlockKind::ALL) {
                black_box(BlockState::from_kind(kind));
            }
        });
    });

    group.bench_function("BlockState::to_kind", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.to_kind());
            }
        });
    });

    group.bench_function("BlockState::get", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.get(PropName::Note));
            }
        });
    });

    group.bench_function("BlockState::set", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.set(PropName::Note, PropValue::Didgeridoo));
            }
        });
    });

    group.bench_function("BlockState::is_liquid", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.is_liquid());
            }
        });
    });

    group.bench_function("BlockState::is_opaque", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.is_opaque());
            }
        })
    });

    group.bench_function("BlockState::is_replaceable", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.is_replaceable());
            }
        })
    });

    group.bench_function("BlockState::luminance", |b| {
        b.iter(|| {
            for state in black_box(states) {
                black_box(state.luminance());
            }
        })
    });

    group.bench_function("BlockKind::to_item_kind", |b| {
        b.iter(|| {
            for kind in black_box(BlockKind::ALL) {
                black_box(kind.to_item_kind());
            }
        });
    });

    group.bench_function("BlockKind::from_item_kind", |b| {
        b.iter(|| {
            for kind in black_box(ItemKind::ALL) {
                black_box(BlockKind::from_item_kind(kind));
            }
        });
    });
}
