//! Core type benchmarks: geometry, color, and scalar operations.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use skia_rs_bench::{create_rng, random_colors, random_colors4f, random_points, random_rects, sizes};
use skia_rs_core::{premultiply_color, Color, Color4f, Point, Rect, Scalar};

fn bench_point_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Point");

    // Point creation
    group.bench_function("new", |b| {
        b.iter(|| Point::new(black_box(100.0), black_box(200.0)))
    });

    // Point addition
    let p1 = Point::new(100.0, 200.0);
    let p2 = Point::new(50.0, 75.0);
    group.bench_function("add", |b| {
        b.iter(|| black_box(p1) + black_box(p2))
    });

    // Point subtraction
    group.bench_function("sub", |b| {
        b.iter(|| black_box(p1) - black_box(p2))
    });

    // Point scaling
    group.bench_function("scale", |b| {
        b.iter(|| black_box(p1) * black_box(2.5))
    });

    // Length calculation
    group.bench_function("length", |b| {
        b.iter(|| black_box(p1).length())
    });

    // Normalization
    group.bench_function("normalize", |b| {
        b.iter(|| black_box(p1).normalize())
    });

    // Dot product
    group.bench_function("dot", |b| {
        b.iter(|| black_box(p1).dot(&black_box(p2)))
    });

    // Cross product
    group.bench_function("cross", |b| {
        b.iter(|| black_box(p1).cross(&black_box(p2)))
    });

    // Batch operations
    for size in [sizes::SMALL, sizes::MEDIUM, sizes::LARGE] {
        let mut rng = create_rng();
        let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 1000.0);
        let points = random_points(&mut rng, size, &bounds);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("batch_normalize", size), &points, |b, points| {
            b.iter(|| {
                points.iter().map(|p| p.normalize()).collect::<Vec<_>>()
            })
        });

        group.bench_with_input(BenchmarkId::new("batch_length", size), &points, |b, points| {
            b.iter(|| {
                points.iter().map(|p| p.length()).fold(0.0, |a, b| a + b)
            })
        });
    }

    group.finish();
}

fn bench_rect_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Rect");

    // Rect creation
    group.bench_function("new_ltrb", |b| {
        b.iter(|| Rect::new(black_box(10.0), black_box(20.0), black_box(100.0), black_box(200.0)))
    });

    group.bench_function("from_xywh", |b| {
        b.iter(|| Rect::from_xywh(black_box(10.0), black_box(20.0), black_box(90.0), black_box(180.0)))
    });

    // Rect queries
    let rect = Rect::new(10.0, 20.0, 100.0, 200.0);
    let point_inside = Point::new(50.0, 100.0);
    let point_outside = Point::new(200.0, 300.0);

    group.bench_function("width", |b| {
        b.iter(|| black_box(rect).width())
    });

    group.bench_function("height", |b| {
        b.iter(|| black_box(rect).height())
    });

    group.bench_function("is_empty", |b| {
        b.iter(|| black_box(rect).is_empty())
    });

    group.bench_function("contains_hit", |b| {
        b.iter(|| black_box(rect).contains(black_box(point_inside)))
    });

    group.bench_function("contains_miss", |b| {
        b.iter(|| black_box(rect).contains(black_box(point_outside)))
    });

    // Rect operations
    let rect2 = Rect::new(50.0, 80.0, 150.0, 250.0);

    group.bench_function("intersect", |b| {
        b.iter(|| black_box(rect).intersect(black_box(&rect2)))
    });

    group.bench_function("join", |b| {
        b.iter(|| black_box(rect).join(black_box(&rect2)))
    });

    group.bench_function("offset", |b| {
        b.iter(|| black_box(rect).offset(black_box(10.0), black_box(20.0)))
    });

    group.bench_function("inset", |b| {
        b.iter(|| black_box(rect).inset(black_box(5.0), black_box(5.0)))
    });

    // Batch operations
    for size in [sizes::SMALL, sizes::MEDIUM, sizes::LARGE] {
        let mut rng = create_rng();
        let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 1000.0);
        let rects = random_rects(&mut rng, size, &bounds, 100.0);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("batch_union", size), &rects, |b, rects| {
            b.iter(|| {
                rects.iter().fold(Rect::EMPTY, |acc, r| acc.join(r))
            })
        });
    }

    group.finish();
}

fn bench_color_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Color");

    // Color creation
    group.bench_function("from_argb", |b| {
        b.iter(|| Color::from_argb(black_box(255), black_box(128), black_box(64), black_box(32)))
    });

    group.bench_function("from_rgb", |b| {
        b.iter(|| Color::from_rgb(black_box(128), black_box(64), black_box(32)))
    });

    // Component extraction
    let color = Color::from_argb(200, 128, 64, 32);
    group.bench_function("components", |b| {
        b.iter(|| {
            let c = black_box(color);
            (c.alpha(), c.red(), c.green(), c.blue())
        })
    });

    // Color conversion
    group.bench_function("to_color4f", |b| {
        b.iter(|| black_box(color).to_color4f())
    });

    // Premultiply
    group.bench_function("premultiply", |b| {
        b.iter(|| premultiply_color(black_box(color)))
    });

    // Batch operations
    for size in [sizes::SMALL, sizes::MEDIUM, sizes::LARGE] {
        let mut rng = create_rng();
        let colors = random_colors(&mut rng, size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("batch_premultiply", size), &colors, |b, colors| {
            b.iter(|| {
                colors.iter().map(|c| premultiply_color(*c)).collect::<Vec<_>>()
            })
        });

        group.bench_with_input(BenchmarkId::new("batch_to_color4f", size), &colors, |b, colors| {
            b.iter(|| {
                colors.iter().map(|c| c.to_color4f()).collect::<Vec<_>>()
            })
        });
    }

    group.finish();
}

fn bench_color4f_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Color4f");

    // Color4f creation
    group.bench_function("new", |b| {
        b.iter(|| Color4f::new(black_box(0.5), black_box(0.25), black_box(0.125), black_box(0.8)))
    });

    // Color4f operations
    let c1 = Color4f::new(0.5, 0.25, 0.125, 0.8);
    let c2 = Color4f::new(0.8, 0.6, 0.4, 1.0);

    group.bench_function("to_color", |b| {
        b.iter(|| black_box(c1).to_color())
    });

    group.bench_function("premul", |b| {
        b.iter(|| black_box(c1).premul())
    });

    group.bench_function("lerp", |b| {
        b.iter(|| black_box(c1).lerp(black_box(&c2), black_box(0.5)))
    });

    // Batch operations
    for size in [sizes::SMALL, sizes::MEDIUM, sizes::LARGE] {
        let mut rng = create_rng();
        let colors = random_colors4f(&mut rng, size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("batch_premul", size), &colors, |b, colors| {
            b.iter(|| {
                colors.iter().map(|c| c.premul()).collect::<Vec<_>>()
            })
        });

        group.bench_with_input(BenchmarkId::new("batch_to_color", size), &colors, |b, colors| {
            b.iter(|| {
                colors.iter().map(|c| c.to_color()).collect::<Vec<_>>()
            })
        });
    }

    group.finish();
}

fn bench_scalar_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Scalar");

    let values: Vec<Scalar> = (0..1000).map(|i| i as Scalar * 0.001).collect();

    group.bench_function("nearly_zero", |b| {
        b.iter(|| {
            values.iter().filter(|&&x| x.abs() < 1e-6).count()
        })
    });

    group.bench_function("nearly_equal", |b| {
        b.iter(|| {
            values.windows(2).filter(|w| (w[0] - w[1]).abs() < 1e-6).count()
        })
    });

    group.bench_function("is_finite", |b| {
        b.iter(|| {
            values.iter().filter(|&&x| x.is_finite()).count()
        })
    });

    group.bench_function("interp", |b| {
        b.iter(|| {
            let a = black_box(0.0_f32);
            let b_val = black_box(100.0_f32);
            let t = black_box(0.5_f32);
            a + (b_val - a) * t
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_point_operations,
    bench_rect_operations,
    bench_color_operations,
    bench_color4f_operations,
    bench_scalar_operations,
);

criterion_main!(benches);
