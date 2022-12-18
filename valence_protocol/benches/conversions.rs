use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(20));
    targets = criterion_benchmark
}
criterion_main!(benches);

use valence_protocol::block::BlockKind;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("BlockState::to_kind", |b| {
        b.iter_with_setup(
            || BlockKind::ALL.map(BlockKind::to_state),
            |targets| {
                for target in targets {
                    black_box(target.to_kind());
                }
            },
        );
    });
}
