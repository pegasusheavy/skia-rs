//! Matrix transformation benchmarks.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use skia_rs_bench::{create_rng, random_matrices, random_points, random_rects, sizes};
use skia_rs_core::{Matrix, Point, Rect};
use std::hint::black_box;

fn bench_matrix_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Matrix/creation");

    group.bench_function("identity", |b| b.iter(|| Matrix::IDENTITY));

    group.bench_function("translate", |b| {
        b.iter(|| Matrix::translate(black_box(100.0), black_box(200.0)))
    });

    group.bench_function("scale", |b| {
        b.iter(|| Matrix::scale(black_box(2.0), black_box(3.0)))
    });

    group.bench_function("rotate", |b| {
        b.iter(|| Matrix::rotate(black_box(0.785))) // ~45 degrees
    });

    group.bench_function("rotate_around", |b| {
        let pivot = Point::new(100.0, 100.0);
        b.iter(|| Matrix::rotate_around(black_box(0.785), black_box(pivot)))
    });

    group.bench_function("skew", |b| {
        b.iter(|| Matrix::skew(black_box(0.5), black_box(0.25)))
    });

    group.finish();
}

fn bench_matrix_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("Matrix/queries");

    let identity = Matrix::IDENTITY;
    let translate = Matrix::translate(100.0, 200.0);
    let complex = Matrix::translate(100.0, 200.0)
        .concat(&Matrix::scale(2.0, 2.0))
        .concat(&Matrix::rotate(0.5));

    group.bench_function("is_identity/true", |b| {
        b.iter(|| black_box(identity).is_identity())
    });

    group.bench_function("is_identity/false", |b| {
        b.iter(|| black_box(translate).is_identity())
    });

    group.bench_function("is_translate/true", |b| {
        b.iter(|| black_box(translate).is_translate())
    });

    group.bench_function("is_translate/false", |b| {
        b.iter(|| black_box(complex).is_translate())
    });

    group.finish();
}

fn bench_matrix_concat(c: &mut Criterion) {
    let mut group = c.benchmark_group("Matrix/concat");

    let m1 = Matrix::translate(100.0, 200.0);
    let m2 = Matrix::scale(2.0, 2.0);
    let m3 = Matrix::rotate(0.785);

    group.bench_function("two_matrices", |b| {
        b.iter(|| black_box(m1).concat(black_box(&m2)))
    });

    group.bench_function("three_matrices", |b| {
        b.iter(|| black_box(m1).concat(black_box(&m2)).concat(black_box(&m3)))
    });

    // Chain many matrices
    for count in [4, 8, 16, 32] {
        let mut rng = create_rng();
        let matrices = random_matrices(&mut rng, count);

        group.bench_with_input(
            BenchmarkId::new("chain", count),
            &matrices,
            |b, matrices| {
                b.iter(|| {
                    matrices
                        .iter()
                        .fold(Matrix::IDENTITY, |acc, m| acc.concat(m))
                })
            },
        );
    }

    group.finish();
}

fn bench_matrix_invert(c: &mut Criterion) {
    let mut group = c.benchmark_group("Matrix/invert");

    let translate = Matrix::translate(100.0, 200.0);
    let scale = Matrix::scale(2.0, 3.0);
    let rotate = Matrix::rotate(0.785);
    let complex = translate.concat(&scale).concat(&rotate);

    group.bench_function("translate", |b| b.iter(|| black_box(translate).invert()));

    group.bench_function("scale", |b| b.iter(|| black_box(scale).invert()));

    group.bench_function("rotate", |b| b.iter(|| black_box(rotate).invert()));

    group.bench_function("complex", |b| b.iter(|| black_box(complex).invert()));

    // Batch inversions
    for size in [sizes::SMALL, sizes::MEDIUM] {
        let mut rng = create_rng();
        let matrices = random_matrices(&mut rng, size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("batch", size), &matrices, |b, matrices| {
            b.iter(|| matrices.iter().filter_map(|m| m.invert()).count())
        });
    }

    group.finish();
}

fn bench_matrix_map_point(c: &mut Criterion) {
    let mut group = c.benchmark_group("Matrix/map_point");

    let identity = Matrix::IDENTITY;
    let translate = Matrix::translate(100.0, 200.0);
    let scale = Matrix::scale(2.0, 2.0);
    let rotate = Matrix::rotate(0.785);
    let complex = translate.concat(&scale).concat(&rotate);

    let point = Point::new(50.0, 75.0);

    group.bench_function("identity", |b| {
        b.iter(|| black_box(identity).map_point(black_box(point)))
    });

    group.bench_function("translate", |b| {
        b.iter(|| black_box(translate).map_point(black_box(point)))
    });

    group.bench_function("scale", |b| {
        b.iter(|| black_box(scale).map_point(black_box(point)))
    });

    group.bench_function("rotate", |b| {
        b.iter(|| black_box(rotate).map_point(black_box(point)))
    });

    group.bench_function("complex", |b| {
        b.iter(|| black_box(complex).map_point(black_box(point)))
    });

    // Batch point transforms
    for size in [sizes::SMALL, sizes::MEDIUM, sizes::LARGE] {
        let mut rng = create_rng();
        let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 1000.0);
        let points = random_points(&mut rng, size, &bounds);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_complex", size),
            &points,
            |b, points| {
                b.iter(|| {
                    points
                        .iter()
                        .map(|p| complex.map_point(*p))
                        .collect::<Vec<_>>()
                })
            },
        );
    }

    group.finish();
}

fn bench_matrix_map_rect(c: &mut Criterion) {
    let mut group = c.benchmark_group("Matrix/map_rect");

    let translate = Matrix::translate(100.0, 200.0);
    let scale = Matrix::scale(2.0, 2.0);
    let rotate = Matrix::rotate(0.785);
    let complex = translate.concat(&scale).concat(&rotate);

    let rect = Rect::from_xywh(10.0, 20.0, 100.0, 50.0);

    group.bench_function("translate", |b| {
        b.iter(|| black_box(translate).map_rect(black_box(&rect)))
    });

    group.bench_function("scale", |b| {
        b.iter(|| black_box(scale).map_rect(black_box(&rect)))
    });

    group.bench_function("rotate", |b| {
        b.iter(|| black_box(rotate).map_rect(black_box(&rect)))
    });

    group.bench_function("complex", |b| {
        b.iter(|| black_box(complex).map_rect(black_box(&rect)))
    });

    // Batch rect transforms
    for size in [sizes::SMALL, sizes::MEDIUM] {
        let mut rng = create_rng();
        let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 1000.0);
        let rects = random_rects(&mut rng, size, &bounds, 100.0);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_complex", size),
            &rects,
            |b, rects| {
                b.iter(|| {
                    rects
                        .iter()
                        .map(|r| complex.map_rect(r))
                        .collect::<Vec<_>>()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_matrix_creation,
    bench_matrix_queries,
    bench_matrix_concat,
    bench_matrix_invert,
    bench_matrix_map_point,
    bench_matrix_map_rect,
);

criterion_main!(benches);
