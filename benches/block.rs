use std::hint::black_box;

use valence::block::{BlockKind, BlockState, PropName, PropValue};
use valence::ItemKind;
use divan::Bencher;

#[divan::bench]
pub fn from_kind(bencher: Bencher) {
    bencher.bench(|| {
            for kind in black_box(BlockKind::ALL) {
                black_box(BlockState::from_kind(kind));
            }
    });

}
#[divan::bench]
pub fn to_kind(bencher: Bencher) {

    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
            for state in black_box(states) {
                black_box(state.to_kind());
            }
    });

}
#[divan::bench]
pub fn get_prop(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
            for state in black_box(states) {
                black_box(state.get(PropName::Note));
            }
    });

}
#[divan::bench]
pub fn set_prop(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
            for state in black_box(states) {
                black_box(state.set(PropName::Note, PropValue::Didgeridoo));
            }
    });

}
#[divan::bench]
pub fn is_liquid(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
            for state in black_box(states) {
                black_box(state.is_liquid());
            }
    });

}
#[divan::bench]
pub fn is_opaque(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
            for state in black_box(states) {
                black_box(state.is_opaque());
            }
    });

}
#[divan::bench]
pub fn is_replaceable(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
            for state in black_box(states) {
                black_box(state.is_replaceable());
            }
    });

}
#[divan::bench]
pub fn luminance(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
            for state in black_box(states) {
                black_box(state.luminance());
            }
    });

}
#[divan::bench]
pub fn to_item_kind(bencher: Bencher) {
    bencher.bench(|| {
            for kind in black_box(BlockKind::ALL) {
                black_box(kind.to_item_kind());
            }
    });

}
#[divan::bench]
pub fn from_item_kind(bencher: Bencher) {
    bencher.bench(|| {
            for kind in black_box(ItemKind::ALL) {
                black_box(BlockKind::from_item_kind(kind));
            }
    });
}
