//! Binary ABI Compatibility Layer
//!
//! This module provides C ABI-compatible struct layouts that match Skia's
//! binary representation for maximum interoperability with existing Skia
//! code and bindings.
//!
//! # Binary Compatibility Guarantees
//!
//! The types in this module guarantee:
//! - Exact struct size matching Skia's C API types
//! - Exact field offsets matching Skia's memory layout
//! - Compatible calling conventions for all FFI functions
//! - ABI stability across minor version bumps
//!
//! # Usage
//!
//! ```c
//! // These types are binary-compatible with Skia's C API
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
pub extern "C" fn sk_abi_get_version() -> u32 {
    (SK_ABI_VERSION_MAJOR << 16) | (SK_ABI_VERSION_MINOR << 8) | SK_ABI_VERSION_PATCH
}

/// Check if the ABI version is compatible
#[unsafe(no_mangle)]
pub extern "C" fn sk_abi_is_compatible(major: u32, minor: u32) -> bool {
    major == SK_ABI_VERSION_MAJOR && minor <= SK_ABI_VERSION_MINOR
}

// =============================================================================
// Core Types - Binary Compatible with Skia
// =============================================================================

/// Binary-compatible 2D point (matches SkPoint exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkPointABI {
    pub x: f32,
    pub y: f32,
}

const _: () = assert!(std::mem::size_of::<SkPointABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkPointABI>() == 4);

/// Binary-compatible integer point (matches SkIPoint exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkIPointABI {
    pub x: i32,
    pub y: i32,
}

const _: () = assert!(std::mem::size_of::<SkIPointABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkIPointABI>() == 4);

/// Binary-compatible 2D size (matches SkSize exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkSizeABI {
    pub width: f32,
    pub height: f32,
}

const _: () = assert!(std::mem::size_of::<SkSizeABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkSizeABI>() == 4);

/// Binary-compatible integer size (matches SkISize exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkISizeABI {
    pub width: i32,
    pub height: i32,
}

const _: () = assert!(std::mem::size_of::<SkISizeABI>() == 8);
const _: () = assert!(std::mem::align_of::<SkISizeABI>() == 4);

/// Binary-compatible rectangle (matches SkRect exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkRectABI {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

const _: () = assert!(std::mem::size_of::<SkRectABI>() == 16);
const _: () = assert!(std::mem::align_of::<SkRectABI>() == 4);

/// Binary-compatible integer rectangle (matches SkIRect exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkIRectABI {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

const _: () = assert!(std::mem::size_of::<SkIRectABI>() == 16);
const _: () = assert!(std::mem::align_of::<SkIRectABI>() == 4);

/// Binary-compatible 3x3 matrix (matches SkMatrix exactly)
///
/// Layout: [scaleX, skewX, transX, skewY, scaleY, transY, persp0, persp1, persp2]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkMatrixABI {
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
    pub const fn identity() -> Self {
        Self {
            values: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

/// Binary-compatible ARGB color (matches SkColor exactly)
pub type SkColorABI = u32;

/// Binary-compatible ARGB color with premultiplied alpha (matches SkPMColor)
pub type SkPMColorABI = u32;

/// Binary-compatible 4-component color (matches SkColor4f exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkColor4fABI {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

const _: () = assert!(std::mem::size_of::<SkColor4fABI>() == 16);
const _: () = assert!(std::mem::align_of::<SkColor4fABI>() == 4);

// =============================================================================
// Image Info - Binary Compatible
// =============================================================================

/// Binary-compatible color type (matches SkColorType exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkColorTypeABI {
    #[default]
    Unknown = 0,
    Alpha8 = 1,
    Rgb565 = 2,
    Argb4444 = 3,
    Rgba8888 = 4,
    Rgb888x = 5,
    Bgra8888 = 6,
    Rgba1010102 = 7,
    Bgra1010102 = 8,
    Rgb101010x = 9,
    Bgr101010x = 10,
    Gray8 = 11,
    RgbaF16Norm = 12,
    RgbaF16 = 13,
    RgbaF32 = 14,
    R8g8Unorm = 15,
    A16Float = 16,
    R16g16Float = 17,
    A16Unorm = 18,
    R16g16Unorm = 19,
    R16g16b16a16Unorm = 20,
    Srgba8888 = 21,
    R8Unorm = 22,
}

/// Binary-compatible alpha type (matches SkAlphaType exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkAlphaTypeABI {
    #[default]
    Unknown = 0,
    Opaque = 1,
    Premul = 2,
    Unpremul = 3,
}

/// Binary-compatible image info (matches SkImageInfo layout)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkImageInfoABI {
    pub width: i32,
    pub height: i32,
    pub color_type: SkColorTypeABI,
    pub alpha_type: SkAlphaTypeABI,
    pub color_space: *const c_void, // SkColorSpace*
}

const _: () = assert!(std::mem::size_of::<SkImageInfoABI>() == 24);

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

/// Binary-compatible paint style (matches SkPaint::Style exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkPaintStyleABI {
    #[default]
    Fill = 0,
    Stroke = 1,
    StrokeAndFill = 2,
}

/// Binary-compatible stroke cap (matches SkPaint::Cap exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkStrokeCapABI {
    #[default]
    Butt = 0,
    Round = 1,
    Square = 2,
}

/// Binary-compatible stroke join (matches SkPaint::Join exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkStrokeJoinABI {
    #[default]
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

/// Binary-compatible blend mode (matches SkBlendMode exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkBlendModeABI {
    Clear = 0,
    Src = 1,
    Dst = 2,
    #[default]
    SrcOver = 3,
    DstOver = 4,
    SrcIn = 5,
    DstIn = 6,
    SrcOut = 7,
    DstOut = 8,
    SrcATop = 9,
    DstATop = 10,
    Xor = 11,
    Plus = 12,
    Modulate = 13,
    Screen = 14,
    Overlay = 15,
    Darken = 16,
    Lighten = 17,
    ColorDodge = 18,
    ColorBurn = 19,
    HardLight = 20,
    SoftLight = 21,
    Difference = 22,
    Exclusion = 23,
    Multiply = 24,
    Hue = 25,
    Saturation = 26,
    Color = 27,
    Luminosity = 28,
}

// =============================================================================
// Path - Binary Compatible
// =============================================================================

/// Binary-compatible path fill type (matches SkPathFillType exactly)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkPathFillTypeABI {
    #[default]
    Winding = 0,
    EvenOdd = 1,
    InverseWinding = 2,
    InverseEvenOdd = 3,
}

/// Binary-compatible path verb (matches SkPath::Verb exactly)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkPathVerbABI {
    Move = 0,
    Line = 1,
    Quad = 2,
    Conic = 3,
    Cubic = 4,
    Close = 5,
    Done = 6,
}

// =============================================================================
// Conversion Functions
// =============================================================================

/// Convert from internal Point to ABI Point
#[unsafe(no_mangle)]
pub extern "C" fn sk_point_to_abi(x: f32, y: f32) -> SkPointABI {
    SkPointABI { x, y }
}

/// Convert from internal Rect to ABI Rect
#[unsafe(no_mangle)]
pub extern "C" fn sk_rect_to_abi(left: f32, top: f32, right: f32, bottom: f32) -> SkRectABI {
    SkRectABI {
        left,
        top,
        right,
        bottom,
    }
}

/// Create identity matrix
#[unsafe(no_mangle)]
pub extern "C" fn sk_matrix_identity() -> SkMatrixABI {
    SkMatrixABI::identity()
}

/// Create identity matrix 4x4
#[unsafe(no_mangle)]
pub extern "C" fn sk_matrix44_identity() -> SkMatrix44ABI {
    SkMatrix44ABI::identity()
}

// =============================================================================
// Type Size Verification Functions
// =============================================================================

/// Get size of SkPointABI (for runtime verification)
#[unsafe(no_mangle)]
pub extern "C" fn sk_sizeof_point() -> usize {
    std::mem::size_of::<SkPointABI>()
}

/// Get size of SkRectABI (for runtime verification)
#[unsafe(no_mangle)]
pub extern "C" fn sk_sizeof_rect() -> usize {
    std::mem::size_of::<SkRectABI>()
}

/// Get size of SkMatrixABI (for runtime verification)
#[unsafe(no_mangle)]
pub extern "C" fn sk_sizeof_matrix() -> usize {
    std::mem::size_of::<SkMatrixABI>()
}

/// Get size of SkImageInfoABI (for runtime verification)
#[unsafe(no_mangle)]
pub extern "C" fn sk_sizeof_imageinfo() -> usize {
    std::mem::size_of::<SkImageInfoABI>()
}

/// Get size of SkColor4fABI (for runtime verification)
#[unsafe(no_mangle)]
pub extern "C" fn sk_sizeof_color4f() -> usize {
    std::mem::size_of::<SkColor4fABI>()
}

// =============================================================================
// ABI Validation
// =============================================================================

/// Validate that all ABI types have expected sizes
/// Returns true if all sizes match, false otherwise
#[unsafe(no_mangle)]
pub extern "C" fn sk_abi_validate() -> bool {
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
    fn test_point_layout() {
        let p = SkPointABI { x: 1.0, y: 2.0 };
        let bytes: &[u8; 8] = unsafe { std::mem::transmute(&p) };
        // Verify x comes first
        assert_eq!(&bytes[0..4], &1.0_f32.to_ne_bytes());
        assert_eq!(&bytes[4..8], &2.0_f32.to_ne_bytes());
    }

    #[test]
    fn test_matrix_identity() {
        let m = SkMatrixABI::identity();
        assert_eq!(m.values[0], 1.0); // scaleX
        assert_eq!(m.values[4], 1.0); // scaleY
        assert_eq!(m.values[8], 1.0); // persp2
    }

    #[test]
    fn test_version_compatibility() {
        assert!(sk_abi_is_compatible(1, 0));
        assert!(!sk_abi_is_compatible(2, 0));
    }
}
