use std::hint::black_box;

use divan::Bencher;
use valence::block::{BlockKind, BlockState, PropName, PropValue};
use valence::ItemKind;

#[divan::bench]
pub(crate) fn from_kind(bencher: Bencher) {
    bencher.bench(|| {
        for kind in black_box(BlockKind::ALL) {
            black_box(BlockState::from_kind(kind));
        }
    });
}
#[divan::bench]
pub(crate) fn to_kind(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
        for state in black_box(states) {
            black_box(state.to_kind());
        }
    });
}
#[divan::bench]
pub(crate) fn get_prop(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
        for state in black_box(states) {
            black_box(state.get(PropName::Note));
        }
    });
}
#[divan::bench]
pub(crate) fn set_prop(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
        for state in black_box(states) {
            black_box(state.set(PropName::Note, PropValue::Didgeridoo));
        }
    });
}
#[divan::bench]
pub(crate) fn is_liquid(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
        for state in black_box(states) {
            black_box(state.is_liquid());
        }
    });
}
#[divan::bench]
pub(crate) fn is_opaque(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
        for state in black_box(states) {
            black_box(state.is_opaque());
        }
    });
}
#[divan::bench]
pub(crate) fn is_replaceable(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
        for state in black_box(states) {
            black_box(state.is_replaceable());
        }
    });
}
#[divan::bench]
pub(crate) fn luminance(bencher: Bencher) {
    let states = BlockKind::ALL.map(BlockKind::to_state);
    bencher.bench(|| {
        for state in black_box(states) {
            black_box(state.luminance());
        }
    });
}
#[divan::bench]
pub(crate) fn to_item_kind(bencher: Bencher) {
    bencher.bench(|| {
        for kind in black_box(BlockKind::ALL) {
            black_box(kind.to_item_kind());
        }
    });
}
#[divan::bench]
pub(crate) fn from_item_kind(bencher: Bencher) {
    bencher.bench(|| {
        for kind in black_box(ItemKind::ALL) {
            black_box(BlockKind::from_item_kind(kind));
        }
    });
}
