#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{Color, Color4f, Scalar};
use skia_rs_paint::{BlendMode, Paint, Style, StrokeCap, StrokeJoin};

#[derive(Debug, Arbitrary)]
struct PaintInput {
    // Color
    a: u8,
    r: u8,
    g: u8,
    b: u8,
    // Float color
    rf: Scalar,
    gf: Scalar,
    bf: Scalar,
    af: Scalar,
    // Style settings
    style: u8,       // 0-2
    blend_mode: u8,  // 0-28
    stroke_cap: u8,  // 0-2
    stroke_join: u8, // 0-2
    // Stroke parameters
    stroke_width: Scalar,
    stroke_miter: Scalar,
    // Flags
    anti_alias: bool,
    dither: bool,
}

fuzz_target!(|input: PaintInput| {
    let mut paint = Paint::new();

    // Set color from ARGB
    paint.set_argb(input.a, input.r, input.g, input.b);

    // Verify color was set
    let color = paint.color();
    assert!(color.a >= 0.0 && color.a <= 1.0);

    // Set color from Color32
    paint.set_color32(Color::from_argb(input.a, input.r, input.g, input.b));

    // Set color from Color4f
    if input.rf.is_finite() && input.gf.is_finite() &&
       input.bf.is_finite() && input.af.is_finite() {
        paint.set_color(Color4f::new(input.rf, input.gf, input.bf, input.af));
    }

    // Set style
    let style = match input.style % 3 {
        0 => Style::Fill,
        1 => Style::Stroke,
        _ => Style::StrokeAndFill,
    };
    paint.set_style(style);
    assert_eq!(paint.style(), style);

    // Set blend mode
    let blend_mode = match input.blend_mode % 29 {
        0 => BlendMode::Clear,
        1 => BlendMode::Src,
        2 => BlendMode::Dst,
        3 => BlendMode::SrcOver,
        4 => BlendMode::DstOver,
        5 => BlendMode::SrcIn,
        6 => BlendMode::DstIn,
        7 => BlendMode::SrcOut,
        8 => BlendMode::DstOut,
        9 => BlendMode::SrcATop,
        10 => BlendMode::DstATop,
        11 => BlendMode::Xor,
        12 => BlendMode::Plus,
        13 => BlendMode::Modulate,
        14 => BlendMode::Screen,
        15 => BlendMode::Overlay,
        16 => BlendMode::Darken,
        17 => BlendMode::Lighten,
        18 => BlendMode::ColorDodge,
        19 => BlendMode::ColorBurn,
        20 => BlendMode::HardLight,
        21 => BlendMode::SoftLight,
        22 => BlendMode::Difference,
        23 => BlendMode::Exclusion,
        24 => BlendMode::Multiply,
        25 => BlendMode::Hue,
        26 => BlendMode::Saturation,
        27 => BlendMode::Color,
        _ => BlendMode::Luminosity,
    };
    paint.set_blend_mode(blend_mode);
    assert_eq!(paint.blend_mode(), blend_mode);

    // Set stroke properties
    if input.stroke_width.is_finite() {
        paint.set_stroke_width(input.stroke_width);
        // Width should be clamped to >= 0
        assert!(paint.stroke_width() >= 0.0);
    }

    if input.stroke_miter.is_finite() {
        paint.set_stroke_miter(input.stroke_miter);
        assert!(paint.stroke_miter() >= 0.0);
    }

    // Set stroke cap
    let cap = match input.stroke_cap % 3 {
        0 => StrokeCap::Butt,
        1 => StrokeCap::Round,
        _ => StrokeCap::Square,
    };
    paint.set_stroke_cap(cap);
    assert_eq!(paint.stroke_cap(), cap);

    // Set stroke join
    let join = match input.stroke_join % 3 {
        0 => StrokeJoin::Miter,
        1 => StrokeJoin::Round,
        _ => StrokeJoin::Bevel,
    };
    paint.set_stroke_join(join);
    assert_eq!(paint.stroke_join(), join);

    // Set flags
    paint.set_anti_alias(input.anti_alias);
    assert_eq!(paint.is_anti_alias(), input.anti_alias);

    paint.set_dither(input.dither);
    assert_eq!(paint.is_dither(), input.dither);

    // Set alpha
    if input.af.is_finite() {
        paint.set_alpha(input.af);
        // Alpha should be stored
        let _ = paint.alpha();
    }

    // Test clone
    let cloned = paint.clone();
    assert_eq!(cloned.style(), paint.style());
    assert_eq!(cloned.is_anti_alias(), paint.is_anti_alias());

    // Test chaining
    let mut chained = Paint::new();
    chained
        .set_color32(Color::RED)
        .set_style(Style::Stroke)
        .set_stroke_width(2.0)
        .set_anti_alias(true);
});
