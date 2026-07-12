//! Blend modes for compositing.

use skia_rs_core::Color4f;

/// Porter-Duff and other blend modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlendMode {
    // Porter-Duff modes
    /// Clear destination.
    Clear = 0,
    /// Source only.
    Src,
    /// Destination only.
    Dst,
    /// Source over destination (default).
    #[default]
    SrcOver,
    /// Destination over source.
    DstOver,
    /// Source where destination exists.
    SrcIn,
    /// Destination where source exists.
    DstIn,
    /// Source where destination is empty.
    SrcOut,
    /// Destination where source is empty.
    DstOut,
    /// Source on top of destination.
    SrcATop,
    /// Destination on top of source.
    DstATop,
    /// XOR of source and destination.
    Xor,
    /// Sum of source and destination.
    Plus,
    /// Product of source and destination.
    Modulate,

    // Separable blend modes
    /// Screen blend.
    Screen,
    /// Overlay blend.
    Overlay,
    /// Darken (minimum).
    Darken,
    /// Lighten (maximum).
    Lighten,
    /// Color dodge.
    ColorDodge,
    /// Color burn.
    ColorBurn,
    /// Hard light.
    HardLight,
    /// Soft light.
    SoftLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
    /// Multiply.
    Multiply,

    // Non-separable blend modes
    /// Hue blend.
    Hue,
    /// Saturation blend.
    Saturation,
    /// Color blend.
    Color,
    /// Luminosity blend.
    Luminosity,
}

impl BlendMode {
    /// Get the name of the blend mode.
    pub const fn name(&self) -> &'static str {
        match self {
            BlendMode::Clear => "Clear",
            BlendMode::Src => "Src",
            BlendMode::Dst => "Dst",
            BlendMode::SrcOver => "SrcOver",
            BlendMode::DstOver => "DstOver",
            BlendMode::SrcIn => "SrcIn",
            BlendMode::DstIn => "DstIn",
            BlendMode::SrcOut => "SrcOut",
            BlendMode::DstOut => "DstOut",
            BlendMode::SrcATop => "SrcATop",
            BlendMode::DstATop => "DstATop",
            BlendMode::Xor => "Xor",
            BlendMode::Plus => "Plus",
            BlendMode::Modulate => "Modulate",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::ColorDodge => "ColorDodge",
            BlendMode::ColorBurn => "ColorBurn",
            BlendMode::HardLight => "HardLight",
            BlendMode::SoftLight => "SoftLight",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Multiply => "Multiply",
            BlendMode::Hue => "Hue",
            BlendMode::Saturation => "Saturation",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
        }
    }

    /// Create a blend mode from a u8 value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(BlendMode::Clear),
            1 => Some(BlendMode::Src),
            2 => Some(BlendMode::Dst),
            3 => Some(BlendMode::SrcOver),
            4 => Some(BlendMode::DstOver),
            5 => Some(BlendMode::SrcIn),
            6 => Some(BlendMode::DstIn),
            7 => Some(BlendMode::SrcOut),
            8 => Some(BlendMode::DstOut),
            9 => Some(BlendMode::SrcATop),
            10 => Some(BlendMode::DstATop),
            11 => Some(BlendMode::Xor),
            12 => Some(BlendMode::Plus),
            13 => Some(BlendMode::Modulate),
            14 => Some(BlendMode::Screen),
            15 => Some(BlendMode::Overlay),
            16 => Some(BlendMode::Darken),
            17 => Some(BlendMode::Lighten),
            18 => Some(BlendMode::ColorDodge),
            19 => Some(BlendMode::ColorBurn),
            20 => Some(BlendMode::HardLight),
            21 => Some(BlendMode::SoftLight),
            22 => Some(BlendMode::Difference),
            23 => Some(BlendMode::Exclusion),
            24 => Some(BlendMode::Multiply),
            25 => Some(BlendMode::Hue),
            26 => Some(BlendMode::Saturation),
            27 => Some(BlendMode::Color),
            28 => Some(BlendMode::Luminosity),
            _ => None,
        }
    }

    /// Check if this is a Porter-Duff mode.
    #[inline]
    pub fn is_porter_duff(&self) -> bool {
        (*self as u8) <= (BlendMode::Modulate as u8)
    }

    /// Check if this is a separable blend mode.
    #[inline]
    pub fn is_separable(&self) -> bool {
        let v = *self as u8;
        v >= (BlendMode::Screen as u8) && v <= (BlendMode::Multiply as u8)
    }

    /// Check if this is a non-separable blend mode.
    #[inline]
    pub fn is_non_separable(&self) -> bool {
        let v = *self as u8;
        v >= (BlendMode::Hue as u8) && v <= (BlendMode::Luminosity as u8)
    }

    /// Apply this blend mode to src over dst, producing the composited color.
    ///
    /// Inputs and output are premultiplied linear-space Color4f values.
    /// All color components and alpha are in the [0.0, 1.0] range.
    ///
    /// The separable RGB modes apply the formula per-channel.
    /// Non-separable modes (Hue, Saturation, Color, Luminosity) operate
    /// on the RGB color as a whole, converting through HSL space.
    pub fn apply(&self, src: Color4f, dst: Color4f) -> Color4f {
        // Premultiplied alpha compositing formulas.
        // For Porter-Duff modes: result = Fa*src + Fb*dst, then clipped.
        // For separable blend modes: RGB computed per-channel, alpha via src-over.
        // For non-separable: HSL blending.

        match self {
            // --- Porter-Duff modes ---
            BlendMode::Clear => Color4f::new(0.0, 0.0, 0.0, 0.0),
            BlendMode::Src => src,
            BlendMode::Dst => dst,
            BlendMode::SrcOver => porter_duff(src, dst, 1.0, 1.0 - src.a),
            BlendMode::DstOver => porter_duff(src, dst, 1.0 - dst.a, 1.0),
            BlendMode::SrcIn => porter_duff(src, dst, dst.a, 0.0),
            BlendMode::DstIn => porter_duff(src, dst, 0.0, src.a),
            BlendMode::SrcOut => porter_duff(src, dst, 1.0 - dst.a, 0.0),
            BlendMode::DstOut => porter_duff(src, dst, 0.0, 1.0 - src.a),
            BlendMode::SrcATop => porter_duff(src, dst, dst.a, 1.0 - src.a),
            BlendMode::DstATop => porter_duff(src, dst, 1.0 - dst.a, src.a),
            BlendMode::Xor => porter_duff(src, dst, 1.0 - dst.a, 1.0 - src.a),

            // --- Arithmetic ---
            BlendMode::Plus => {
                let r = (src.r + dst.r).min(1.0);
                let g = (src.g + dst.g).min(1.0);
                let b = (src.b + dst.b).min(1.0);
                let a = (src.a + dst.a).min(1.0);
                Color4f::new(r, g, b, a)
            }
            BlendMode::Modulate => {
                Color4f::new(src.r * dst.r, src.g * dst.g, src.b * dst.b, src.a * dst.a)
            }

            // --- Separable blend modes (RGB via f, alpha via src-over) ---
            BlendMode::Screen => separable_blend(src, dst, |s, d| s + d - s * d),
            // Upstream BLEND_MODE(overlay):
            //   s*inv(da) + d*inv(sa)
            //     + if_then_else(two(d) <= da, two(s*d), sa*da - two((da-d)*(sa-s)))
            BlendMode::Overlay => separable_blend(src, dst, |s, d| {
                s * (1.0 - dst.a)
                    + d * (1.0 - src.a)
                    + if 2.0 * d <= dst.a {
                        2.0 * s * d
                    } else {
                        src.a * dst.a - 2.0 * (dst.a - d) * (src.a - s)
                    }
            }),
            BlendMode::Darken => separable_blend(src, dst, |s, d| {
                (s * dst.a).min(d * src.a) + s * (1.0 - dst.a) + d * (1.0 - src.a)
            }),
            BlendMode::Lighten => separable_blend(src, dst, |s, d| {
                (s * dst.a).max(d * src.a) + s * (1.0 - dst.a) + d * (1.0 - src.a)
            }),
            // Upstream BLEND_MODE(colordodge):
            //   d == 0  -> s*inv(da)
            //   s == sa -> s + d*inv(sa)
            //   else    -> sa*min(da, (d*sa)/(sa-s)) + s*inv(da) + d*inv(sa)
            BlendMode::ColorDodge => separable_blend(src, dst, |s, d| {
                let (sa, da) = (src.a, dst.a);
                if d == 0.0 {
                    s * (1.0 - da)
                } else if s == sa {
                    s + d * (1.0 - sa)
                } else {
                    sa * da.min((d * sa) / (sa - s)) + s * (1.0 - da) + d * (1.0 - sa)
                }
            }),
            // Upstream BLEND_MODE(colorburn):
            //   d == da -> d + s*inv(da)
            //   s == 0  -> d*inv(sa)
            //   else    -> sa*(da - min(da, (da-d)*sa/s)) + s*inv(da) + d*inv(sa)
            BlendMode::ColorBurn => separable_blend(src, dst, |s, d| {
                let (sa, da) = (src.a, dst.a);
                if d == da {
                    d + s * (1.0 - da)
                } else if s == 0.0 {
                    d * (1.0 - sa)
                } else {
                    sa * (da - da.min((da - d) * sa / s)) + s * (1.0 - da) + d * (1.0 - sa)
                }
            }),
            // Upstream BLEND_MODE(hardlight):
            //   s*inv(da) + d*inv(sa)
            //     + if_then_else(two(s) <= sa, two(s*d), sa*da - two((da-d)*(sa-s)))
            BlendMode::HardLight => separable_blend(src, dst, |s, d| {
                s * (1.0 - dst.a)
                    + d * (1.0 - src.a)
                    + if 2.0 * s <= src.a {
                        2.0 * s * d
                    } else {
                        src.a * dst.a - 2.0 * (src.a - s) * (dst.a - d)
                    }
            }),
            BlendMode::SoftLight => {
                separable_blend(src, dst, |s, d| soft_light_channel(s, d, src.a, dst.a))
            }
            BlendMode::Difference => {
                separable_blend(src, dst, |s, d| s + d - 2.0 * (s * dst.a).min(d * src.a))
            }
            BlendMode::Exclusion => separable_blend(src, dst, |s, d| s + d - 2.0 * s * d),
            BlendMode::Multiply => separable_blend(src, dst, |s, d| {
                s * (1.0 - dst.a) + d * (1.0 - src.a) + s * d
            }),

            // --- Non-separable modes ---
            BlendMode::Hue => non_separable_blend(src, dst, NonSepMode::Hue),
            BlendMode::Saturation => non_separable_blend(src, dst, NonSepMode::Saturation),
            BlendMode::Color => non_separable_blend(src, dst, NonSepMode::Color),
            BlendMode::Luminosity => non_separable_blend(src, dst, NonSepMode::Luminosity),
        }
    }
}

fn porter_duff(src: Color4f, dst: Color4f, fa: f32, fb: f32) -> Color4f {
    Color4f::new(
        (src.r * fa + dst.r * fb).clamp(0.0, 1.0),
        (src.g * fa + dst.g * fb).clamp(0.0, 1.0),
        (src.b * fa + dst.b * fb).clamp(0.0, 1.0),
        (src.a * fa + dst.a * fb).clamp(0.0, 1.0),
    )
}

/// Apply a per-channel separable blend function `f(src_channel, dst_channel)`.
///
/// For separable blend modes, alpha follows src-over (a = sa + da - sa*da)
/// and the RGB channels apply `f` to the premultiplied channels then add
/// the standard Porter-Duff source-over weighting.
fn separable_blend<F>(src: Color4f, dst: Color4f, f: F) -> Color4f
where
    F: Fn(f32, f32) -> f32,
{
    let a = src.a + dst.a - src.a * dst.a;
    Color4f::new(
        f(src.r, dst.r).clamp(0.0, 1.0),
        f(src.g, dst.g).clamp(0.0, 1.0),
        f(src.b, dst.b).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
}

fn soft_light_channel(s: f32, d: f32, sa: f32, da: f32) -> f32 {
    // Matches upstream BLEND_MODE(softlight) in SkRasterPipeline_opts.h:
    //   m  = da > 0 ? d/da : 0
    //   darkSrc = d*(sa + (2s - sa)*(1 - m))              // 2s <= sa
    //   darkDst = (m4*m4 + m4)*(m - 1) + 7m               // == 16m^3 - 12m^2 + 3m
    //   liteDst = sqrt(m) - m
    //   liteSrc = d*sa + da*(2s - sa) * (4d <= da ? darkDst : liteDst)
    //   result  = s*inv(da) + d*inv(sa) + (2s <= sa ? darkSrc : liteSrc)
    let m = if da > 0.0 { d / da } else { 0.0 };
    let s2 = 2.0 * s;
    let m4 = 4.0 * m;

    let dark_src = d * (sa + (s2 - sa) * (1.0 - m));
    let dark_dst = (m4 * m4 + m4) * (m - 1.0) + 7.0 * m;
    let lite_dst = m.sqrt() - m;
    let lite_src = d * sa + da * (s2 - sa) * if 4.0 * d <= da { dark_dst } else { lite_dst };

    s * (1.0 - da) + d * (1.0 - sa) + if s2 <= sa { dark_src } else { lite_src }
}

#[derive(Clone, Copy)]
enum NonSepMode {
    Hue,
    Saturation,
    Color,
    Luminosity,
}

/// Non-separable blend: operate on the RGB triplet as a whole.
/// Operates in unpremultiplied space then re-premultiplies.
fn non_separable_blend(src: Color4f, dst: Color4f, mode: NonSepMode) -> Color4f {
    // Unpremultiply to linear RGB
    let s_rgb = if src.a > 0.0 {
        [src.r / src.a, src.g / src.a, src.b / src.a]
    } else {
        [0.0, 0.0, 0.0]
    };
    let d_rgb = if dst.a > 0.0 {
        [dst.r / dst.a, dst.g / dst.a, dst.b / dst.a]
    } else {
        [0.0, 0.0, 0.0]
    };

    let blended = match mode {
        NonSepMode::Hue => set_lum(set_sat(s_rgb, sat(d_rgb)), lum(d_rgb)),
        NonSepMode::Saturation => set_lum(set_sat(d_rgb, sat(s_rgb)), lum(d_rgb)),
        NonSepMode::Color => set_lum(s_rgb, lum(d_rgb)),
        NonSepMode::Luminosity => set_lum(d_rgb, lum(s_rgb)),
    };

    // Composite over dst using src-over
    let a = src.a + dst.a - src.a * dst.a;
    let r = blended[0] * src.a * dst.a + src.r * (1.0 - dst.a) + dst.r * (1.0 - src.a);
    let g = blended[1] * src.a * dst.a + src.g * (1.0 - dst.a) + dst.g * (1.0 - src.a);
    let b = blended[2] * src.a * dst.a + src.b * (1.0 - dst.a) + dst.b * (1.0 - src.a);
    Color4f::new(
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
}

// W3C-compatible HSL helpers for non-separable blend modes.

fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn sat(c: [f32; 3]) -> f32 {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    max - min
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        for i in 0..3 {
            out[i] = l + (out[i] - l) * l / (l - n).max(1e-7);
        }
    }
    if x > 1.0 {
        for i in 0..3 {
            out[i] = l + (out[i] - l) * (1.0 - l) / (x - l).max(1e-7);
        }
    }
    out
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    // Sort channels to find min, mid, max
    let mut idx = [0, 1, 2];
    idx.sort_by(|&a, &b| c[a].partial_cmp(&c[b]).unwrap_or(std::cmp::Ordering::Equal));
    let min_i = idx[0];
    let mid_i = idx[1];
    let max_i = idx[2];

    let mut out = [0.0_f32; 3];
    if c[max_i] > c[min_i] {
        out[mid_i] = (c[mid_i] - c[min_i]) * s / (c[max_i] - c[min_i]);
        out[max_i] = s;
    } else {
        out[mid_i] = 0.0;
        out[max_i] = 0.0;
    }
    out[min_i] = 0.0;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(r: f32, g: f32, b: f32, a: f32) -> Color4f {
        Color4f::new(r, g, b, a)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn colors_close(a: Color4f, b: Color4f) -> bool {
        close(a.r, b.r) && close(a.g, b.g) && close(a.b, b.b) && close(a.a, b.a)
    }

    #[test]
    fn test_blend_clear() {
        let src = c(1.0, 0.5, 0.25, 0.8);
        let dst = c(0.2, 0.3, 0.4, 1.0);
        assert!(colors_close(
            BlendMode::Clear.apply(src, dst),
            c(0.0, 0.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn test_blend_src() {
        let src = c(1.0, 0.5, 0.25, 0.8);
        let dst = c(0.2, 0.3, 0.4, 1.0);
        assert!(colors_close(BlendMode::Src.apply(src, dst), src));
    }

    #[test]
    fn test_blend_dst() {
        let src = c(1.0, 0.5, 0.25, 0.8);
        let dst = c(0.2, 0.3, 0.4, 1.0);
        assert!(colors_close(BlendMode::Dst.apply(src, dst), dst));
    }

    #[test]
    fn test_blend_src_over_opaque() {
        // Opaque src over any dst should equal src
        let src = c(0.5, 0.5, 0.5, 1.0);
        let dst = c(0.2, 0.3, 0.4, 1.0);
        assert!(colors_close(BlendMode::SrcOver.apply(src, dst), src));
    }

    #[test]
    fn test_blend_src_over_half_alpha() {
        // Half-alpha src over opaque dst = half src + half dst (premultiplied)
        let src = c(0.4, 0.0, 0.0, 0.5); // premul: 0.4 = 0.8 * 0.5
        let dst = c(0.0, 0.4, 0.0, 1.0); // premul: 0.4 = 0.4 * 1.0
        let result = BlendMode::SrcOver.apply(src, dst);
        // R = src.r + dst.r * (1 - src.a) = 0.4 + 0 = 0.4
        // G = 0 + 0.4 * 0.5 = 0.2
        // B = 0
        // A = 0.5 + 1.0 * 0.5 = 1.0
        assert!(close(result.r, 0.4), "r was {}", result.r);
        assert!(close(result.g, 0.2), "g was {}", result.g);
        assert!(close(result.b, 0.0), "b was {}", result.b);
        assert!(close(result.a, 1.0), "a was {}", result.a);
    }

    #[test]
    fn test_blend_plus() {
        let src = c(0.3, 0.4, 0.5, 0.5);
        let dst = c(0.2, 0.1, 0.4, 0.5);
        let result = BlendMode::Plus.apply(src, dst);
        assert!(close(result.r, 0.5));
        assert!(close(result.g, 0.5));
        assert!(close(result.b, 0.9));
        assert!(close(result.a, 1.0));
    }

    #[test]
    fn test_blend_plus_clamps() {
        let src = c(0.8, 0.8, 0.8, 0.8);
        let dst = c(0.8, 0.8, 0.8, 0.8);
        let result = BlendMode::Plus.apply(src, dst);
        assert!(close(result.r, 1.0));
        assert!(close(result.a, 1.0));
    }

    #[test]
    fn test_blend_modulate() {
        let src = c(0.5, 0.5, 0.5, 1.0);
        let dst = c(0.5, 0.5, 0.5, 1.0);
        let result = BlendMode::Modulate.apply(src, dst);
        assert!(close(result.r, 0.25));
        assert!(close(result.g, 0.25));
        assert!(close(result.b, 0.25));
    }

    #[test]
    fn test_blend_screen() {
        // Screen: 1 - (1-s)(1-d) = s + d - sd
        let src = c(0.5, 0.0, 0.0, 1.0);
        let dst = c(0.0, 0.5, 0.0, 1.0);
        let result = BlendMode::Screen.apply(src, dst);
        assert!(close(result.r, 0.5));
        assert!(close(result.g, 0.5));
    }

    // --- Conformance regression tests against SkRasterPipeline_opts.h ---
    //
    // All separable blend-mode expectations below are hand-evaluated from the
    // upstream BLEND_MODE(...) channel formulas on premultiplied inputs.

    #[test]
    fn test_overlay_adds_porter_duff_edge_terms() {
        // Upstream: s*inv(da) + d*inv(sa) + branch.
        // s = 0.4, sa = 0.5, d = 0.1, da = 0.5.
        // Branch: 2d = 0.2 <= da -> 2sd = 0.08. Edge: 0.4*0.5 + 0.1*0.5 = 0.25.
        let src = c(0.4, 0.4, 0.4, 0.5);
        let dst = c(0.1, 0.1, 0.1, 0.5);
        let r = BlendMode::Overlay.apply(src, dst);
        assert!(close(r.r, 0.33), "overlay dark-dst channel was {}", r.r);

        // Other branch: s = 0.1, d = 0.4 -> 2d > da:
        // sa*da - 2*(da-d)*(sa-s) = 0.25 - 2*0.1*0.4 = 0.17; + edge 0.25 = 0.42.
        let src = c(0.1, 0.1, 0.1, 0.5);
        let dst = c(0.4, 0.4, 0.4, 0.5);
        let r = BlendMode::Overlay.apply(src, dst);
        assert!(close(r.r, 0.42), "overlay lite-dst channel was {}", r.r);
    }

    #[test]
    fn test_hardlight_adds_porter_duff_edge_terms() {
        // s = 0.4, sa = 0.5, d = 0.1, da = 0.5. 2s = 0.8 > sa ->
        // sa*da - 2*(sa-s)*(da-d) = 0.25 - 2*0.1*0.4 = 0.17; + edge 0.25 = 0.42.
        let src = c(0.4, 0.4, 0.4, 0.5);
        let dst = c(0.1, 0.1, 0.1, 0.5);
        let r = BlendMode::HardLight.apply(src, dst);
        assert!(close(r.r, 0.42), "hardlight lite-src channel was {}", r.r);
    }

    #[test]
    fn test_colordodge_general_branch_da_normalization() {
        // Upstream: sa*min(da, d*sa/(sa-s)) + s*inv(da) + d*inv(sa).
        // s = 0.2, sa = 0.5, d = 0.3, da = 0.8:
        // d*sa/(sa-s) = 0.5; min(0.8, 0.5) = 0.5; sa*0.5 = 0.25.
        // Edge: 0.2*0.2 + 0.3*0.5 = 0.19. Total = 0.44.
        let src = c(0.2, 0.2, 0.2, 0.5);
        let dst = c(0.3, 0.3, 0.3, 0.8);
        let r = BlendMode::ColorDodge.apply(src, dst);
        assert!(close(r.r, 0.44), "colordodge general channel was {}", r.r);
    }

    #[test]
    fn test_colorburn_general_branch_da_normalization() {
        // Upstream: sa*(da - min(da, (da-d)*sa/s)) + s*inv(da) + d*inv(sa).
        // s = 0.4, sa = 0.5, d = 0.3, da = 0.8:
        // (da-d)*sa/s = 0.625; min(0.8, 0.625) = 0.625; sa*(0.8-0.625) = 0.0875.
        // Edge: 0.4*0.2 + 0.3*0.5 = 0.23. Total = 0.3175.
        let src = c(0.4, 0.4, 0.4, 0.5);
        let dst = c(0.3, 0.3, 0.3, 0.8);
        let r = BlendMode::ColorBurn.apply(src, dst);
        assert!(close(r.r, 0.3175), "colorburn general channel was {}", r.r);
    }

    #[test]
    fn test_softlight_matches_upstream_polynomial() {
        // Case 1 (dark src): s = 0.2, sa = 0.5 (2s <= sa), d = 0.1, da = 0.8.
        // m = 0.125. darkSrc = d*(sa + (2s-sa)*(1-m)) = 0.1*(0.5 - 0.1*0.875)
        //          = 0.04125. Edge = 0.2*0.2 + 0.1*0.5 = 0.09. Total 0.13125.
        let r = BlendMode::SoftLight.apply(c(0.2, 0.2, 0.2, 0.5), c(0.1, 0.1, 0.1, 0.8));
        assert!(close(r.r, 0.13125), "softlight dark-src was {}", r.r);

        // Case 2 (light src, dark dst): s = 0.4, sa = 0.5, d = 0.1, da = 0.8.
        // m = 0.125, m4 = 0.5. darkDst = (m4^2 + m4)*(m-1) + 7m
        //   = 0.75*(-0.875) + 0.875 = 0.21875  (== 16m^3 - 12m^2 + 3m).
        // liteSrc = d*sa + da*(2s-sa)*darkDst = 0.05 + 0.8*0.3*0.21875 = 0.1025.
        // Edge = 0.4*0.2 + 0.1*0.5 = 0.13. Total 0.2325.
        let r = BlendMode::SoftLight.apply(c(0.4, 0.4, 0.4, 0.5), c(0.1, 0.1, 0.1, 0.8));
        assert!(close(r.r, 0.2325), "softlight dark-dst was {}", r.r);

        // Case 3 (light src, light dst): s = 0.4, sa = 0.5, d = 0.3, da = 0.8.
        // m = 0.375. liteDst = sqrt(m) - m = 0.2373724.
        // liteSrc = 0.15 + 0.8*0.3*liteDst = 0.2069694. Edge = 0.23. Total 0.4369694.
        let r = BlendMode::SoftLight.apply(c(0.4, 0.4, 0.4, 0.5), c(0.3, 0.3, 0.3, 0.8));
        assert!(close(r.r, 0.4369694), "softlight lite-dst was {}", r.r);
    }

    #[test]
    fn test_separable_modes_over_transparent_dst_return_src() {
        // With da = 0 every upstream separable formula reduces to s.
        let src = c(0.4, 0.3, 0.2, 0.5);
        let dst = c(0.0, 0.0, 0.0, 0.0);
        for i in 14..=24u8 {
            let mode = BlendMode::from_u8(i).unwrap();
            let r = mode.apply(src, dst);
            assert!(
                colors_close(r, src),
                "{:?} over transparent dst was {:?}, want src {:?}",
                mode,
                r,
                src
            );
        }
    }

    #[test]
    fn test_separable_modes_with_transparent_src_return_dst() {
        // With sa = 0 (and thus s = 0) every upstream separable formula
        // reduces to d.
        let src = c(0.0, 0.0, 0.0, 0.0);
        let dst = c(0.4, 0.3, 0.2, 0.5);
        for i in 14..=24u8 {
            let mode = BlendMode::from_u8(i).unwrap();
            let r = mode.apply(src, dst);
            assert!(
                colors_close(r, dst),
                "{:?} with transparent src was {:?}, want dst {:?}",
                mode,
                r,
                dst
            );
        }
    }

    #[test]
    fn test_all_modes_produce_finite_output() {
        // Regression: no mode should produce NaN or infinity
        let src = c(0.3, 0.7, 0.2, 0.8);
        let dst = c(0.6, 0.4, 0.9, 0.5);
        // Iterate via from_u8 on range
        for i in 0..29u8 {
            if let Some(mode) = BlendMode::from_u8(i) {
                let r = mode.apply(src, dst);
                assert!(r.r.is_finite(), "{:?} produced NaN r", mode);
                assert!(r.g.is_finite(), "{:?} produced NaN g", mode);
                assert!(r.b.is_finite(), "{:?} produced NaN b", mode);
                assert!(r.a.is_finite(), "{:?} produced NaN a", mode);
            }
        }
    }
}
