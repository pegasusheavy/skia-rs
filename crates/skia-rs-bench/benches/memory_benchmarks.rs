//! Memory usage benchmarks for skia-rs.
//!
//! These benchmarks measure memory allocation patterns rather than speed.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

use skia_rs_bench::{
    canvas_sizes, generate_complex_path, generate_multi_contour_path, generate_simple_path,
    generate_star, random_points, random_rects, sizes,
};
use skia_rs_canvas::Surface;
use skia_rs_core::{Color, Point, Rect};
use skia_rs_paint::Paint;
use skia_rs_path::PathBuilder;

/// Benchmark surface allocation for various sizes.
fn bench_surface_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/surface_allocation");

    let sizes = [
        ("64x64", 64, 64),
        ("256x256", 256, 256),
        ("512x512", 512, 512),
        ("1024x1024", 1024, 1024),
        ("1920x1080", 1920, 1080),
        ("3840x2160", 3840, 2160),
    ];

    for (name, width, height) in sizes {
        let expected_bytes = (width * height * 4) as u64; // RGBA
        group.throughput(Throughput::Bytes(expected_bytes));

        group.bench_with_input(
            BenchmarkId::new("create", name),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    let surface = Surface::new_raster_n32_premul(w, h);
                    black_box(surface)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark path memory usage.
fn bench_path_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/path");

    let segment_counts = [10, 100, 1000, 10000];

    for &count in &segment_counts {
        group.bench_with_input(
            BenchmarkId::new("simple_path", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let path = generate_simple_path(count);
                    black_box(path)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("complex_path", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let path = generate_complex_path(count);
                    black_box(path)
                });
            },
        );
    }

    // Multi-contour paths
    let configs = [(10, 10), (10, 100), (100, 10), (100, 100)];
    for (contours, segments) in configs {
        let name = format!("{}contours_{}segs", contours, segments);
        group.bench_with_input(
            BenchmarkId::new("multi_contour", &name),
            &(contours, segments),
            |b, &(c, s)| {
                b.iter(|| {
                    let path = generate_multi_contour_path(c, s);
                    black_box(path)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark PathBuilder incremental growth.
fn bench_pathbuilder_growth(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/pathbuilder_growth");

    let counts = [100, 1000, 10000, 100000];

    for &count in &counts {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("line_to", count), &count, |b, &count| {
            b.iter(|| {
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, 0.0);
                for i in 0..count {
                    builder.line_to(i as f32, (i % 100) as f32);
                }
                black_box(builder.build())
            });
        });

        group.bench_with_input(BenchmarkId::new("cubic_to", count), &count, |b, &count| {
            b.iter(|| {
                let mut builder = PathBuilder::new();
                builder.move_to(0.0, 0.0);
                for i in 0..count {
                    let x = i as f32;
                    builder.cubic_to(x, 10.0, x + 5.0, -10.0, x + 10.0, 0.0);
                }
                black_box(builder.build())
            });
        });
    }

    group.finish();
}

/// Benchmark Paint cloning.
fn bench_paint_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/paint");

    // Simple paint
    let simple_paint = Paint::new();

    group.bench_function("clone_simple", |b| {
        b.iter(|| black_box(simple_paint.clone()));
    });

    // Paint with various settings
    let mut complex_paint = Paint::new();
    complex_paint.set_color32(Color::from_argb(128, 255, 0, 0));
    complex_paint.set_stroke_width(2.0);
    complex_paint.set_anti_alias(true);

    group.bench_function("clone_complex", |b| {
        b.iter(|| black_box(complex_paint.clone()));
    });

    group.finish();
}

/// Benchmark drawing operations memory overhead.
fn bench_drawing_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/drawing");

    // Pre-create test data
    let mut rng = skia_rs_bench::create_rng();
    let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 1000.0);

    let rects_100 = random_rects(&mut rng, 100, &bounds, 50.0);
    let rects_1000 = random_rects(&mut rng, 1000, &bounds, 50.0);
    let points_100 = random_points(&mut rng, 100, &bounds);
    let points_1000 = random_points(&mut rng, 1000, &bounds);

    let star_path = generate_star(5, 50.0, 20.0);
    let complex_path = generate_complex_path(100);

    let paint = Paint::new();

    // Create surface once
    let mut surface = Surface::new_raster_n32_premul(1000, 1000).unwrap();

    group.bench_function("draw_100_rects", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            for rect in &rects_100 {
                canvas.draw_rect(rect, &paint);
            }
        });
    });

    group.bench_function("draw_1000_rects", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            for rect in &rects_1000 {
                canvas.draw_rect(rect, &paint);
            }
        });
    });

    group.bench_function("draw_100_circles", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            for point in &points_100 {
                canvas.draw_circle(*point, 10.0, &paint);
            }
        });
    });

    group.bench_function("draw_1000_circles", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            for point in &points_1000 {
                canvas.draw_circle(*point, 10.0, &paint);
            }
        });
    });

    group.bench_function("draw_star_path_100x", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            for _ in 0..100 {
                canvas.draw_path(&star_path, &paint);
            }
        });
    });

    group.bench_function("draw_complex_path_100x", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            for _ in 0..100 {
                canvas.draw_path(&complex_path, &paint);
            }
        });
    });

    group.finish();
}

/// Benchmark batch operations.
fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/batch");

    let mut rng = skia_rs_bench::create_rng();
    let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 1000.0);

    // Test point batch allocation
    for &count in &[100, 1000, 10000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("alloc_points", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let points = random_points(&mut rng, count, &bounds);
                    black_box(points)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("alloc_rects", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let rects = random_rects(&mut rng, count, &bounds, 50.0);
                    black_box(rects)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_surface_allocation,
    bench_path_memory,
    bench_pathbuilder_growth,
    bench_paint_clone,
    bench_drawing_memory,
    bench_batch_operations,
);
criterion_main!(benches);
