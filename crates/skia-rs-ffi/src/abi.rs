//! Binary ABI Compatibility Layer
//!
//! This module provides C-ABI struct layouts for skia-rs's own C API. Plain
//! POD types with no upstream indirect/ref-counted members (points, sizes,
//! rects, matrices, colors) are laid out identically to their upstream
//! `Sk*` counterparts and are safe to treat as binary-interchangeable with
//! real Skia. Types that embed pointers to non-POD upstream state (e.g.
//! [`SkImageInfoABI`]'s `color_space` field, which stands in for an
//! `sk_sp<SkColorSpace>`) are **not** byte-for-byte compatible with
//! upstream Skia's internal layout — see each type's own docs for the
//! specifics.
//!
//! # Guarantees
//!
//! The types in this module guarantee:
//! - Stable size/layout for the plain POD types across skia-rs minor
//!   version bumps (verified by `size_of`/`align_of` assertions, checked
//!   per target pointer width where a type contains a pointer field)
//! - Compatible `extern "C"` calling conventions for all FFI functions
//!
//! # Usage
//!
//! ```c
//! // These POD types are binary-compatible with Skia's C API
//! sk_point_t point = { 10.0f, 20.0f };
//! sk_rect_t rect = { 0.0f, 0.0f, 100.0f, 100.0f };
//! sk_matrix_t identity = SK_MATRIX_IDENTITY;
//! ```

use std::ffi::c_void;

// =============================================================================
// ABI Version Information
// =============================================================================

/// ABI version major number
pub const SK_ABI_VERSION_MAJOR: u32 = 1;

/// ABI version minor number
pub const SK_ABI_VERSION_MINOR: u32 = 0;

/// ABI version patch number
pub const SK_ABI_VERSION_PATCH: u32 = 0;

/// Get the ABI version as a packed 32-bit integer
#[unsafe(no_mangle)]
pub const extern "C" fn sk_abi_get_version() -> u32 {
    (SK_ABI_VERSION_MAJOR << 16) | (SK_ABI_VERSION_MINOR << 8) | SK_ABI_VERSION_PATCH
}

/// Check if the ABI version is compatible
#[unsafe(no_mangle)]
#[allow(
    clippy::absurd_extreme_comparisons,
    reason = "minor <= SK_ABI_VERSION_MINOR is intentional forward-compatible semver logic; it only looks absurd because SK_ABI_VERSION_MINOR is currently 0, and will behave correctly once the constant is bumped"
)]
pub const extern "C" fn sk_abi_is_compatible(major: u32, minor: u32) -> bool {
    major == SK_ABI_VERSION_MAJOR && minor <= SK_ABI_VERSION_MINOR
}

// =============================================================================
// Core Types - Binary Compatible with Skia
// =============================================================================

/// Binary-compatible 2D point (matches `SkPoint` exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkPointABI {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
}

const _: () = assert!(std::mem::size_of::<SkPointABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkPointABI>() == 4);

/// Binary-compatible integer point (matches `SkIPoint` exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkIPointABI {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

const _: () = assert!(std::mem::size_of::<SkIPointABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkIPointABI>() == 4);

/// Binary-compatible 2D size (matches `SkSize` exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkSizeABI {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

const _: () = assert!(std::mem::size_of::<SkSizeABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkSizeABI>() == 4);

/// Binary-compatible integer size (matches `SkISize` exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkISizeABI {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

const _: () = assert!(std::mem::size_of::<SkISizeABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkISizeABI>() == 4);

/// Binary-compatible rectangle (matches `SkRect` exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkRectABI {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
}

const _: () = assert!(std::mem::size_of::<SkRectABI>() == 16);
const _: () = assert!(std::mem::align_of::<SkRectABI>() == 4);

/// Binary-compatible integer rectangle (matches `SkIRect` exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkIRectABI {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

const _: () = assert!(std::mem::size_of::<SkIRectABI>() == 16);
const _: () = assert!(std::mem::align_of::<SkIRectABI>() == 4);

/// Binary-compatible 3x3 matrix (matches `SkMatrix` exactly)
///
/// Layout: [scaleX, skewX, transX, skewY, scaleY, transY, persp0, persp1, persp2]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkMatrixABI {
    /// Row-major 3x3 matrix values.
    pub values: [f32; 9],
}

const _: () = assert!(std::mem::size_of::<SkMatrixABI>() == 36);
const _: () = assert!(std::mem::align_of::<SkMatrixABI>() == 4);

impl Default for SkMatrixABI {
    fn default() -> Self {
        Self::identity()
    }
}

impl SkMatrixABI {
    /// Identity matrix constant
    #[must_use] 
    pub const fn identity() -> Self {
        Self {
            values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Binary-compatible 4x4 matrix (matches SkMatrix44/SkM44 exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkMatrix44ABI {
    /// Row-major 4x4 matrix values.
    pub values: [f32; 16],
}

const _: () = assert!(std::mem::size_of::<SkMatrix44ABI>() == 64);
const _: () = assert!(std::mem::align_of::<SkMatrix44ABI>() == 4);

impl Default for SkMatrix44ABI {
    fn default() -> Self {
        Self::identity()
    }
}

impl SkMatrix44ABI {
    /// Identity matrix constant
    #[must_use] 
    pub const fn identity() -> Self {
        Self {
            values: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

/// Binary-compatible ARGB color (matches `SkColor` exactly)
pub type SkColorABI = u32;

/// Binary-compatible ARGB color with premultiplied alpha (matches `SkPMColor`)
pub type SkPMColorABI = u32;

/// Binary-compatible 4-component color (matches `SkColor4f` exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkColor4fABI {
    /// Red channel, 0.0-1.0.
    pub r: f32,
    /// Green channel, 0.0-1.0.
    pub g: f32,
    /// Blue channel, 0.0-1.0.
    pub b: f32,
    /// Alpha channel, 0.0-1.0.
    pub a: f32,
}

const _: () = assert!(std::mem::size_of::<SkColor4fABI>() == 16);
const _: () = assert!(std::mem::align_of::<SkColor4fABI>() == 4);

// =============================================================================
// Image Info - Binary Compatible
// =============================================================================

/// Binary-compatible color type (matches `SkColorType` exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkColorTypeABI {
    /// Unknown color type.
    #[default]
    Unknown = 0,
    /// 8-bit alpha only.
    Alpha8 = 1,
    /// 5-6-5 RGB, 16 bits per pixel.
    Rgb565 = 2,
    /// 4-4-4-4 ARGB, 16 bits per pixel.
    Argb4444 = 3,
    /// 8-8-8-8 RGBA, 32 bits per pixel.
    Rgba8888 = 4,
    /// 8-8-8-8 RGB (X unused), 32 bits per pixel.
    Rgb888x = 5,
    /// 8-8-8-8 BGRA, 32 bits per pixel.
    Bgra8888 = 6,
    /// 10-10-10-2 RGBA, 32 bits per pixel.
    Rgba1010102 = 7,
    /// 10-10-10-2 BGRA, 32 bits per pixel.
    Bgra1010102 = 8,
    /// 10-10-10-x RGB, 32 bits per pixel.
    Rgb101010x = 9,
    /// 10-10-10-x BGR, 32 bits per pixel.
    Bgr101010x = 10,
    /// 8-bit grayscale.
    Gray8 = 11,
    /// Half-float RGBA, normalized (0.0-1.0).
    RgbaF16Norm = 12,
    /// Half-float RGBA, linear space.
    RgbaF16 = 13,
    /// Full-float RGBA.
    RgbaF32 = 14,
    /// 8-8 two-channel unorm.
    R8g8Unorm = 15,
    /// 16-bit float alpha only.
    A16Float = 16,
    /// 16-16 two-channel float.
    R16g16Float = 17,
    /// 16-bit unorm alpha only.
    A16Unorm = 18,
    /// 16-16 two-channel unorm.
    R16g16Unorm = 19,
    /// 16-16-16-16 RGBA unorm.
    R16g16b16a16Unorm = 20,
    /// 8-8-8-8 sRGB-encoded RGBA.
    Srgba8888 = 21,
    /// 8-bit single-channel unorm.
    R8Unorm = 22,
}

/// Binary-compatible alpha type (matches `SkAlphaType` exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkAlphaTypeABI {
    /// Unknown alpha type.
    #[default]
    Unknown = 0,
    /// Pixels are fully opaque.
    Opaque = 1,
    /// Alpha is premultiplied into color channels.
    Premul = 2,
    /// Alpha is not premultiplied into color channels.
    Unpremul = 3,
}

/// A simplified, C-ABI-friendly stand-in for `SkImageInfo`.
///
/// This does **not** claim byte-for-byte binary compatibility with upstream
/// `SkImageInfo` (`include/core/SkImageInfo.h`): real `SkImageInfo` embeds
/// an `SkColorInfo` holding an `sk_sp<SkColorSpace>` (a ref-counted smart
/// pointer with its own control-block layout), not a raw pointer, and its
/// field order/packing is an implementation detail not part of Skia's
/// public ABI contract. This struct only guarantees a *stable layout for
/// this crate's own C API* across skia-rs releases; treat `color_space` as
/// an opaque `SkColorSpace*` handle, not a `sk_sp` you can memcpy into
/// upstream Skia code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkImageInfoABI {
    /// Pixel width.
    pub width: i32,
    /// Pixel height.
    pub height: i32,
    /// Pixel color type.
    pub color_type: SkColorTypeABI,
    /// Pixel alpha type.
    pub alpha_type: SkAlphaTypeABI,
    /// Opaque `SkColorSpace*` handle (see struct docs); may be null.
    pub color_space: *const c_void,
}

// Size depends on the target's pointer width (`color_space` is a raw
// pointer): 24 bytes on 64-bit targets (4 x i32 fields + 4 bytes padding +
// an 8-byte pointer), 20 bytes on 32-bit targets (no padding needed before
// a 4-byte-aligned pointer field).
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<SkImageInfoABI>() == 24);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(std::mem::size_of::<SkImageInfoABI>() == 20);

impl Default for SkImageInfoABI {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            color_type: SkColorTypeABI::Unknown,
            alpha_type: SkAlphaTypeABI::Unknown,
            color_space: std::ptr::null(),
        }
    }
}

// =============================================================================
// Paint - Binary Compatible
// =============================================================================

/// Binary-compatible paint style (matches `SkPaint::Style` exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkPaintStyleABI {
    /// Fill the geometry.
    #[default]
    Fill = 0,
    /// Stroke the geometry's outline.
    Stroke = 1,
    /// Stroke and fill the geometry.
    StrokeAndFill = 2,
}

/// Binary-compatible stroke cap (matches `SkPaint::Cap` exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkStrokeCapABI {
    /// No stroke extension past the endpoint.
    #[default]
    Butt = 0,
    /// Adds a round cap past the endpoint.
    Round = 1,
    /// Adds a square cap past the endpoint.
    Square = 2,
}

/// Binary-compatible stroke join (matches `SkPaint::Join` exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkStrokeJoinABI {
    /// Extends the outer edges to a sharp point.
    #[default]
    Miter = 0,
    /// Rounds the outer join corner.
    Round = 1,
    /// Flattens the outer join corner.
    Bevel = 2,
}

/// Binary-compatible blend mode (matches `SkBlendMode` exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkBlendModeABI {
    /// Replaces destination with transparent black.
    Clear = 0,
    /// Replaces destination with source.
    Src = 1,
    /// Keeps destination, ignores source.
    Dst = 2,
    /// Source over destination (the default/normal mode).
    #[default]
    SrcOver = 3,
    /// Destination over source.
    DstOver = 4,
    /// Source, restricted to destination's alpha.
    SrcIn = 5,
    /// Destination, restricted to source's alpha.
    DstIn = 6,
    /// Source, excluded from destination's alpha.
    SrcOut = 7,
    /// Destination, excluded from source's alpha.
    DstOut = 8,
    /// Source over destination, restricted to destination's alpha.
    SrcATop = 9,
    /// Destination over source, restricted to source's alpha.
    DstATop = 10,
    /// Source and destination, excluding their overlap.
    Xor = 11,
    /// Sums source and destination.
    Plus = 12,
    /// Multiplies source and destination.
    Modulate = 13,
    /// Screens source and destination.
    Screen = 14,
    /// Overlays source and destination.
    Overlay = 15,
    /// Retains the darker of source and destination.
    Darken = 16,
    /// Retains the lighter of source and destination.
    Lighten = 17,
    /// Brightens destination to reflect source.
    ColorDodge = 18,
    /// Darkens destination to reflect source.
    ColorBurn = 19,
    /// Multiplies or screens depending on source.
    HardLight = 20,
    /// Lightens or darkens depending on source.
    SoftLight = 21,
    /// Subtracts the darker from the lighter.
    Difference = 22,
    /// Similar to `Difference`, but with lower contrast.
    Exclusion = 23,
    /// Multiplies source and destination colors.
    Multiply = 24,
    /// Uses source's hue with destination's saturation and luminosity.
    Hue = 25,
    /// Uses source's saturation with destination's hue and luminosity.
    Saturation = 26,
    /// Uses source's hue and saturation with destination's luminosity.
    Color = 27,
    /// Uses source's luminosity with destination's hue and saturation.
    Luminosity = 28,
}

// =============================================================================
// Path - Binary Compatible
// =============================================================================

/// Binary-compatible path fill type (matches `SkPathFillType` exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkPathFillTypeABI {
    /// Fills regions with a non-zero winding count.
    #[default]
    Winding = 0,
    /// Fills regions with an odd winding count.
    EvenOdd = 1,
    /// Fills regions with a zero winding count.
    InverseWinding = 2,
    /// Fills regions with an even winding count.
    InverseEvenOdd = 3,
}

/// Binary-compatible path verb (matches `SkPath::Verb` exactly)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkPathVerbABI {
    /// Starts a new contour at a point.
    Move = 0,
    /// A straight line segment.
    Line = 1,
    /// A quadratic Bezier segment.
    Quad = 2,
    /// A conic (rational quadratic) segment.
    Conic = 3,
    /// A cubic Bezier segment.
    Cubic = 4,
    /// Closes the current contour.
    Close = 5,
    /// Marks the end of path iteration.
    Done = 6,
}

// =============================================================================
// Conversion Functions
// =============================================================================

/// Convert from internal Point to ABI Point
#[unsafe(no_mangle)]
pub const extern "C" fn sk_point_to_abi(x: f32, y: f32) -> SkPointABI {
    SkPointABI { x, y }
}

/// Convert from internal Rect to ABI Rect
#[unsafe(no_mangle)]
pub const extern "C" fn sk_rect_to_abi(left: f32, top: f32, right: f32, bottom: f32) -> SkRectABI {
    SkRectABI {
        left,
        top,
        right,
        bottom,
    }
}

/// Create identity matrix
#[unsafe(no_mangle)]
pub const extern "C" fn sk_matrix_identity() -> SkMatrixABI {
    SkMatrixABI::identity()
}

/// Create identity matrix 4x4
#[unsafe(no_mangle)]
pub const extern "C" fn sk_matrix44_identity() -> SkMatrix44ABI {
    SkMatrix44ABI::identity()
}

// =============================================================================
// Type Size Verification Functions
// =============================================================================

/// Get size of `SkPointABI` (for runtime verification)
#[unsafe(no_mangle)]
pub const extern "C" fn sk_sizeof_point() -> usize {
    std::mem::size_of::<SkPointABI>()
}

/// Get size of `SkRectABI` (for runtime verification)
#[unsafe(no_mangle)]
pub const extern "C" fn sk_sizeof_rect() -> usize {
    std::mem::size_of::<SkRectABI>()
}

/// Get size of `SkMatrixABI` (for runtime verification)
#[unsafe(no_mangle)]
pub const extern "C" fn sk_sizeof_matrix() -> usize {
    std::mem::size_of::<SkMatrixABI>()
}

/// Get size of `SkImageInfoABI` (for runtime verification)
#[unsafe(no_mangle)]
pub const extern "C" fn sk_sizeof_imageinfo() -> usize {
    std::mem::size_of::<SkImageInfoABI>()
}

/// Get size of `SkColor4fABI` (for runtime verification)
#[unsafe(no_mangle)]
pub const extern "C" fn sk_sizeof_color4f() -> usize {
    std::mem::size_of::<SkColor4fABI>()
}

// =============================================================================
// ABI Validation
// =============================================================================

/// Validate that all ABI types have expected sizes
/// Returns true if all sizes match, false otherwise
#[unsafe(no_mangle)]
pub const extern "C" fn sk_abi_validate() -> bool {
    // These sizes must match Skia's C API exactly
    std::mem::size_of::<SkPointABI>() == 8
        && std::mem::size_of::<SkIPointABI>() == 8
        && std::mem::size_of::<SkSizeABI>() == 8
        && std::mem::size_of::<SkISizeABI>() == 8
        && std::mem::size_of::<SkRectABI>() == 16
        && std::mem::size_of::<SkIRectABI>() == 16
        && std::mem::size_of::<SkMatrixABI>() == 36
        && std::mem::size_of::<SkMatrix44ABI>() == 64
        && std::mem::size_of::<SkColor4fABI>() == 16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_sizes() {
        assert!(sk_abi_validate());
    }

    #[test]
    fn test_imageinfo_abi_size_is_pointer_width_aware() {
        // `SkImageInfoABI::color_space` is a raw pointer, so the struct's
        // size depends on the target's pointer width; the compile-time
        // assert next to the type definition already enforces this per
        // `#[cfg(target_pointer_width = ...)]`, this test just documents
        // the expected value for the size actually compiled in.
        let expected = if cfg!(target_pointer_width = "64") {
            24
        } else {
            20
        };
        assert_eq!(std::mem::size_of::<SkImageInfoABI>(), expected);
    }

    #[test]
    fn test_point_layout() {
        let p = SkPointABI { x: 1.0, y: 2.0 };
        let bytes: [u8; 8] = unsafe { std::mem::transmute(p) };
        // Verify x comes first
        assert_eq!(&bytes[0..4], &1.0_f32.to_ne_bytes());
        assert_eq!(&bytes[4..8], &2.0_f32.to_ne_bytes());
    }

    #[test]
    fn test_matrix_identity() {
        let m = SkMatrixABI::identity();
        assert!((m.values[0] - 1.0).abs() < f32::EPSILON); // scaleX
        assert!((m.values[4] - 1.0).abs() < f32::EPSILON); // scaleY
        assert!((m.values[8] - 1.0).abs() < f32::EPSILON); // persp2
    }

    #[test]
    fn test_version_compatibility() {
        assert!(sk_abi_is_compatible(1, 0));
        assert!(!sk_abi_is_compatible(2, 0));
    }
}
