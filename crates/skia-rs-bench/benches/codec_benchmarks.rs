//! Image codec benchmarks.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use skia_rs_codec::{
    EncoderQuality, Image, ImageDecoder, ImageEncoder, ImageFormat, ImageInfo, JpegDecoder,
    JpegEncoder, PngDecoder, PngEncoder,
};
use skia_rs_core::{AlphaType, ColorType};
use std::hint::black_box;

fn create_test_image(width: i32, height: i32) -> Image {
    let info = ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul);
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Create a gradient pattern
    for y in 0..height {
        for x in 0..width {
            let offset = ((y * width + x) * 4) as usize;
            pixels[offset] = (x * 255 / width) as u8; // R
            pixels[offset + 1] = (y * 255 / height) as u8; // G
            pixels[offset + 2] = 128; // B
            pixels[offset + 3] = 255; // A
        }
    }

    Image::from_raster_data_owned(info, pixels, width as usize * 4).unwrap()
}

fn bench_image_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codec/image_creation");

    for (name, (w, h)) in [
        ("small", (64, 64)),
        ("medium", (256, 256)),
        ("large", (1024, 1024)),
    ] {
        group.throughput(Throughput::Elements((w * h) as u64));
        group.bench_with_input(
            BenchmarkId::new("from_raster", name),
            &(w, h),
            |b, &(w, h)| {
                let info = ImageInfo::new(w, h, ColorType::Rgba8888, AlphaType::Premul);
                let pixels = vec![0u8; (w * h * 4) as usize];
                b.iter(|| {
                    Image::from_raster_data_owned(
                        black_box(info.clone()),
                        black_box(pixels.clone()),
                        black_box(w as usize * 4),
                    )
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("from_color", name),
            &(w, h),
            |b, &(w, h)| {
                b.iter(|| {
                    Image::from_color(
                        black_box(w),
                        black_box(h),
                        black_box(0xFF804020u32), // ARGB color as u32
                    )
                })
            },
        );
    }

    group.finish();
}

fn bench_format_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codec/format_detection");

    // PNG magic bytes
    let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    group.bench_function("png", |b| {
        b.iter(|| ImageFormat::from_magic(black_box(&png_data)))
    });

    // JPEG magic bytes
    let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
    group.bench_function("jpeg", |b| {
        b.iter(|| ImageFormat::from_magic(black_box(&jpeg_data)))
    });

    // GIF magic bytes
    let gif_data = b"GIF89a\x00\x00";
    group.bench_function("gif", |b| {
        b.iter(|| ImageFormat::from_magic(black_box(gif_data)))
    });

    // WebP magic bytes
    let webp_data = b"RIFF\x00\x00\x00\x00WEBP";
    group.bench_function("webp", |b| {
        b.iter(|| ImageFormat::from_magic(black_box(webp_data)))
    });

    group.finish();
}

fn bench_png_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codec/png_encode");

    for (name, (w, h)) in [
        ("small", (64, 64)),
        ("medium", (256, 256)),
        ("large", (1024, 1024)),
    ] {
        let image = create_test_image(w, h);
        let encoder = PngEncoder::new();

        group.throughput(Throughput::Bytes((w * h * 4) as u64));
        group.bench_with_input(BenchmarkId::new("encode", name), &image, |b, image| {
            b.iter(|| {
                let mut output = Vec::new();
                encoder.encode(black_box(image), &mut output).unwrap();
                output
            })
        });
    }

    group.finish();
}

fn bench_png_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codec/png_decode");

    for (name, (w, h)) in [
        ("small", (64, 64)),
        ("medium", (256, 256)),
        ("large", (1024, 1024)),
    ] {
        // Create and encode an image first
        let image = create_test_image(w, h);
        let encoder = PngEncoder::new();
        let mut encoded = Vec::new();
        encoder.encode(&image, &mut encoded).unwrap();

        let decoder = PngDecoder::new();

        group.throughput(Throughput::Bytes(encoded.len() as u64));
        group.bench_with_input(BenchmarkId::new("decode", name), &encoded, |b, encoded| {
            b.iter(|| decoder.decode_bytes(black_box(encoded)).unwrap())
        });
    }

    group.finish();
}

fn bench_jpeg_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codec/jpeg_encode");

    for (name, (w, h)) in [
        ("small", (64, 64)),
        ("medium", (256, 256)),
        ("large", (1024, 1024)),
    ] {
        let image = create_test_image(w, h);

        // Different quality levels
        for quality in [50, 75, 90, 100] {
            let encoder = JpegEncoder::with_quality(EncoderQuality::new(quality));

            group.throughput(Throughput::Bytes((w * h * 4) as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("q{}", quality), name),
                &image,
                |b, image| {
                    b.iter(|| {
                        let mut output = Vec::new();
                        encoder.encode(black_box(image), &mut output).unwrap();
                        output
                    })
                },
            );
        }
    }

    group.finish();
}

fn bench_jpeg_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codec/jpeg_decode");

    for (name, (w, h)) in [
        ("small", (64, 64)),
        ("medium", (256, 256)),
        ("large", (1024, 1024)),
    ] {
        // Create and encode an image first
        let image = create_test_image(w, h);
        let encoder = JpegEncoder::with_quality(EncoderQuality::new(90));
        let mut encoded = Vec::new();
        encoder.encode(&image, &mut encoded).unwrap();

        let decoder = JpegDecoder::new();

        group.throughput(Throughput::Bytes(encoded.len() as u64));
        group.bench_with_input(BenchmarkId::new("decode", name), &encoded, |b, encoded| {
            b.iter(|| decoder.decode_bytes(black_box(encoded)).unwrap())
        });
    }

    group.finish();
}

fn bench_image_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codec/image_ops");

    let image = create_test_image(1024, 1024);

    group.bench_function("width", |b| b.iter(|| black_box(&image).width()));

    group.bench_function("height", |b| b.iter(|| black_box(&image).height()));

    group.bench_function("bounds", |b| b.iter(|| black_box(&image).bounds()));

    group.bench_function("color_type", |b| b.iter(|| black_box(&image).color_type()));

    group.bench_function("alpha_type", |b| b.iter(|| black_box(&image).alpha_type()));

    // Scaling
    for scale in [0.5, 0.25, 0.125] {
        let new_w = (1024.0 * scale) as i32;
        let new_h = (1024.0 * scale) as i32;
        group.bench_with_input(
            BenchmarkId::new("scale", format!("{}x", scale)),
            &(new_w, new_h),
            |b, &(w, h)| b.iter(|| image.make_scaled(black_box(w), black_box(h))),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_image_creation,
    bench_format_detection,
    bench_png_encode,
    bench_png_decode,
    bench_jpeg_encode,
    bench_jpeg_decode,
    bench_image_operations,
);

criterion_main!(benches);
