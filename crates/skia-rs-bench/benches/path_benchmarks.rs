//! Path construction and operation benchmarks.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use skia_rs_bench::{
    generate_complex_path, generate_multi_contour_path, generate_nested_rects,
    generate_simple_path, generate_star, sizes,
};
use skia_rs_core::Rect;
use skia_rs_path::{FillType, Path, PathBuilder};
use std::hint::black_box;

fn bench_path_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("PathBuilder");

    group.bench_function("new", |b| b.iter(|| PathBuilder::new()));

    group.bench_function("move_to", |b| {
        b.iter_batched(
            PathBuilder::new,
            |mut builder| {
                builder.move_to(black_box(100.0), black_box(200.0));
                builder
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("line_to", |b| {
        b.iter_batched(
            || {
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, 0.0);
                builder
            },
            |mut builder| {
                builder.line_to(black_box(100.0), black_box(200.0));
                builder
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("quad_to", |b| {
        b.iter_batched(
            || {
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, 0.0);
                builder
            },
            |mut builder| {
                builder.quad_to(
                    black_box(50.0),
                    black_box(100.0),
                    black_box(100.0),
                    black_box(0.0),
                );
                builder
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("cubic_to", |b| {
        b.iter_batched(
            || {
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, 0.0);
                builder
            },
            |mut builder| {
                builder.cubic_to(
                    black_box(25.0),
                    black_box(50.0),
                    black_box(75.0),
                    black_box(50.0),
                    black_box(100.0),
                    black_box(0.0),
                );
                builder
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("conic_to", |b| {
        b.iter_batched(
            || {
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, 0.0);
                builder
            },
            |mut builder| {
                builder.conic_to(
                    black_box(50.0),
                    black_box(100.0),
                    black_box(100.0),
                    black_box(0.0),
                    black_box(0.707),
                );
                builder
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("close", |b| {
        b.iter_batched(
            || {
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, 0.0);
                builder.line_to(100.0, 0.0);
                builder.line_to(100.0, 100.0);
                builder
            },
            |mut builder| {
                builder.close();
                builder
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_path_shapes(c: &mut Criterion) {
    let mut group = c.benchmark_group("PathBuilder/shapes");

    let rect = Rect::from_xywh(10.0, 20.0, 100.0, 50.0);

    group.bench_function("add_rect", |b| {
        b.iter_batched(
            PathBuilder::new,
            |mut builder| {
                builder.add_rect(black_box(&rect));
                builder.build()
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("add_oval", |b| {
        b.iter_batched(
            PathBuilder::new,
            |mut builder| {
                builder.add_oval(black_box(&rect));
                builder.build()
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("add_circle", |b| {
        b.iter_batched(
            PathBuilder::new,
            |mut builder| {
                builder.add_circle(black_box(50.0), black_box(50.0), black_box(25.0));
                builder.build()
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("add_round_rect", |b| {
        b.iter_batched(
            PathBuilder::new,
            |mut builder| {
                builder.add_round_rect(black_box(&rect), black_box(5.0), black_box(5.0));
                builder.build()
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // Star shapes with varying complexity
    for points in [5, 8, 12, 24] {
        group.bench_with_input(BenchmarkId::new("star", points), &points, |b, &points| {
            b.iter(|| generate_star(points, 100.0, 50.0))
        });
    }

    group.finish();
}

fn bench_path_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Path/construction");

    // Build paths of varying complexity
    for size in [10, 50, 100, 500, 1000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("simple_lines", size), &size, |b, &size| {
            b.iter(|| generate_simple_path(size))
        });

        group.bench_with_input(BenchmarkId::new("mixed_curves", size), &size, |b, &size| {
            b.iter(|| generate_complex_path(size))
        });
    }

    // Multi-contour paths
    for (contours, segments) in [(5, 20), (10, 10), (20, 5), (50, 10)] {
        let label = format!("{}x{}", contours, segments);
        group.throughput(Throughput::Elements((contours * segments) as u64));

        group.bench_with_input(
            BenchmarkId::new("multi_contour", &label),
            &(contours, segments),
            |b, &(contours, segments)| b.iter(|| generate_multi_contour_path(contours, segments)),
        );
    }

    // Nested rectangles
    for count in [5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("nested_rects", count),
            &count,
            |b, &count| b.iter(|| generate_nested_rects(count, 2.0)),
        );
    }

    group.finish();
}

fn bench_path_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("Path/queries");

    let simple_path = generate_simple_path(100);
    let complex_path = generate_complex_path(100);
    let multi_contour = generate_multi_contour_path(10, 20);

    group.bench_function("is_empty/false", |b| {
        b.iter(|| black_box(&simple_path).is_empty())
    });

    group.bench_function("is_empty/true", |b| {
        let empty = Path::new();
        b.iter(|| black_box(&empty).is_empty())
    });

    group.bench_function("verb_count", |b| {
        b.iter(|| black_box(&simple_path).verb_count())
    });

    group.bench_function("point_count", |b| {
        b.iter(|| black_box(&simple_path).point_count())
    });

    group.bench_function("bounds/simple", |b| {
        b.iter(|| black_box(&simple_path).bounds())
    });

    group.bench_function("bounds/complex", |b| {
        b.iter(|| black_box(&complex_path).bounds())
    });

    group.bench_function("bounds/multi_contour", |b| {
        b.iter(|| black_box(&multi_contour).bounds())
    });

    group.bench_function("fill_type", |b| {
        b.iter(|| black_box(&simple_path).fill_type())
    });

    group.finish();
}

fn bench_path_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("Path/iteration");

    for size in [50, 100, 500, 1000] {
        let simple_path = generate_simple_path(size);
        let complex_path = generate_complex_path(size);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("simple_iter", size),
            &simple_path,
            |b, path| b.iter(|| path.iter().count()),
        );

        group.bench_with_input(
            BenchmarkId::new("complex_iter", size),
            &complex_path,
            |b, path| b.iter(|| path.iter().count()),
        );

        group.bench_with_input(
            BenchmarkId::new("verbs_slice", size),
            &simple_path,
            |b, path| b.iter(|| black_box(path.verbs()).len()),
        );

        group.bench_with_input(
            BenchmarkId::new("points_slice", size),
            &simple_path,
            |b, path| b.iter(|| black_box(path.points()).len()),
        );
    }

    group.finish();
}

fn bench_path_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Path/mutation");

    let path = generate_simple_path(100);

    group.bench_function("clone", |b| b.iter(|| black_box(&path).clone()));

    group.bench_function("reset", |b| {
        b.iter_batched(
            || path.clone(),
            |mut p| {
                p.reset();
                p
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_fill_type", |b| {
        b.iter_batched(
            || path.clone(),
            |mut p| {
                p.set_fill_type(FillType::EvenOdd);
                p
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_path_builder,
    bench_path_shapes,
    bench_path_construction,
    bench_path_queries,
    bench_path_iteration,
    bench_path_mutation,
);

criterion_main!(benches);
