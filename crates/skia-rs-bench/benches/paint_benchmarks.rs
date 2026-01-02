//! Paint and shader benchmarks.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use skia_rs_bench::{create_rng, random_colors4f, sizes};
use skia_rs_core::{Color, Color4f, Point, Rect};
use skia_rs_paint::{
    BlendMode, BlurMaskFilter, BlurStyle, ColorMatrixFilter, ColorShader,
    DropShadowImageFilter, LinearGradient, Paint, RadialGradient, Shader,
    Style, StrokeCap, StrokeJoin, SweepGradient, TileMode,
};

fn bench_paint_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Paint/creation");

    group.bench_function("new", |b| {
        b.iter(|| Paint::new())
    });

    group.bench_function("default", |b| {
        b.iter(|| Paint::default())
    });

    group.bench_function("clone", |b| {
        let paint = Paint::new();
        b.iter(|| black_box(&paint).clone())
    });

    group.finish();
}

fn bench_paint_setters(c: &mut Criterion) {
    let mut group = c.benchmark_group("Paint/setters");

    group.bench_function("set_color", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_color(Color4f::new(0.5, 0.25, 0.125, 1.0));
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_color32", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_color32(Color::from_argb(255, 128, 64, 32));
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_argb", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_argb(255, 128, 64, 32);
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_alpha", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_alpha(0.5);
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_style", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_style(Style::Stroke);
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_stroke_width", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_stroke_width(2.0);
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_blend_mode", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_blend_mode(BlendMode::Multiply);
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("set_anti_alias", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint.set_anti_alias(false);
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // Chain multiple setters
    group.bench_function("chain_setters", |b| {
        b.iter_batched(
            Paint::new,
            |mut paint| {
                paint
                    .set_color(Color4f::new(1.0, 0.0, 0.0, 1.0))
                    .set_style(Style::Stroke)
                    .set_stroke_width(3.0)
                    .set_stroke_cap(StrokeCap::Round)
                    .set_stroke_join(StrokeJoin::Round)
                    .set_anti_alias(true)
                    .set_blend_mode(BlendMode::SrcOver);
                paint
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_paint_getters(c: &mut Criterion) {
    let mut group = c.benchmark_group("Paint/getters");

    let mut paint = Paint::new();
    paint
        .set_color(Color4f::new(0.5, 0.25, 0.125, 0.8))
        .set_style(Style::Stroke)
        .set_stroke_width(2.5)
        .set_blend_mode(BlendMode::Multiply);

    group.bench_function("color", |b| {
        b.iter(|| black_box(&paint).color())
    });

    group.bench_function("alpha", |b| {
        b.iter(|| black_box(&paint).alpha())
    });

    group.bench_function("style", |b| {
        b.iter(|| black_box(&paint).style())
    });

    group.bench_function("stroke_width", |b| {
        b.iter(|| black_box(&paint).stroke_width())
    });

    group.bench_function("blend_mode", |b| {
        b.iter(|| black_box(&paint).blend_mode())
    });

    group.bench_function("is_anti_alias", |b| {
        b.iter(|| black_box(&paint).is_anti_alias())
    });

    group.finish();
}

fn bench_blend_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("BlendMode");

    let modes = [
        BlendMode::Clear,
        BlendMode::Src,
        BlendMode::Dst,
        BlendMode::SrcOver,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
    ];

    group.bench_function("name", |b| {
        b.iter(|| {
            modes.iter().map(|m| m.name()).collect::<Vec<_>>()
        })
    });

    group.finish();
}

fn bench_shader_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Shader/creation");

    group.bench_function("color_shader", |b| {
        b.iter(|| ColorShader::new(Color4f::new(1.0, 0.0, 0.0, 1.0)))
    });

    let colors = vec![
        Color4f::new(1.0, 0.0, 0.0, 1.0),
        Color4f::new(0.0, 1.0, 0.0, 1.0),
        Color4f::new(0.0, 0.0, 1.0, 1.0),
    ];

    group.bench_function("linear_gradient", |b| {
        b.iter(|| {
            LinearGradient::new(
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                colors.clone(),
                None,
                TileMode::Clamp,
            )
        })
    });

    group.bench_function("radial_gradient", |b| {
        b.iter(|| {
            RadialGradient::new(
                Point::new(50.0, 50.0),
                50.0,
                colors.clone(),
                None,
                TileMode::Clamp,
            )
        })
    });

    group.bench_function("sweep_gradient", |b| {
        b.iter(|| {
            SweepGradient::new(
                Point::new(50.0, 50.0),
                0.0,
                360.0,
                colors.clone(),
                None,
                TileMode::Clamp,
            )
        })
    });

    // Gradients with varying color stop counts
    for stop_count in [2, 4, 8, 16, 32] {
        let mut rng = create_rng();
        let colors = random_colors4f(&mut rng, stop_count);

        group.bench_with_input(
            BenchmarkId::new("linear_gradient_stops", stop_count),
            &colors,
            |b, colors| {
                b.iter(|| {
                    LinearGradient::new(
                        Point::new(0.0, 0.0),
                        Point::new(100.0, 0.0),
                        colors.clone(),
                        None,
                        TileMode::Clamp,
                    )
                })
            },
        );
    }

    group.finish();
}

fn bench_shader_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("Shader/queries");

    let color_shader = ColorShader::new(Color4f::new(1.0, 0.0, 0.0, 1.0));
    let opaque_color = ColorShader::new(Color4f::new(1.0, 0.0, 0.0, 1.0));
    let transparent_color = ColorShader::new(Color4f::new(1.0, 0.0, 0.0, 0.5));

    let gradient = LinearGradient::new(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        vec![
            Color4f::new(1.0, 0.0, 0.0, 1.0),
            Color4f::new(0.0, 1.0, 0.0, 1.0),
        ],
        None,
        TileMode::Clamp,
    );

    group.bench_function("color_is_opaque/true", |b| {
        b.iter(|| black_box(&opaque_color).is_opaque())
    });

    group.bench_function("color_is_opaque/false", |b| {
        b.iter(|| black_box(&transparent_color).is_opaque())
    });

    group.bench_function("gradient_is_opaque", |b| {
        b.iter(|| black_box(&gradient).is_opaque())
    });

    group.bench_function("local_matrix", |b| {
        b.iter(|| black_box(&color_shader).local_matrix())
    });

    group.finish();
}

fn bench_color_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("ColorFilter");

    group.bench_function("identity_create", |b| {
        b.iter(|| ColorMatrixFilter::identity())
    });

    group.bench_function("saturation_create", |b| {
        b.iter(|| ColorMatrixFilter::saturation(black_box(0.5)))
    });

    let identity = ColorMatrixFilter::identity();
    let saturation = ColorMatrixFilter::saturation(0.5);
    let color = Color4f::new(0.8, 0.6, 0.4, 1.0);

    use skia_rs_paint::ColorFilter;

    group.bench_function("identity_filter", |b| {
        b.iter(|| identity.filter_color(black_box(color)))
    });

    group.bench_function("saturation_filter", |b| {
        b.iter(|| saturation.filter_color(black_box(color)))
    });

    // Batch filtering
    for size in [sizes::SMALL, sizes::MEDIUM, sizes::LARGE] {
        let mut rng = create_rng();
        let colors = random_colors4f(&mut rng, size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_saturation", size),
            &colors,
            |b, colors| {
                b.iter(|| {
                    colors.iter().map(|c| saturation.filter_color(*c)).collect::<Vec<_>>()
                })
            },
        );
    }

    group.finish();
}

fn bench_mask_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("MaskFilter");

    group.bench_function("blur_create", |b| {
        b.iter(|| BlurMaskFilter::new(BlurStyle::Normal, black_box(5.0)))
    });

    let blur = BlurMaskFilter::new(BlurStyle::Normal, 5.0);

    group.bench_function("blur_style", |b| {
        b.iter(|| black_box(&blur).style())
    });

    group.bench_function("blur_sigma", |b| {
        b.iter(|| black_box(&blur).sigma())
    });

    use skia_rs_paint::MaskFilter;
    group.bench_function("blur_radius", |b| {
        b.iter(|| black_box(&blur).blur_radius())
    });

    group.finish();
}

fn bench_image_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("ImageFilter");

    use skia_rs_paint::{BlurImageFilter, ImageFilter};

    group.bench_function("blur_create", |b| {
        b.iter(|| BlurImageFilter::new(black_box(5.0), black_box(5.0), TileMode::Clamp))
    });

    group.bench_function("drop_shadow_create", |b| {
        b.iter(|| {
            DropShadowImageFilter::new(
                black_box(3.0),
                black_box(3.0),
                black_box(2.0),
                black_box(2.0),
                Color4f::new(0.0, 0.0, 0.0, 0.5),
                false,
            )
        })
    });

    let blur = BlurImageFilter::new(5.0, 5.0, TileMode::Clamp);
    let shadow = DropShadowImageFilter::new(3.0, 3.0, 2.0, 2.0, Color4f::new(0.0, 0.0, 0.0, 0.5), false);
    let rect = Rect::from_xywh(10.0, 20.0, 100.0, 50.0);

    group.bench_function("blur_filter_bounds", |b| {
        b.iter(|| blur.filter_bounds(black_box(&rect)))
    });

    group.bench_function("shadow_filter_bounds", |b| {
        b.iter(|| shadow.filter_bounds(black_box(&rect)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_paint_creation,
    bench_paint_setters,
    bench_paint_getters,
    bench_blend_mode,
    bench_shader_creation,
    bench_shader_queries,
    bench_color_filter,
    bench_mask_filter,
    bench_image_filter,
);

criterion_main!(benches);
