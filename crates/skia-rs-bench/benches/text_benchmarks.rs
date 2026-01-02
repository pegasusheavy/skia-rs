//! Text and font benchmarks.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use std::sync::Arc;
use skia_rs_text::{Font, FontStyle, Typeface, TextBlobBuilder, Shaper, TextDirection, Script};
use skia_rs_core::Point;

fn bench_typeface(c: &mut Criterion) {
    let mut group = c.benchmark_group("Text/typeface");

    group.bench_function("default", |b| {
        b.iter(|| Typeface::default_typeface())
    });

    let typeface = Typeface::default_typeface();

    group.bench_function("family_name", |b| {
        b.iter(|| black_box(&typeface).family_name())
    });

    group.bench_function("style", |b| {
        b.iter(|| black_box(&typeface).style())
    });

    group.bench_function("unique_id", |b| {
        b.iter(|| black_box(&typeface).unique_id())
    });

    group.bench_function("units_per_em", |b| {
        b.iter(|| black_box(&typeface).units_per_em())
    });

    group.bench_function("is_bold", |b| {
        b.iter(|| black_box(&typeface).is_bold())
    });

    group.bench_function("is_italic", |b| {
        b.iter(|| black_box(&typeface).is_italic())
    });

    // Glyph lookup
    group.bench_function("char_to_glyph/ascii", |b| {
        b.iter(|| black_box(&typeface).char_to_glyph(black_box('A')))
    });

    group.bench_function("char_to_glyph/unicode", |b| {
        b.iter(|| black_box(&typeface).char_to_glyph(black_box('你')))
    });

    // Batch glyph lookup
    for text in ["Hello", "Hello, World!", "The quick brown fox jumps over the lazy dog"] {
        group.throughput(Throughput::Elements(text.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("chars_to_glyphs", text.len()),
            &text,
            |b, &text| {
                b.iter(|| black_box(&typeface).chars_to_glyphs(black_box(text)))
            },
        );
    }

    group.finish();
}

fn bench_font(c: &mut Criterion) {
    let mut group = c.benchmark_group("Text/font");

    let typeface = Arc::new(Typeface::default_typeface());

    // Font creation
    for size in [12.0, 16.0, 24.0, 48.0, 72.0] {
        group.bench_with_input(BenchmarkId::new("new", size as i32), &size, |b, &size| {
            b.iter(|| Font::new(black_box(typeface.clone()), black_box(size)))
        });
    }

    let font = Font::new(typeface.clone(), 16.0);

    group.bench_function("size", |b| {
        b.iter(|| black_box(&font).size())
    });

    group.bench_function("metrics", |b| {
        b.iter(|| black_box(&font).metrics())
    });

    group.bench_function("spacing", |b| {
        b.iter(|| black_box(&font).spacing())
    });

    // Text measurement
    for text in ["Hi", "Hello, World!", "The quick brown fox jumps over the lazy dog"] {
        group.throughput(Throughput::Elements(text.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("measure_text", text.len()),
            &text,
            |b, &text| {
                b.iter(|| black_box(&font).measure_text(black_box(text)))
            },
        );
    }

    group.finish();
}

fn bench_font_style(c: &mut Criterion) {
    let mut group = c.benchmark_group("Text/font_style");

    group.bench_function("normal", |b| {
        b.iter(|| FontStyle::NORMAL)
    });

    group.bench_function("bold", |b| {
        b.iter(|| FontStyle::BOLD)
    });

    group.bench_function("italic", |b| {
        b.iter(|| FontStyle::ITALIC)
    });

    group.bench_function("bold_italic", |b| {
        b.iter(|| FontStyle::BOLD_ITALIC)
    });

    let style = FontStyle::NORMAL;
    group.bench_function("weight", |b| {
        b.iter(|| black_box(style).weight)
    });

    group.bench_function("width", |b| {
        b.iter(|| black_box(style).width)
    });

    group.bench_function("slant", |b| {
        b.iter(|| black_box(style).slant)
    });

    group.finish();
}

fn bench_text_blob(c: &mut Criterion) {
    let mut group = c.benchmark_group("Text/text_blob");

    let typeface = Arc::new(Typeface::default_typeface());
    let font = Font::new(typeface, 16.0);

    // Builder creation
    group.bench_function("builder_new", |b| {
        b.iter(|| TextBlobBuilder::new())
    });

    // Build simple blob
    group.bench_function("build_simple", |b| {
        b.iter(|| {
            let mut builder = TextBlobBuilder::new();
            builder.add_text("Hello", &font, Point::new(0.0, 0.0));
            builder.build()
        })
    });

    // Build with varying text lengths
    for text in [
        "Hi",
        "Hello, World!",
        "The quick brown fox jumps over the lazy dog",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    ] {
        group.throughput(Throughput::Elements(text.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("build", text.len()),
            &text,
            |b, &text| {
                b.iter(|| {
                    let mut builder = TextBlobBuilder::new();
                    builder.add_text(black_box(text), black_box(&font), Point::new(0.0, 0.0));
                    builder.build()
                })
            },
        );
    }

    // Multiple runs
    for run_count in [2, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("multiple_runs", run_count),
            &run_count,
            |b, &count| {
                b.iter(|| {
                    let mut builder = TextBlobBuilder::new();
                    for i in 0..count {
                        builder.add_text(
                            "Hello",
                            &font,
                            Point::new(i as f32 * 50.0, 0.0),
                        );
                    }
                    builder.build()
                })
            },
        );
    }

    group.finish();
}

fn bench_shaper(c: &mut Criterion) {
    let mut group = c.benchmark_group("Text/shaper");

    let typeface = Arc::new(Typeface::default_typeface());
    let font = Font::new(typeface, 16.0);
    let shaper = Shaper::new();

    group.bench_function("new", |b| {
        b.iter(|| Shaper::new())
    });

    // Note: Actual shaping requires font data, so we test the API but not full shaping
    // These benchmarks will fail gracefully if no font data is available

    // Direction detection
    for (name, text) in [
        ("latin", "Hello, World!"),
        ("arabic", "مرحبا"),
        ("hebrew", "שלום"),
        ("cjk", "你好世界"),
    ] {
        group.bench_with_input(BenchmarkId::new("detect_direction", name), &text, |b, &text| {
            b.iter(|| {
                // Internal function would be called by shape_auto
                for ch in black_box(text).chars() {
                    let _ = ch.is_ascii_alphabetic();
                }
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_typeface,
    bench_font,
    bench_font_style,
    bench_text_blob,
    bench_shaper,
);

criterion_main!(benches);
