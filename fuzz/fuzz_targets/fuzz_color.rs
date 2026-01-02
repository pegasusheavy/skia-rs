#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{premultiply_color, Color, Color4f, Scalar};

#[derive(Debug, Arbitrary)]
struct ColorInput {
    // 32-bit color components
    a: u8,
    r: u8,
    g: u8,
    b: u8,
    // Float color components
    r_f: Scalar,
    g_f: Scalar,
    b_f: Scalar,
    a_f: Scalar,
    // Interpolation factor
    t: Scalar,
}

fuzz_target!(|input: ColorInput| {
    // Test Color from ARGB
    let color = Color::from_argb(input.a, input.r, input.g, input.b);

    // Verify component extraction matches input
    assert_eq!(color.alpha(), input.a);
    assert_eq!(color.red(), input.r);
    assert_eq!(color.green(), input.g);
    assert_eq!(color.blue(), input.b);

    // Test Color from RGB (should be opaque)
    let rgb_color = Color::from_rgb(input.r, input.g, input.b);
    assert_eq!(rgb_color.alpha(), 255);
    assert_eq!(rgb_color.red(), input.r);
    assert_eq!(rgb_color.green(), input.g);
    assert_eq!(rgb_color.blue(), input.b);

    // Test conversion to Color4f
    let color4f = color.to_color4f();
    assert!(color4f.r >= 0.0 && color4f.r <= 1.0);
    assert!(color4f.g >= 0.0 && color4f.g <= 1.0);
    assert!(color4f.b >= 0.0 && color4f.b <= 1.0);
    assert!(color4f.a >= 0.0 && color4f.a <= 1.0);

    // Test round-trip conversion
    let back = color4f.to_color();
    // Allow for rounding differences
    assert!((back.alpha() as i16 - input.a as i16).abs() <= 1);
    assert!((back.red() as i16 - input.r as i16).abs() <= 1);
    assert!((back.green() as i16 - input.g as i16).abs() <= 1);
    assert!((back.blue() as i16 - input.b as i16).abs() <= 1);

    // Test premultiplication
    let premul = premultiply_color(color);
    // Premultiplied components should be <= alpha
    assert!(premul.red() <= premul.alpha() || input.a == 0);
    assert!(premul.green() <= premul.alpha() || input.a == 0);
    assert!(premul.blue() <= premul.alpha() || input.a == 0);

    // Test Color4f creation with arbitrary floats
    if input.r_f.is_finite() && input.g_f.is_finite() &&
       input.b_f.is_finite() && input.a_f.is_finite() {
        let color4f_arb = Color4f::new(input.r_f, input.g_f, input.b_f, input.a_f);

        // Conversion to Color should clamp values
        let clamped = color4f_arb.to_color();
        let _ = clamped; // Just ensure no panic

        // Test premultiplication
        let premul4f = color4f_arb.premul();
        let _ = premul4f;
    }

    // Test Color4f lerp
    if input.t.is_finite() {
        let c1 = Color4f::new(0.0, 0.0, 0.0, 1.0);
        let c2 = Color4f::new(1.0, 1.0, 1.0, 1.0);
        let lerped = c1.lerp(&c2, input.t);

        if input.t >= 0.0 && input.t <= 1.0 {
            // Result should be between the two colors
            assert!(lerped.r >= 0.0 - 1e-6 && lerped.r <= 1.0 + 1e-6);
            assert!(lerped.g >= 0.0 - 1e-6 && lerped.g <= 1.0 + 1e-6);
            assert!(lerped.b >= 0.0 - 1e-6 && lerped.b <= 1.0 + 1e-6);
            assert!(lerped.a >= 0.0 - 1e-6 && lerped.a <= 1.0 + 1e-6);
        }
    }

    // Test standard colors
    let _ = Color::BLACK;
    let _ = Color::WHITE;
    let _ = Color::RED;
    let _ = Color::GREEN;
    let _ = Color::BLUE;
    let _ = Color::TRANSPARENT;
});
