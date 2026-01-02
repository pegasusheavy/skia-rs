//! Rasterization benchmarks.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use skia_rs_bench::{
    canvas_sizes, create_rng, generate_complex_path, generate_simple_path,
    generate_star, random_points, random_rects,
};
use skia_rs_core::{Color, Point, Rect};
use skia_rs_canvas::Surface;
use skia_rs_paint::{Paint, Style};

fn bench_raster_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("Raster/clear");

    for (name, (w, h)) in [
        ("small", canvas_sizes::SMALL),
        ("medium", canvas_sizes::MEDIUM),
        ("hd", canvas_sizes::HD),
    ] {
        group.throughput(Throughput::Elements((w * h) as u64));
        group.bench_with_input(BenchmarkId::new("surface", name), &(w, h), |b, &(w, h)| {
            let mut surface = Surface::new_raster_n32_premul(w, h).unwrap();
            b.iter(|| {
                let mut canvas = surface.raster_canvas();
                canvas.clear(black_box(Color::from_argb(255, 128, 64, 32)));
            })
        });
    }

    group.finish();
}

fn bench_raster_lines(c: &mut Criterion) {
    let mut group = c.benchmark_group("Raster/lines");

    let mut surface = Surface::new_raster_n32_premul(1920, 1080).unwrap();
    let paint = Paint::new();

    // Single line
    group.bench_function("single", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_line(
                Point::new(black_box(0.0), black_box(0.0)),
                Point::new(black_box(1000.0), black_box(500.0)),
                black_box(&paint),
            );
        })
    });

    // Anti-aliased line
    let mut aa_paint = Paint::new();
    aa_paint.set_anti_alias(true);
    group.bench_function("single_aa", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_line(
                Point::new(black_box(0.0), black_box(0.0)),
                Point::new(black_box(1000.0), black_box(500.0)),
                black_box(&aa_paint),
            );
        })
    });

    // Batch lines
    for count in [10, 100, 1000] {
        let mut rng = create_rng();
        let bounds = Rect::from_xywh(0.0, 0.0, 1920.0, 1080.0);
        let points = random_points(&mut rng, count * 2, &bounds);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("batch", count), &points, |b, points| {
            b.iter(|| {
                let mut canvas = surface.raster_canvas();
                for pair in points.chunks(2) {
                    if pair.len() == 2 {
                        canvas.draw_line(pair[0], pair[1], &paint);
                    }
                }
            })
        });
    }

    group.finish();
}

fn bench_raster_rects(c: &mut Criterion) {
    let mut group = c.benchmark_group("Raster/rects");

    let mut surface = Surface::new_raster_n32_premul(1920, 1080).unwrap();

    let mut fill_paint = Paint::new();
    fill_paint.set_style(Style::Fill);
    fill_paint.set_color32(Color::from_argb(255, 0, 0, 255));

    let mut stroke_paint = Paint::new();
    stroke_paint.set_style(Style::Stroke);
    stroke_paint.set_stroke_width(2.0);
    stroke_paint.set_color32(Color::from_argb(255, 255, 0, 0));

    let rect = Rect::from_xywh(100.0, 100.0, 500.0, 300.0);

    group.bench_function("fill/single", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_rect(black_box(&rect), black_box(&fill_paint));
        })
    });

    group.bench_function("stroke/single", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_rect(black_box(&rect), black_box(&stroke_paint));
        })
    });

    // Batch rects
    for count in [10, 100, 1000] {
        let mut rng = create_rng();
        let bounds = Rect::from_xywh(0.0, 0.0, 1920.0, 1080.0);
        let rects = random_rects(&mut rng, count, &bounds, 100.0);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("fill/batch", count), &rects, |b, rects| {
            b.iter(|| {
                let mut canvas = surface.raster_canvas();
                for rect in rects {
                    canvas.draw_rect(rect, &fill_paint);
                }
            })
        });
    }

    group.finish();
}

fn bench_raster_circles(c: &mut Criterion) {
    let mut group = c.benchmark_group("Raster/circles");

    let mut surface = Surface::new_raster_n32_premul(1920, 1080).unwrap();

    let mut fill_paint = Paint::new();
    fill_paint.set_style(Style::Fill);
    fill_paint.set_color32(Color::from_argb(255, 0, 255, 0));

    let mut aa_paint = Paint::new();
    aa_paint.set_style(Style::Fill);
    aa_paint.set_anti_alias(true);
    aa_paint.set_color32(Color::from_argb(255, 0, 255, 0));

    let center = Point::new(500.0, 400.0);
    let radius = 100.0;

    group.bench_function("fill/aliased", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_circle(black_box(center), black_box(radius), black_box(&fill_paint));
        })
    });

    group.bench_function("fill/antialiased", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_circle(black_box(center), black_box(radius), black_box(&aa_paint));
        })
    });

    // Varying radius
    for radius in [10.0, 50.0, 100.0, 500.0] {
        group.bench_with_input(BenchmarkId::new("fill/radius", radius as i32), &radius, |b, &radius| {
            b.iter(|| {
                let mut canvas = surface.raster_canvas();
                canvas.draw_circle(center, black_box(radius), &fill_paint);
            })
        });
    }

    group.finish();
}

fn bench_raster_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("Raster/paths");

    let mut surface = Surface::new_raster_n32_premul(1920, 1080).unwrap();

    let mut fill_paint = Paint::new();
    fill_paint.set_style(Style::Fill);

    let mut stroke_paint = Paint::new();
    stroke_paint.set_style(Style::Stroke);
    stroke_paint.set_stroke_width(2.0);

    // Simple path
    let simple_path = generate_simple_path(50);
    group.bench_function("simple/stroke", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_path(black_box(&simple_path), black_box(&stroke_paint));
        })
    });

    // Complex path with curves
    let complex_path = generate_complex_path(50);
    group.bench_function("complex/stroke", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_path(black_box(&complex_path), black_box(&stroke_paint));
        })
    });

    group.bench_function("complex/fill", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_path(black_box(&complex_path), black_box(&fill_paint));
        })
    });

    // Star path
    for points in [5, 10, 20, 50] {
        let star = generate_star(points, 200.0, 100.0);
        group.bench_with_input(BenchmarkId::new("star/stroke", points), &star, |b, star| {
            b.iter(|| {
                let mut canvas = surface.raster_canvas();
                canvas.draw_path(black_box(star), black_box(&stroke_paint));
            })
        });
    }

    // Path complexity scaling
    for segments in [10, 50, 100, 500, 1000] {
        let path = generate_simple_path(segments);
        group.bench_with_input(BenchmarkId::new("segments", segments), &path, |b, path| {
            b.iter(|| {
                let mut canvas = surface.raster_canvas();
                canvas.draw_path(black_box(path), black_box(&stroke_paint));
            })
        });
    }

    group.finish();
}

fn bench_raster_blending(c: &mut Criterion) {
    let mut group = c.benchmark_group("Raster/blending");

    let mut surface = Surface::new_raster_n32_premul(1920, 1080).unwrap();

    // Pre-fill with background
    {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::from_argb(255, 100, 100, 100));
    }

    let rect = Rect::from_xywh(100.0, 100.0, 500.0, 300.0);

    for (name, mode) in [
        ("src_over", skia_rs_paint::BlendMode::SrcOver),
        ("multiply", skia_rs_paint::BlendMode::Multiply),
        ("screen", skia_rs_paint::BlendMode::Screen),
        ("overlay", skia_rs_paint::BlendMode::Overlay),
    ] {
        let mut paint = Paint::new();
        paint.set_style(Style::Fill);
        paint.set_color32(Color::from_argb(128, 255, 0, 0));
        paint.set_blend_mode(mode);

        group.bench_with_input(BenchmarkId::new("mode", name), &paint, |b, paint| {
            b.iter(|| {
                let mut canvas = surface.raster_canvas();
                canvas.draw_rect(black_box(&rect), black_box(paint));
            })
        });
    }

    group.finish();
}

fn bench_raster_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("Raster/transforms");

    let mut surface = Surface::new_raster_n32_premul(1920, 1080).unwrap();
    let rect = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
    let paint = Paint::new();

    // Drawing with identity
    group.bench_function("identity", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.draw_rect(&rect, &paint);
        })
    });

    // Drawing with translation
    group.bench_function("translated", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.translate(500.0, 300.0);
            canvas.draw_rect(&rect, &paint);
        })
    });

    // Drawing with scale
    group.bench_function("scaled", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.scale(2.0, 2.0);
            canvas.draw_rect(&rect, &paint);
        })
    });

    // Drawing with rotation
    group.bench_function("rotated", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.translate(500.0, 300.0);
            canvas.rotate(45.0);
            canvas.draw_rect(&rect, &paint);
        })
    });

    // Complex transform chain
    group.bench_function("complex_transform", |b| {
        b.iter(|| {
            let mut canvas = surface.raster_canvas();
            canvas.translate(500.0, 300.0);
            canvas.scale(2.0, 1.5);
            canvas.rotate(30.0);
            canvas.draw_rect(&rect, &paint);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_raster_clear,
    bench_raster_lines,
    bench_raster_rects,
    bench_raster_circles,
    bench_raster_paths,
    bench_raster_blending,
    bench_raster_transforms,
);

criterion_main!(benches);
