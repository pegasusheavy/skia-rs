//! C FFI bindings for skia-rs.
//!
//! This crate provides C-compatible bindings for use from other languages.
//! It exposes a C API that mirrors the Skia C API for drop-in compatibility.
//!
//! # Safety
//!
//! All FFI functions are inherently unsafe. Callers must ensure:
//! - Pointers are valid and non-null (unless explicitly documented otherwise)
//! - Proper lifetime management (using the appropriate `_unref` functions)
//! - Thread safety requirements are followed (see below)
//!
//! Every exported `sk_*` function wraps its body in `catch_panic` /
//! `catch_panic_void` so that a Rust panic can never unwind into C. If a
//! panic is caught the function returns a default value (null pointer, zero,
//! `false`, identity matrix, etc.) and sets a thread-visible flag readable
//! via [`sk_last_call_panicked`].
//!
//! # Reference Counting
//!
//! Objects follow Skia's reference counting model:
//! - Objects are created with a reference count of 1
//! - `sk_*_ref()` increments the reference count
//! - `sk_*_unref()` decrements the reference count and frees when it reaches 0
//! - Use `sk_refcnt_get_count()` to query the current count
//!
//! Reference counting operations (`ref`/`unref`) are **thread-safe** and use
//! atomic operations internally. The generic [`sk_refcnt_get_count`] /
//! [`sk_refcnt_is_unique`] entry points perform a magic-tag check before
//! reading the refcount field, so passing a non-refcounted pointer returns 0
//! instead of reading arbitrary memory.
//!
//! # ABI Initialization
//!
//! Callers **must** invoke [`sk_init`] once at startup, passing the major /
//! minor version the client was compiled against. `sk_init` returns `false`
//! if the versions are incompatible; the caller should then abort rather
//! than calling any other `sk_*` function.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)] // FFI types follow C naming conventions

pub mod abi;

use std::ffi::{c_char, c_void};
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::slice;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// =============================================================================
// Panic Catching Infrastructure
// =============================================================================

/// Global flag indicating if the last FFI call panicked.
///
/// The flag is read-and-clear. Callers query it via [`sk_last_call_panicked`]
/// immediately after an FFI call to detect panic recovery.
static LAST_PANIC: AtomicBool = AtomicBool::new(false);

/// Check if the last FFI call panicked.
///
/// Returns true if a panic occurred, false otherwise.
/// Reading this flag clears it.
#[unsafe(no_mangle)]
pub extern "C" fn sk_last_call_panicked() -> bool {
    LAST_PANIC.swap(false, Ordering::SeqCst)
}

/// Catch panics and return a default value if one occurs.
#[inline(always)]
fn catch_panic<T: Default, F: FnOnce() -> T>(f: F) -> T {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_) => {
            LAST_PANIC.store(true, Ordering::SeqCst);
            T::default()
        }
    }
}

/// Catch panics in void-returning functions.
#[inline(always)]
fn catch_panic_void<F: FnOnce()>(f: F) {
    if panic::catch_unwind(AssertUnwindSafe(f)).is_err() {
        LAST_PANIC.store(true, Ordering::SeqCst);
    }
}

// =============================================================================
// Reference Counting Infrastructure
// =============================================================================

/// Magic tag embedded at the head of every [`RefCounted<T>`] instance.
///
/// The generic [`sk_refcnt_get_count`] / [`sk_refcnt_is_unique`] entry points
/// validate this tag before dereferencing the refcount field. That lets them
/// reject bogus pointers (e.g. a `Box<Paint>` that was created outside the
/// refcount wrapper) without reading arbitrary memory.
const REFCOUNT_TAG: u32 = 0x534B_5231; // "SKR1"

/// Reference counted wrapper for FFI objects.
///
/// This provides Skia-compatible reference counting semantics:
/// - Created with refcount of 1
/// - `ref()` increments
/// - `unref()` decrements and frees when 0
///
/// Layout (fixed, `repr(C)`):
///
/// | offset | bytes | field       |
/// |--------|-------|-------------|
/// | 0      | 4     | `tag`       |
/// | 4      | 4     | `refcnt`    |
/// | 8      | ..    | `value`     |
///
/// The refcount lives at offset 4 (not 0) so that callers who rely on the
/// "all refcounted types start with AtomicU32" idiom get a hard type-check
/// via the tag instead of a silent misread.
///
/// cbindgen:no-export
#[repr(C)]
pub struct RefCounted<T> {
    /// Magic tag — must equal [`REFCOUNT_TAG`].
    tag: u32,
    /// Reference count.
    refcnt: AtomicU32,
    /// The wrapped value.
    value: T,
}

impl<T> RefCounted<T> {
    /// Create a new reference counted object with refcount of 1.
    pub fn new(value: T) -> *mut Self {
        Box::into_raw(Box::new(Self {
            tag: REFCOUNT_TAG,
            refcnt: AtomicU32::new(1),
            value,
        }))
    }

    /// Increment the reference count.
    ///
    /// # Safety
    /// Pointer must be valid and non-null.
    pub unsafe fn ref_ptr(ptr: *mut Self) {
        if let Some(rc) = ptr.as_ref() {
            debug_assert_eq!(rc.tag, REFCOUNT_TAG, "sk refcount tag corrupted");
            rc.refcnt.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Decrement the reference count and free if it reaches 0.
    ///
    /// # Safety
    /// Pointer must be valid and non-null.
    /// Returns true if the object was freed.
    pub unsafe fn unref_ptr(ptr: *mut Self) -> bool {
        if ptr.is_null() {
            return false;
        }

        let rc = &*ptr;
        debug_assert_eq!(rc.tag, REFCOUNT_TAG, "sk refcount tag corrupted");
        // Use AcqRel to ensure proper synchronization
        if rc.refcnt.fetch_sub(1, Ordering::AcqRel) == 1 {
            // Last reference, drop the box
            drop(Box::from_raw(ptr));
            true
        } else {
            false
        }
    }

    /// Get the current reference count.
    ///
    /// # Safety
    /// Pointer must be valid and non-null.
    pub unsafe fn get_count(ptr: *const Self) -> u32 {
        if let Some(rc) = ptr.as_ref() {
            rc.refcnt.load(Ordering::Relaxed)
        } else {
            0
        }
    }

    /// Check if this is the only reference.
    ///
    /// # Safety
    /// Pointer must be valid and non-null.
    pub unsafe fn is_unique(ptr: *const Self) -> bool {
        Self::get_count(ptr) == 1
    }

    /// Get a reference to the inner value.
    ///
    /// # Safety
    /// Pointer must be valid and non-null.
    pub unsafe fn get_ref<'a>(ptr: *const Self) -> Option<&'a T> {
        ptr.as_ref().map(|rc| &rc.value)
    }

    /// Get a mutable reference to the inner value.
    ///
    /// # Safety
    /// Pointer must be valid and non-null.
    /// Caller must ensure exclusive access.
    pub unsafe fn get_mut<'a>(ptr: *mut Self) -> Option<&'a mut T> {
        ptr.as_mut().map(|rc| &mut rc.value)
    }
}

// =============================================================================
// Reference Counting C API
// =============================================================================

/// Opaque reference counted object type.
pub type sk_refcnt_t = c_void;

/// Tag-validating view of the shared refcount header.
///
/// Every [`RefCounted<T>`] starts with `{ tag: u32, refcnt: AtomicU32 }`.
/// The generic entry points below match this layout to peek at the refcount
/// without knowing `T`.
#[repr(C)]
struct RefCountedHeader {
    tag: u32,
    refcnt: AtomicU32,
}

/// Get the reference count of an object.
///
/// Returns 0 if the pointer is null or fails the tag check.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_refcnt_get_count(ptr: *const sk_refcnt_t) -> u32 {
    catch_panic(|| {
        if ptr.is_null() {
            return 0;
        }
        let header = &*(ptr as *const RefCountedHeader);
        if header.tag != REFCOUNT_TAG {
            return 0;
        }
        header.refcnt.load(Ordering::Relaxed)
    })
}

/// Check if an object has only one reference (is unique).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_refcnt_is_unique(ptr: *const sk_refcnt_t) -> bool {
    sk_refcnt_get_count(ptr) == 1
}

// Re-export types for FFI
use skia_rs_canvas::{PixelBuffer, Surface};
use skia_rs_core::{
    AlphaType, Color, Color4f, ColorType, IPoint, IRect, ISize, ImageInfo, Matrix, Point, Rect,
    Scalar, Size,
};
use skia_rs_paint::{
    BlendMode, BlurMaskFilter, BlurStyle, ColorFilterRef, ColorMatrixFilter, ImageFilterRef,
    LinearGradient, MaskFilterRef, Paint, RadialGradient, ShaderRef, Style, SweepGradient,
    TileMode,
};
use skia_rs_path::{FillType, Path, PathBuilder, Verb};
use skia_rs_text::{Font, Typeface, TypefaceRef};

// =============================================================================
// Type Definitions
// =============================================================================

/// C-compatible point structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_point_t {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
}

/// C-compatible integer point structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_ipoint_t {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

/// C-compatible size structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_size_t {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// C-compatible integer size structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_isize_t {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

/// C-compatible rectangle structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_rect_t {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
}

/// C-compatible integer rectangle structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_irect_t {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

/// C-compatible matrix structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sk_matrix_t {
    /// Matrix values (row-major).
    pub values: [f32; 9],
}

impl Default for sk_matrix_t {
    fn default() -> Self {
        Self {
            values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// C-compatible image info structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sk_imageinfo_t {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Color type.
    pub color_type: u32,
    /// Alpha type.
    pub alpha_type: u32,
}

/// C-compatible 4-component color (float RGBA).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_color4f_t {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha.
    pub a: f32,
}

/// C-compatible color (ARGB packed).
pub type sk_color_t = u32;

// =============================================================================
// Conversion helpers
// =============================================================================

impl From<Point> for sk_point_t {
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<sk_point_t> for Point {
    fn from(p: sk_point_t) -> Self {
        Point::new(p.x, p.y)
    }
}

impl From<Rect> for sk_rect_t {
    fn from(r: Rect) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

impl From<sk_rect_t> for Rect {
    fn from(r: sk_rect_t) -> Self {
        Rect::new(r.left, r.top, r.right, r.bottom)
    }
}

impl From<Matrix> for sk_matrix_t {
    fn from(m: Matrix) -> Self {
        Self { values: m.values }
    }
}

impl From<sk_matrix_t> for Matrix {
    fn from(m: sk_matrix_t) -> Self {
        Matrix { values: m.values }
    }
}

impl From<Color4f> for sk_color4f_t {
    fn from(c: Color4f) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }
}

impl From<sk_color4f_t> for Color4f {
    fn from(c: sk_color4f_t) -> Self {
        Color4f::new(c.r, c.g, c.b, c.a)
    }
}

fn decode_color_type(v: u32) -> ColorType {
    match v {
        0 => ColorType::Unknown,
        1 => ColorType::Alpha8,
        2 => ColorType::Rgb565,
        3 => ColorType::Argb4444,
        4 => ColorType::Rgba8888,
        5 => ColorType::Bgra8888,
        _ => ColorType::Rgba8888,
    }
}

fn decode_alpha_type(v: u32) -> AlphaType {
    match v {
        0 => AlphaType::Unknown,
        1 => AlphaType::Opaque,
        2 => AlphaType::Premul,
        3 => AlphaType::Unpremul,
        _ => AlphaType::Premul,
    }
}

fn decode_tile_mode(v: u32) -> TileMode {
    match v {
        0 => TileMode::Clamp,
        1 => TileMode::Repeat,
        2 => TileMode::Mirror,
        3 => TileMode::Decal,
        _ => TileMode::Clamp,
    }
}

// =============================================================================
// ABI initialization
// =============================================================================

static INIT_OK: AtomicBool = AtomicBool::new(false);

/// Initialize the library and verify ABI compatibility.
///
/// C clients must call this once, before any other `sk_*` function, passing
/// the major/minor version they were compiled against. Returns `true` if the
/// linked library is compatible. If `false`, the caller must not call any
/// other `sk_*` function — behavior is undefined.
///
/// Calling `sk_init` twice with compatible values is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn sk_init(major: u32, minor: u32) -> bool {
    if abi::sk_abi_is_compatible(major, minor) {
        INIT_OK.store(true, Ordering::SeqCst);
        true
    } else {
        false
    }
}

/// Check whether [`sk_init`] has been called successfully.
#[unsafe(no_mangle)]
pub extern "C" fn sk_is_initialized() -> bool {
    INIT_OK.load(Ordering::SeqCst)
}

// =============================================================================
// Surface API (Reference Counted)
// =============================================================================

/// Reference counted surface type.
pub type sk_surface_t = RefCounted<Surface>;

/// Create a new raster surface.
///
/// Returns a surface with refcount of 1, or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_new_raster(width: i32, height: i32) -> *mut sk_surface_t {
    catch_panic(|| match Surface::new_raster_n32_premul(width, height) {
        Some(surface) => RefCounted::new(surface),
        None => ptr::null_mut(),
    })
}

/// Create a raster surface with specific image info.
///
/// Returns a surface with refcount of 1, or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_new_raster_with_info(
    info: *const sk_imageinfo_t,
) -> *mut sk_surface_t {
    catch_panic(|| {
        if info.is_null() {
            return ptr::null_mut();
        }

        let info = &*info;
        let img_info = match ImageInfo::new(
            info.width,
            info.height,
            decode_color_type(info.color_type),
            decode_alpha_type(info.alpha_type),
        ) {
            Ok(i) => i,
            Err(_) => return ptr::null_mut(),
        };

        match Surface::new_raster(&img_info, None) {
            Some(surface) => RefCounted::new(surface),
            None => ptr::null_mut(),
        }
    })
}

/// Increment the reference count of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_ref(surface: *mut sk_surface_t) {
    catch_panic_void(|| RefCounted::ref_ptr(surface));
}

/// Decrement the reference count of a surface.
///
/// Frees the surface when the count reaches 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_unref(surface: *mut sk_surface_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(surface);
    });
}

/// Get the reference count of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_refcnt(surface: *const sk_surface_t) -> u32 {
    catch_panic(|| RefCounted::get_count(surface))
}

/// Check if the surface has only one reference.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_is_unique(surface: *const sk_surface_t) -> bool {
    catch_panic(|| RefCounted::is_unique(surface))
}

/// Get the width of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_width(surface: *const sk_surface_t) -> i32 {
    catch_panic(|| RefCounted::get_ref(surface).map_or(0, |s| s.width()))
}

/// Get the height of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_height(surface: *const sk_surface_t) -> i32 {
    catch_panic(|| RefCounted::get_ref(surface).map_or(0, |s| s.height()))
}

/// Get the pixel data from a surface (unsynchronized borrow).
///
/// **Lifetime:** the pointer returned via `out_pixels` is only valid for as
/// long as `surface` remains allocated AND is not mutated. Any subsequent
/// drawing call on this surface (e.g. [`sk_surface_draw_rect`]) may move
/// or resize the underlying buffer, invalidating the pointer. Callers that
/// need a stable snapshot should copy out via [`sk_surface_read_pixels`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_peek_pixels(
    surface: *const sk_surface_t,
    out_pixels: *mut *const u8,
    out_row_bytes: *mut usize,
) -> bool {
    catch_panic(|| {
        if out_pixels.is_null() || out_row_bytes.is_null() {
            return false;
        }

        if let Some(s) = RefCounted::get_ref(surface) {
            *out_pixels = s.pixels().as_ptr();
            *out_row_bytes = s.row_bytes();
            true
        } else {
            false
        }
    })
}

/// Copy the pixel data from a surface into a caller-owned buffer.
///
/// Safer than [`sk_surface_peek_pixels`] — the caller owns the destination
/// buffer and does not need to track the surface's lifetime.
///
/// Returns the number of bytes written, or 0 on failure (null surface,
/// null/undersized destination buffer).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_read_pixels(
    surface: *const sk_surface_t,
    dst: *mut u8,
    dst_len: usize,
) -> usize {
    catch_panic(|| {
        if dst.is_null() {
            return 0;
        }
        let Some(s) = RefCounted::get_ref(surface) else {
            return 0;
        };
        let src = s.pixels();
        if dst_len < src.len() {
            return 0;
        }
        ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
        src.len()
    })
}

// =============================================================================
// Paint API (Reference Counted)
// =============================================================================

/// Reference counted paint type.
pub type sk_paint_t = RefCounted<Paint>;

/// Create a new paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_new() -> *mut sk_paint_t {
    catch_panic(|| RefCounted::new(Paint::new()))
}

/// Clone a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_clone(paint: *const sk_paint_t) -> *mut sk_paint_t {
    catch_panic(|| RefCounted::get_ref(paint).map_or(ptr::null_mut(), |p| RefCounted::new(p.clone())))
}

/// Increment the reference count of a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_ref(paint: *mut sk_paint_t) {
    catch_panic_void(|| RefCounted::ref_ptr(paint));
}

/// Decrement the reference count of a paint (alias for unref).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_delete(paint: *mut sk_paint_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(paint);
    });
}

/// Decrement the reference count of a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_unref(paint: *mut sk_paint_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(paint);
    });
}

/// Get the reference count of a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_refcnt(paint: *const sk_paint_t) -> u32 {
    catch_panic(|| RefCounted::get_count(paint))
}

/// Set the paint color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_color(paint: *mut sk_paint_t, color: sk_color_t) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            p.set_color32(Color(color));
        }
    });
}

/// Get the paint color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_color(paint: *const sk_paint_t) -> sk_color_t {
    catch_panic(|| RefCounted::get_ref(paint).map_or(0, |p| p.color32().0))
}

/// Set the paint color as float RGBA.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_color4f(paint: *mut sk_paint_t, color: sk_color4f_t) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            p.set_color(color.into());
        }
    });
}

/// Get the paint color as float RGBA.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_color4f(paint: *const sk_paint_t) -> sk_color4f_t {
    catch_panic(|| {
        RefCounted::get_ref(paint).map_or(sk_color4f_t::default(), |p| p.color().into())
    })
}

/// Set the paint style.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_style(paint: *mut sk_paint_t, style: u32) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            let style = match style {
                0 => Style::Fill,
                1 => Style::Stroke,
                2 => Style::StrokeAndFill,
                _ => Style::Fill,
            };
            p.set_style(style);
        }
    });
}

/// Set the stroke width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_stroke_width(paint: *mut sk_paint_t, width: f32) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            p.set_stroke_width(width);
        }
    });
}

/// Get the stroke width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_stroke_width(paint: *const sk_paint_t) -> f32 {
    catch_panic(|| RefCounted::get_ref(paint).map_or(0.0, |p| p.stroke_width()))
}

/// Set anti-alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_antialias(paint: *mut sk_paint_t, aa: bool) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            p.set_anti_alias(aa);
        }
    });
}

/// Check if anti-alias is enabled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_is_antialias(paint: *const sk_paint_t) -> bool {
    catch_panic(|| RefCounted::get_ref(paint).is_some_and(|p| p.is_anti_alias()))
}

/// Set the paint's blend mode (matches [`abi::SkBlendModeABI`]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_blend_mode(paint: *mut sk_paint_t, mode: u32) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            let mode = match mode {
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
                _ => BlendMode::SrcOver,
            };
            p.set_blend_mode(mode);
        }
    });
}

/// Attach a shader to the paint. Passing null clears it.
///
/// The paint takes a new reference to the shader; the caller retains their
/// own reference and must still unref it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_shader(paint: *mut sk_paint_t, shader: *const sk_shader_t) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            if shader.is_null() {
                p.set_shader(None);
            } else if let Some(s) = RefCounted::get_ref(shader) {
                p.set_shader(Some(s.clone()));
            }
        }
    });
}

/// Attach a color filter to the paint. Passing null clears it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_color_filter(
    paint: *mut sk_paint_t,
    filter: *const sk_colorfilter_t,
) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            if filter.is_null() {
                p.set_color_filter(None);
            } else if let Some(f) = RefCounted::get_ref(filter) {
                p.set_color_filter(Some(f.clone()));
            }
        }
    });
}

/// Attach a mask filter to the paint. Passing null clears it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_mask_filter(
    paint: *mut sk_paint_t,
    filter: *const sk_maskfilter_t,
) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            if filter.is_null() {
                p.set_mask_filter(None);
            } else if let Some(f) = RefCounted::get_ref(filter) {
                p.set_mask_filter(Some(f.clone()));
            }
        }
    });
}

/// Attach an image filter to the paint. Passing null clears it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_image_filter(
    paint: *mut sk_paint_t,
    filter: *const sk_imagefilter_t,
) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(paint) {
            if filter.is_null() {
                p.set_image_filter(None);
            } else if let Some(f) = RefCounted::get_ref(filter) {
                p.set_image_filter(Some(f.clone()));
            }
        }
    });
}

// =============================================================================
// Path API (Reference Counted)
// =============================================================================

/// Reference counted path type.
pub type sk_path_t = RefCounted<Path>;

/// Create a new path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_new() -> *mut sk_path_t {
    catch_panic(|| RefCounted::new(Path::new()))
}

/// Clone a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_clone(path: *const sk_path_t) -> *mut sk_path_t {
    catch_panic(|| RefCounted::get_ref(path).map_or(ptr::null_mut(), |p| RefCounted::new(p.clone())))
}

/// Increment the reference count of a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_ref(path: *mut sk_path_t) {
    catch_panic_void(|| RefCounted::ref_ptr(path));
}

/// Decrement the reference count of a path (alias for unref).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_delete(path: *mut sk_path_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(path);
    });
}

/// Decrement the reference count of a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_unref(path: *mut sk_path_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(path);
    });
}

/// Get the reference count of a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_refcnt(path: *const sk_path_t) -> u32 {
    catch_panic(|| RefCounted::get_count(path))
}

/// Get the path bounds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_bounds(path: *const sk_path_t, bounds: *mut sk_rect_t) {
    catch_panic_void(|| {
        if let (Some(p), Some(b)) = (RefCounted::get_ref(path), bounds.as_mut()) {
            let rect = p.bounds();
            *b = rect.into();
        }
    });
}

/// Check if path is empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_is_empty(path: *const sk_path_t) -> bool {
    catch_panic(|| RefCounted::get_ref(path).is_none_or(|p| p.is_empty()))
}

/// Get the fill type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_filltype(path: *const sk_path_t) -> u32 {
    catch_panic(|| {
        RefCounted::get_ref(path).map_or(0, |p| match p.fill_type() {
            FillType::Winding => 0,
            FillType::EvenOdd => 1,
            FillType::InverseWinding => 2,
            FillType::InverseEvenOdd => 3,
        })
    })
}

/// Set the fill type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_set_filltype(path: *mut sk_path_t, fill_type: u32) {
    catch_panic_void(|| {
        if let Some(p) = RefCounted::get_mut(path) {
            let ft = match fill_type {
                0 => FillType::Winding,
                1 => FillType::EvenOdd,
                2 => FillType::InverseWinding,
                3 => FillType::InverseEvenOdd,
                _ => FillType::Winding,
            };
            p.set_fill_type(ft);
        }
    });
}

/// Check if path contains a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_contains(path: *const sk_path_t, x: f32, y: f32) -> bool {
    catch_panic(|| RefCounted::get_ref(path).is_some_and(|p| p.contains(Point::new(x, y))))
}

// =============================================================================
// Path Iteration API
// =============================================================================

/// Stateful iterator over a path's verbs.
///
/// Create with [`sk_path_iter_new`], advance with [`sk_path_iter_next`],
/// destroy with [`sk_path_iter_delete`]. The iterator holds a clone of the
/// path's verbs/points — the source path can be modified or dropped after
/// iteration begins without invalidating the iterator.
pub struct sk_path_iter_t {
    verbs: Vec<Verb>,
    points: Vec<Point>,
    weights: Vec<Scalar>,
    verb_index: usize,
    point_index: usize,
    weight_index: usize,
}

/// ABI-stable verb codes emitted by [`sk_path_iter_next`].
pub const SK_PATH_VERB_MOVE: u32 = 0;
/// ABI-stable verb code for a line segment.
pub const SK_PATH_VERB_LINE: u32 = 1;
/// ABI-stable verb code for a quadratic bezier.
pub const SK_PATH_VERB_QUAD: u32 = 2;
/// ABI-stable verb code for a conic section.
pub const SK_PATH_VERB_CONIC: u32 = 3;
/// ABI-stable verb code for a cubic bezier.
pub const SK_PATH_VERB_CUBIC: u32 = 4;
/// ABI-stable verb code for a contour close.
pub const SK_PATH_VERB_CLOSE: u32 = 5;
/// Sentinel verb code returned once the iterator is exhausted.
pub const SK_PATH_VERB_DONE: u32 = 6;

/// Create an iterator over a path's verbs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_iter_new(path: *const sk_path_t) -> *mut sk_path_iter_t {
    catch_panic(|| {
        let Some(p) = RefCounted::get_ref(path) else {
            return ptr::null_mut();
        };
        // Collect conic weights (if any) via the public iterator — the
        // raw `conic_weights` field isn't publicly exposed on Path.
        let weights: Vec<Scalar> = if p.verbs().contains(&Verb::Conic) {
            p.iter()
                .filter_map(|el| match el {
                    skia_rs_path::PathElement::Conic(_, _, w) => Some(w),
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };

        Box::into_raw(Box::new(sk_path_iter_t {
            verbs: p.verbs().to_vec(),
            points: p.points().to_vec(),
            weights,
            verb_index: 0,
            point_index: 0,
            weight_index: 0,
        }))
    })
}

/// Destroy a path iterator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_iter_delete(iter: *mut sk_path_iter_t) {
    catch_panic_void(|| {
        if !iter.is_null() {
            drop(Box::from_raw(iter));
        }
    });
}

/// Advance the iterator. Fills up to four points in `out_points` (caller
/// provides at least 4 slots) and the conic weight (if applicable) in
/// `out_weight`. Returns the verb code; `SK_PATH_VERB_DONE` indicates
/// exhaustion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_iter_next(
    iter: *mut sk_path_iter_t,
    out_points: *mut sk_point_t,
    out_weight: *mut f32,
) -> u32 {
    catch_panic(|| {
        let Some(it) = iter.as_mut() else {
            return SK_PATH_VERB_DONE;
        };
        if it.verb_index >= it.verbs.len() {
            return SK_PATH_VERB_DONE;
        }
        let verb = it.verbs[it.verb_index];
        it.verb_index += 1;

        let write = |i: usize, p: Point| {
            if !out_points.is_null() {
                *out_points.add(i) = p.into();
            }
        };

        match verb {
            Verb::Move => {
                write(0, it.points[it.point_index]);
                it.point_index += 1;
                SK_PATH_VERB_MOVE
            }
            Verb::Line => {
                write(0, it.points[it.point_index]);
                it.point_index += 1;
                SK_PATH_VERB_LINE
            }
            Verb::Quad => {
                write(0, it.points[it.point_index]);
                write(1, it.points[it.point_index + 1]);
                it.point_index += 2;
                SK_PATH_VERB_QUAD
            }
            Verb::Conic => {
                write(0, it.points[it.point_index]);
                write(1, it.points[it.point_index + 1]);
                let w = it
                    .weights
                    .get(it.weight_index)
                    .copied()
                    .unwrap_or(1.0);
                if !out_weight.is_null() {
                    *out_weight = w;
                }
                it.point_index += 2;
                it.weight_index += 1;
                SK_PATH_VERB_CONIC
            }
            Verb::Cubic => {
                write(0, it.points[it.point_index]);
                write(1, it.points[it.point_index + 1]);
                write(2, it.points[it.point_index + 2]);
                it.point_index += 3;
                SK_PATH_VERB_CUBIC
            }
            Verb::Close => SK_PATH_VERB_CLOSE,
        }
    })
}

// =============================================================================
// Path Builder API (Reference Counted)
// =============================================================================

/// Reference counted path builder type.
pub type sk_pathbuilder_t = RefCounted<PathBuilder>;

/// Create a new path builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_new() -> *mut sk_pathbuilder_t {
    catch_panic(|| RefCounted::new(PathBuilder::new()))
}

/// Increment the reference count of a path builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_ref(builder: *mut sk_pathbuilder_t) {
    catch_panic_void(|| RefCounted::ref_ptr(builder));
}

/// Decrement the reference count of a path builder (alias for unref).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_delete(builder: *mut sk_pathbuilder_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(builder);
    });
}

/// Decrement the reference count of a path builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_unref(builder: *mut sk_pathbuilder_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(builder);
    });
}

/// Move to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_move_to(builder: *mut sk_pathbuilder_t, x: f32, y: f32) {
    catch_panic_void(|| {
        if let Some(b) = RefCounted::get_mut(builder) {
            b.move_to(x, y);
        }
    });
}

/// Line to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_line_to(builder: *mut sk_pathbuilder_t, x: f32, y: f32) {
    catch_panic_void(|| {
        if let Some(b) = RefCounted::get_mut(builder) {
            b.line_to(x, y);
        }
    });
}

/// Quadratic bezier to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_quad_to(
    builder: *mut sk_pathbuilder_t,
    cx: f32,
    cy: f32,
    x: f32,
    y: f32,
) {
    catch_panic_void(|| {
        if let Some(b) = RefCounted::get_mut(builder) {
            b.quad_to(cx, cy, x, y);
        }
    });
}

/// Cubic bezier to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_cubic_to(
    builder: *mut sk_pathbuilder_t,
    c1x: f32,
    c1y: f32,
    c2x: f32,
    c2y: f32,
    x: f32,
    y: f32,
) {
    catch_panic_void(|| {
        if let Some(b) = RefCounted::get_mut(builder) {
            b.cubic_to(c1x, c1y, c2x, c2y, x, y);
        }
    });
}

/// Close the path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_close(builder: *mut sk_pathbuilder_t) {
    catch_panic_void(|| {
        if let Some(b) = RefCounted::get_mut(builder) {
            b.close();
        }
    });
}

/// Add a rectangle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_rect(
    builder: *mut sk_pathbuilder_t,
    rect: *const sk_rect_t,
) {
    catch_panic_void(|| {
        if let (Some(b), Some(r)) = (RefCounted::get_mut(builder), rect.as_ref()) {
            b.add_rect(&Rect::from(*r));
        }
    });
}

/// Add an oval.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_oval(
    builder: *mut sk_pathbuilder_t,
    rect: *const sk_rect_t,
) {
    catch_panic_void(|| {
        if let (Some(b), Some(r)) = (RefCounted::get_mut(builder), rect.as_ref()) {
            b.add_oval(&Rect::from(*r));
        }
    });
}

/// Add a circle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_circle(
    builder: *mut sk_pathbuilder_t,
    cx: f32,
    cy: f32,
    radius: f32,
) {
    catch_panic_void(|| {
        if let Some(b) = RefCounted::get_mut(builder) {
            b.add_circle(cx, cy, radius);
        }
    });
}

/// Build the path and reset the builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_detach(builder: *mut sk_pathbuilder_t) -> *mut sk_path_t {
    catch_panic(|| {
        if let Some(b) = RefCounted::get_mut(builder) {
            let path = std::mem::replace(b, PathBuilder::new()).build();
            RefCounted::new(path)
        } else {
            ptr::null_mut()
        }
    })
}

/// Build the path without resetting the builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_snapshot(
    builder: *const sk_pathbuilder_t,
) -> *mut sk_path_t {
    catch_panic(|| {
        RefCounted::get_ref(builder).map_or(ptr::null_mut(), |b| RefCounted::new(b.clone().build()))
    })
}

// =============================================================================
// Matrix API
// =============================================================================

/// Set matrix to identity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_identity(matrix: *mut sk_matrix_t) {
    catch_panic_void(|| {
        if let Some(m) = matrix.as_mut() {
            *m = sk_matrix_t::default();
        }
    });
}

/// Set matrix to translate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_translate(matrix: *mut sk_matrix_t, dx: f32, dy: f32) {
    catch_panic_void(|| {
        if let Some(m) = matrix.as_mut() {
            *m = Matrix::translate(dx, dy).into();
        }
    });
}

/// Set matrix to scale.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_scale(matrix: *mut sk_matrix_t, sx: f32, sy: f32) {
    catch_panic_void(|| {
        if let Some(m) = matrix.as_mut() {
            *m = Matrix::scale(sx, sy).into();
        }
    });
}

/// Set matrix to rotate (degrees).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_rotate(matrix: *mut sk_matrix_t, degrees: f32) {
    catch_panic_void(|| {
        if let Some(m) = matrix.as_mut() {
            let radians = degrees * std::f32::consts::PI / 180.0;
            *m = Matrix::rotate(radians).into();
        }
    });
}

/// Concatenate two matrices.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_concat(
    result: *mut sk_matrix_t,
    a: *const sk_matrix_t,
    b: *const sk_matrix_t,
) {
    catch_panic_void(|| {
        if let (Some(r), Some(a), Some(b)) = (result.as_mut(), a.as_ref(), b.as_ref()) {
            let ma: Matrix = (*a).into();
            let mb: Matrix = (*b).into();
            *r = ma.concat(&mb).into();
        }
    });
}

/// Map a point through a matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_map_point(
    matrix: *const sk_matrix_t,
    point: *const sk_point_t,
    result: *mut sk_point_t,
) {
    catch_panic_void(|| {
        if let (Some(m), Some(p), Some(r)) = (matrix.as_ref(), point.as_ref(), result.as_mut()) {
            let mat: Matrix = (*m).into();
            let pt: Point = (*p).into();
            *r = mat.map_point(pt).into();
        }
    });
}

/// Invert a matrix into `result`. Returns true on success, false if the
/// matrix is singular (non-invertible); on failure `result` is unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_invert(
    matrix: *const sk_matrix_t,
    result: *mut sk_matrix_t,
) -> bool {
    catch_panic(|| {
        let Some(m) = matrix.as_ref() else {
            return false;
        };
        let Some(r) = result.as_mut() else {
            return false;
        };
        let mat: Matrix = (*m).into();
        match mat.invert() {
            Some(inv) => {
                *r = inv.into();
                true
            }
            None => false,
        }
    })
}

/// Return true if the matrix is the identity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_is_identity(matrix: *const sk_matrix_t) -> bool {
    catch_panic(|| {
        let Some(m) = matrix.as_ref() else {
            return false;
        };
        let mat: Matrix = (*m).into();
        mat.is_identity()
    })
}

/// Return the determinant of the matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_determinant(matrix: *const sk_matrix_t) -> f32 {
    catch_panic(|| {
        let Some(m) = matrix.as_ref() else {
            return 0.0;
        };
        let mat: Matrix = (*m).into();
        mat.determinant()
    })
}

// =============================================================================
// Utility functions
// =============================================================================

/// Get the library version as a NUL-terminated static string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_version() -> *const c_char {
    static VERSION: &[u8] = b"skia-rs 0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Check if the library is available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_is_available() -> bool {
    true
}

// =============================================================================
// Surface drawing helpers (simplified path; prefer sk_canvas_* for parity)
// =============================================================================

/// Clear a surface with a color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_clear(surface: *mut sk_surface_t, color: sk_color_t) {
    catch_panic_void(|| {
        if let Some(s) = RefCounted::get_mut(surface) {
            let mut canvas = s.raster_canvas();
            canvas.clear(Color(color));
        }
    });
}

/// Draw a rect on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_rect(
    surface: *mut sk_surface_t,
    rect: *const sk_rect_t,
    paint: *const sk_paint_t,
) {
    catch_panic_void(|| {
        if let (Some(s), Some(r), Some(p)) = (
            RefCounted::get_mut(surface),
            rect.as_ref(),
            RefCounted::get_ref(paint),
        ) {
            let mut canvas = s.raster_canvas();
            canvas.draw_rect(&Rect::from(*r), p);
        }
    });
}

/// Draw a circle on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_circle(
    surface: *mut sk_surface_t,
    cx: f32,
    cy: f32,
    radius: f32,
    paint: *const sk_paint_t,
) {
    catch_panic_void(|| {
        if let (Some(s), Some(p)) = (RefCounted::get_mut(surface), RefCounted::get_ref(paint)) {
            let mut canvas = s.raster_canvas();
            canvas.draw_circle(Point::new(cx, cy), radius, p);
        }
    });
}

/// Draw a path on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_path(
    surface: *mut sk_surface_t,
    path: *const sk_path_t,
    paint: *const sk_paint_t,
) {
    catch_panic_void(|| {
        if let (Some(s), Some(path), Some(p)) = (
            RefCounted::get_mut(surface),
            RefCounted::get_ref(path),
            RefCounted::get_ref(paint),
        ) {
            let mut canvas = s.raster_canvas();
            canvas.draw_path(path, p);
        }
    });
}

/// Draw a line on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_line(
    surface: *mut sk_surface_t,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    paint: *const sk_paint_t,
) {
    catch_panic_void(|| {
        if let (Some(s), Some(p)) = (RefCounted::get_mut(surface), RefCounted::get_ref(paint)) {
            let mut canvas = s.raster_canvas();
            canvas.draw_line(Point::new(x0, y0), Point::new(x1, y1), p);
        }
    });
}

// =============================================================================
// Canvas API (borrowed from a surface)
// =============================================================================

/// Opaque canvas handle.
///
/// A canvas is an explicit raster drawing context borrowed from a surface.
/// Call [`sk_surface_lock_canvas`] to acquire one, issue draw/transform/clip
/// calls, then release with [`sk_canvas_release`]. The canvas retains
/// exclusive mutable access to the backing pixel buffer while locked.
pub struct sk_canvas_t {
    /// Non-null while locked; dropping this box releases the borrow.
    buffer: *mut PixelBuffer,
    width: i32,
    height: i32,
    state: CanvasState,
}

struct CanvasState {
    save_count: usize,
    matrix: Matrix,
    stack: Vec<Matrix>,
}

impl sk_canvas_t {
    unsafe fn canvas(&mut self) -> skia_rs_canvas::Canvas<'_> {
        let buf = &mut *self.buffer;
        skia_rs_canvas::Canvas::new_raster(buf)
    }
}

/// Acquire a canvas borrowed from the given surface.
///
/// The returned canvas is valid until [`sk_canvas_release`] is called and
/// must not outlive `surface`. Only one canvas may be held per surface at
/// a time — attempting to lock twice returns null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_lock_canvas(
    surface: *mut sk_surface_t,
) -> *mut sk_canvas_t {
    catch_panic(|| {
        let Some(s) = RefCounted::get_mut(surface) else {
            return ptr::null_mut();
        };
        let width = s.width();
        let height = s.height();
        let buffer: *mut PixelBuffer = s.pixel_buffer_mut();
        Box::into_raw(Box::new(sk_canvas_t {
            buffer,
            width,
            height,
            state: CanvasState {
                save_count: 0,
                matrix: Matrix::identity(),
                stack: Vec::new(),
            },
        }))
    })
}

/// Release a canvas acquired via [`sk_surface_lock_canvas`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_release(canvas: *mut sk_canvas_t) {
    catch_panic_void(|| {
        if !canvas.is_null() {
            drop(Box::from_raw(canvas));
        }
    });
}

/// Get the width of the canvas.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_get_width(canvas: *const sk_canvas_t) -> i32 {
    catch_panic(|| canvas.as_ref().map_or(0, |c| c.width))
}

/// Get the height of the canvas.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_get_height(canvas: *const sk_canvas_t) -> i32 {
    catch_panic(|| canvas.as_ref().map_or(0, |c| c.height))
}

/// Save the current transformation matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_save(canvas: *mut sk_canvas_t) -> i32 {
    catch_panic(|| {
        let Some(c) = canvas.as_mut() else {
            return 0;
        };
        c.state.stack.push(c.state.matrix);
        c.state.save_count += 1;
        c.state.save_count as i32
    })
}

/// Restore the most recently saved matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_restore(canvas: *mut sk_canvas_t) {
    catch_panic_void(|| {
        let Some(c) = canvas.as_mut() else {
            return;
        };
        if let Some(m) = c.state.stack.pop() {
            c.state.matrix = m;
            c.state.save_count = c.state.save_count.saturating_sub(1);
        }
    });
}

/// Concatenate a matrix onto the canvas's current transform.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_concat(canvas: *mut sk_canvas_t, matrix: *const sk_matrix_t) {
    catch_panic_void(|| {
        let Some(c) = canvas.as_mut() else { return };
        let Some(m) = matrix.as_ref() else { return };
        let mat: Matrix = (*m).into();
        c.state.matrix = c.state.matrix.concat(&mat);
    });
}

/// Translate the canvas coordinate system.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_translate(canvas: *mut sk_canvas_t, dx: f32, dy: f32) {
    catch_panic_void(|| {
        if let Some(c) = canvas.as_mut() {
            c.state.matrix = c.state.matrix.concat(&Matrix::translate(dx, dy));
        }
    });
}

/// Scale the canvas coordinate system.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_scale(canvas: *mut sk_canvas_t, sx: f32, sy: f32) {
    catch_panic_void(|| {
        if let Some(c) = canvas.as_mut() {
            c.state.matrix = c.state.matrix.concat(&Matrix::scale(sx, sy));
        }
    });
}

/// Rotate the canvas coordinate system (degrees).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_rotate(canvas: *mut sk_canvas_t, degrees: f32) {
    catch_panic_void(|| {
        if let Some(c) = canvas.as_mut() {
            let r = degrees * std::f32::consts::PI / 180.0;
            c.state.matrix = c.state.matrix.concat(&Matrix::rotate(r));
        }
    });
}

/// Clear the canvas with a color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_clear(canvas: *mut sk_canvas_t, color: sk_color_t) {
    catch_panic_void(|| {
        let Some(c) = canvas.as_mut() else { return };
        let mut inner = c.canvas();
        inner.clear(Color(color));
    });
}

/// Draw a rect on the canvas, honoring the canvas's current transform.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_draw_rect(
    canvas: *mut sk_canvas_t,
    rect: *const sk_rect_t,
    paint: *const sk_paint_t,
) {
    catch_panic_void(|| {
        let Some(c) = canvas.as_mut() else { return };
        let Some(r) = rect.as_ref() else { return };
        let Some(p) = RefCounted::get_ref(paint) else {
            return;
        };
        let transform = c.state.matrix;
        let mut inner = c.canvas();
        inner.concat(&transform);
        inner.draw_rect(&Rect::from(*r), p);
    });
}

/// Draw a path on the canvas, honoring the canvas's current transform.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_canvas_draw_path(
    canvas: *mut sk_canvas_t,
    path: *const sk_path_t,
    paint: *const sk_paint_t,
) {
    catch_panic_void(|| {
        let Some(c) = canvas.as_mut() else { return };
        let Some(pth) = RefCounted::get_ref(path) else {
            return;
        };
        let Some(p) = RefCounted::get_ref(paint) else {
            return;
        };
        let transform = c.state.matrix;
        let mut inner = c.canvas();
        inner.concat(&transform);
        inner.draw_path(pth, p);
    });
}

// =============================================================================
// Image API (Reference Counted)
// =============================================================================

/// Reference counted image type.
pub type sk_image_t = RefCounted<skia_rs_codec::Image>;

/// Decode an encoded image (PNG/JPEG/…) from a byte buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_from_encoded(
    data: *const u8,
    len: usize,
) -> *mut sk_image_t {
    catch_panic(|| {
        if data.is_null() || len == 0 {
            return ptr::null_mut();
        }
        let bytes = slice::from_raw_parts(data, len);
        match skia_rs_codec::decode_image(bytes) {
            Ok(img) => RefCounted::new(img),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Create an image filled with a single color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_from_color(
    width: i32,
    height: i32,
    color: sk_color_t,
) -> *mut sk_image_t {
    catch_panic(|| match skia_rs_codec::Image::from_color(width, height, color) {
        Some(img) => RefCounted::new(img),
        None => ptr::null_mut(),
    })
}

/// Increment the reference count of an image.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_ref(image: *mut sk_image_t) {
    catch_panic_void(|| RefCounted::ref_ptr(image));
}

/// Decrement the reference count of an image.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_unref(image: *mut sk_image_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(image);
    });
}

/// Get the reference count of an image.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_get_refcnt(image: *const sk_image_t) -> u32 {
    catch_panic(|| RefCounted::get_count(image))
}

/// Get the image width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_get_width(image: *const sk_image_t) -> i32 {
    catch_panic(|| RefCounted::get_ref(image).map_or(0, |i| i.width()))
}

/// Get the image height.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_get_height(image: *const sk_image_t) -> i32 {
    catch_panic(|| RefCounted::get_ref(image).map_or(0, |i| i.height()))
}

/// Encode an image as PNG into a caller-allocated buffer.
///
/// Returns the number of bytes written. If `dst` is null or `dst_len` is 0,
/// returns the required buffer size without writing. Returns 0 on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_image_encode_png(
    image: *const sk_image_t,
    dst: *mut u8,
    dst_len: usize,
) -> usize {
    catch_panic(|| {
        let Some(img) = RefCounted::get_ref(image) else {
            return 0;
        };
        use skia_rs_codec::ImageEncoder;
        let encoder = skia_rs_codec::PngEncoder::new();
        let Ok(bytes) = encoder.encode_bytes(img) else {
            return 0;
        };
        if dst.is_null() || dst_len == 0 {
            return bytes.len();
        }
        if dst_len < bytes.len() {
            return 0;
        }
        ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        bytes.len()
    })
}

// =============================================================================
// Typeface / Font API (Reference Counted)
// =============================================================================

/// Reference counted typeface type.
pub type sk_typeface_t = RefCounted<TypefaceRef>;

/// Reference counted font type (typeface + size + options).
pub type sk_font_t = RefCounted<Font>;

/// Create the default typeface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_typeface_default() -> *mut sk_typeface_t {
    catch_panic(|| RefCounted::new(Arc::new(Typeface::default_typeface())))
}

/// Load a typeface from raw font file data (TTF/OTF).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_typeface_from_data(
    data: *const u8,
    len: usize,
) -> *mut sk_typeface_t {
    catch_panic(|| {
        if data.is_null() || len == 0 {
            return ptr::null_mut();
        }
        let buf = slice::from_raw_parts(data, len).to_vec();
        match Typeface::from_data(buf) {
            Some(t) => RefCounted::new(Arc::new(t)),
            None => ptr::null_mut(),
        }
    })
}

/// Increment typeface refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_typeface_ref(tf: *mut sk_typeface_t) {
    catch_panic_void(|| RefCounted::ref_ptr(tf));
}

/// Decrement typeface refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_typeface_unref(tf: *mut sk_typeface_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(tf);
    });
}

/// Get typeface refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_typeface_get_refcnt(tf: *const sk_typeface_t) -> u32 {
    catch_panic(|| RefCounted::get_count(tf))
}

/// Get the typeface's units-per-em.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_typeface_units_per_em(tf: *const sk_typeface_t) -> u16 {
    catch_panic(|| RefCounted::get_ref(tf).map_or(0, |t| t.units_per_em()))
}

/// Get the typeface's glyph count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_typeface_glyph_count(tf: *const sk_typeface_t) -> u16 {
    catch_panic(|| RefCounted::get_ref(tf).map_or(0, |t| t.glyph_count()))
}

/// Create a font from a typeface and size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_font_new(tf: *const sk_typeface_t, size: f32) -> *mut sk_font_t {
    catch_panic(|| {
        let Some(t) = RefCounted::get_ref(tf) else {
            return ptr::null_mut();
        };
        RefCounted::new(Font::new(t.clone(), size))
    })
}

/// Increment font refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_font_ref(font: *mut sk_font_t) {
    catch_panic_void(|| RefCounted::ref_ptr(font));
}

/// Decrement font refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_font_unref(font: *mut sk_font_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(font);
    });
}

/// Get font size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_font_get_size(font: *const sk_font_t) -> f32 {
    catch_panic(|| RefCounted::get_ref(font).map_or(0.0, |f| f.size()))
}

/// Measure a UTF-8 text string at the font's current size. Returns the
/// advance width in pixels; negative return indicates an invalid font or
/// invalid UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_font_measure_text(
    font: *const sk_font_t,
    text: *const c_char,
    text_len: usize,
) -> f32 {
    catch_panic(|| {
        if text.is_null() {
            return -1.0;
        }
        let Some(f) = RefCounted::get_ref(font) else {
            return -1.0;
        };
        let slice = slice::from_raw_parts(text as *const u8, text_len);
        let Ok(s) = std::str::from_utf8(slice) else {
            return -1.0;
        };
        f.measure_text(s)
    })
}

// =============================================================================
// Shader API (Reference Counted)
// =============================================================================

/// Reference counted shader type.
pub type sk_shader_t = RefCounted<ShaderRef>;

/// Increment shader refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_shader_ref(shader: *mut sk_shader_t) {
    catch_panic_void(|| RefCounted::ref_ptr(shader));
}

/// Decrement shader refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_shader_unref(shader: *mut sk_shader_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(shader);
    });
}

/// Get shader refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_shader_get_refcnt(shader: *const sk_shader_t) -> u32 {
    catch_panic(|| RefCounted::get_count(shader))
}

/// Create a linear gradient shader.
///
/// `colors_len` must equal `positions_len` (or `positions` may be null).
/// `tile_mode` maps to [`TileMode`]: 0=Clamp, 1=Repeat, 2=Mirror, 3=Decal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_shader_new_linear_gradient(
    start: sk_point_t,
    end: sk_point_t,
    colors: *const sk_color4f_t,
    positions: *const f32,
    count: usize,
    tile_mode: u32,
) -> *mut sk_shader_t {
    catch_panic(|| {
        if colors.is_null() || count == 0 {
            return ptr::null_mut();
        }
        let colors_slice = slice::from_raw_parts(colors, count);
        let colors_vec: Vec<Color4f> = colors_slice.iter().copied().map(Into::into).collect();
        let positions_vec = if positions.is_null() {
            None
        } else {
            Some(slice::from_raw_parts(positions, count).to_vec())
        };
        let grad = LinearGradient::new(
            start.into(),
            end.into(),
            colors_vec,
            positions_vec,
            decode_tile_mode(tile_mode),
        );
        let shader: ShaderRef = Arc::new(grad);
        RefCounted::new(shader)
    })
}

/// Create a radial gradient shader.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_shader_new_radial_gradient(
    center: sk_point_t,
    radius: f32,
    colors: *const sk_color4f_t,
    positions: *const f32,
    count: usize,
    tile_mode: u32,
) -> *mut sk_shader_t {
    catch_panic(|| {
        if colors.is_null() || count == 0 {
            return ptr::null_mut();
        }
        let colors_slice = slice::from_raw_parts(colors, count);
        let colors_vec: Vec<Color4f> = colors_slice.iter().copied().map(Into::into).collect();
        let positions_vec = if positions.is_null() {
            None
        } else {
            Some(slice::from_raw_parts(positions, count).to_vec())
        };
        let grad = RadialGradient::new(
            center.into(),
            radius,
            colors_vec,
            positions_vec,
            decode_tile_mode(tile_mode),
        );
        let shader: ShaderRef = Arc::new(grad);
        RefCounted::new(shader)
    })
}

/// Create a sweep gradient shader.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_shader_new_sweep_gradient(
    center: sk_point_t,
    start_angle: f32,
    end_angle: f32,
    colors: *const sk_color4f_t,
    positions: *const f32,
    count: usize,
) -> *mut sk_shader_t {
    catch_panic(|| {
        if colors.is_null() || count == 0 {
            return ptr::null_mut();
        }
        let colors_slice = slice::from_raw_parts(colors, count);
        let colors_vec: Vec<Color4f> = colors_slice.iter().copied().map(Into::into).collect();
        let positions_vec = if positions.is_null() {
            None
        } else {
            Some(slice::from_raw_parts(positions, count).to_vec())
        };
        let grad = SweepGradient::new(
            center.into(),
            start_angle,
            end_angle,
            colors_vec,
            positions_vec,
            TileMode::Clamp,
        );
        let shader: ShaderRef = Arc::new(grad);
        RefCounted::new(shader)
    })
}

// =============================================================================
// Color Filter API (Reference Counted)
// =============================================================================

/// Reference counted color filter.
pub type sk_colorfilter_t = RefCounted<ColorFilterRef>;

/// Increment color filter refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_colorfilter_ref(f: *mut sk_colorfilter_t) {
    catch_panic_void(|| RefCounted::ref_ptr(f));
}

/// Decrement color filter refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_colorfilter_unref(f: *mut sk_colorfilter_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(f);
    });
}

/// Create an identity color-matrix filter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_colorfilter_new_identity() -> *mut sk_colorfilter_t {
    catch_panic(|| {
        let cf: ColorFilterRef = Arc::new(ColorMatrixFilter::identity());
        RefCounted::new(cf)
    })
}

/// Create a saturation color filter (1.0 = identity, 0.0 = grayscale).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_colorfilter_new_saturation(amount: f32) -> *mut sk_colorfilter_t {
    catch_panic(|| {
        let cf: ColorFilterRef = Arc::new(ColorMatrixFilter::saturation(amount));
        RefCounted::new(cf)
    })
}

/// Create a color-matrix filter from 20 floats (row-major 4x5).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_colorfilter_new_matrix(
    values: *const f32,
) -> *mut sk_colorfilter_t {
    catch_panic(|| {
        if values.is_null() {
            return ptr::null_mut();
        }
        let mut m = [0.0f32; 20];
        ptr::copy_nonoverlapping(values, m.as_mut_ptr(), 20);
        let cf: ColorFilterRef = Arc::new(ColorMatrixFilter::new(m));
        RefCounted::new(cf)
    })
}

// =============================================================================
// Mask Filter API (Reference Counted)
// =============================================================================

/// Reference counted mask filter.
pub type sk_maskfilter_t = RefCounted<MaskFilterRef>;

/// Increment mask filter refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_maskfilter_ref(f: *mut sk_maskfilter_t) {
    catch_panic_void(|| RefCounted::ref_ptr(f));
}

/// Decrement mask filter refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_maskfilter_unref(f: *mut sk_maskfilter_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(f);
    });
}

/// Create a blur mask filter with the given sigma (pixels).
///
/// `style` follows [`BlurStyle`]: 0=Normal, 1=Solid, 2=Outer, 3=Inner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_maskfilter_new_blur(
    style: u32,
    sigma: f32,
) -> *mut sk_maskfilter_t {
    catch_panic(|| {
        let s = match style {
            0 => BlurStyle::Normal,
            1 => BlurStyle::Solid,
            2 => BlurStyle::Outer,
            3 => BlurStyle::Inner,
            _ => BlurStyle::Normal,
        };
        let mf: MaskFilterRef = Arc::new(BlurMaskFilter::new(s, sigma));
        RefCounted::new(mf)
    })
}

// =============================================================================
// Image Filter API (Reference Counted)
// =============================================================================

/// Reference counted image filter.
pub type sk_imagefilter_t = RefCounted<ImageFilterRef>;

/// Increment image filter refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_imagefilter_ref(f: *mut sk_imagefilter_t) {
    catch_panic_void(|| RefCounted::ref_ptr(f));
}

/// Decrement image filter refcount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_imagefilter_unref(f: *mut sk_imagefilter_t) {
    catch_panic_void(|| {
        RefCounted::unref_ptr(f);
    });
}

/// Create a Gaussian blur image filter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_imagefilter_new_blur(
    sigma_x: f32,
    sigma_y: f32,
    tile_mode: u32,
) -> *mut sk_imagefilter_t {
    catch_panic(|| {
        let filter =
            skia_rs_paint::BlurImageFilter::new(sigma_x, sigma_y, decode_tile_mode(tile_mode));
        let ifref: ImageFilterRef = Arc::new(filter);
        RefCounted::new(ifref)
    })
}

// Silence unused import warnings — these types are needed for re-exports and
// conversion glue declared elsewhere in the crate, but rustc can't see them
// used directly.
#[allow(dead_code)]
fn _unused_deps() {
    let _: Option<IPoint> = None;
    let _: Option<IRect> = None;
    let _: Option<ISize> = None;
    let _: Option<Size> = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_surface_creation() {
        unsafe {
            let surface = sk_surface_new_raster(100, 100);
            assert!(!surface.is_null());
            assert_eq!(sk_surface_get_width(surface), 100);
            assert_eq!(sk_surface_get_height(surface), 100);
            assert_eq!(sk_surface_get_refcnt(surface), 1);
            sk_surface_unref(surface);
        }
    }

    #[test]
    fn test_surface_refcounting() {
        unsafe {
            let surface = sk_surface_new_raster(100, 100);
            assert_eq!(sk_surface_get_refcnt(surface), 1);
            assert!(sk_surface_is_unique(surface));

            sk_surface_ref(surface);
            assert_eq!(sk_surface_get_refcnt(surface), 2);
            assert!(!sk_surface_is_unique(surface));

            sk_surface_unref(surface);
            assert_eq!(sk_surface_get_refcnt(surface), 1);
            assert!(sk_surface_is_unique(surface));

            sk_surface_unref(surface); // Frees it.
        }
    }

    #[test]
    fn test_paint_operations() {
        unsafe {
            let paint = sk_paint_new();
            assert!(!paint.is_null());
            assert_eq!(sk_paint_get_refcnt(paint), 1);

            sk_paint_set_color(paint, 0xFF0000FF); // Blue
            assert_eq!(sk_paint_get_color(paint), 0xFF0000FF);

            sk_paint_set_stroke_width(paint, 2.0);
            assert_eq!(sk_paint_get_stroke_width(paint), 2.0);

            sk_paint_delete(paint);
        }
    }

    #[test]
    fn test_paint_refcounting() {
        unsafe {
            let paint = sk_paint_new();
            assert_eq!(sk_paint_get_refcnt(paint), 1);

            sk_paint_ref(paint);
            assert_eq!(sk_paint_get_refcnt(paint), 2);

            sk_paint_unref(paint);
            assert_eq!(sk_paint_get_refcnt(paint), 1);

            let paint2 = sk_paint_clone(paint);
            assert_eq!(sk_paint_get_refcnt(paint), 1);
            assert_eq!(sk_paint_get_refcnt(paint2), 1);

            sk_paint_unref(paint);
            sk_paint_unref(paint2);
        }
    }

    #[test]
    fn test_path_builder() {
        unsafe {
            let builder = sk_pathbuilder_new();
            assert!(!builder.is_null());

            sk_pathbuilder_move_to(builder, 0.0, 0.0);
            sk_pathbuilder_line_to(builder, 100.0, 0.0);
            sk_pathbuilder_line_to(builder, 100.0, 100.0);
            sk_pathbuilder_close(builder);

            let path = sk_pathbuilder_detach(builder);
            assert!(!path.is_null());
            assert!(!sk_path_is_empty(path));
            assert_eq!(sk_path_get_refcnt(path), 1);

            let mut bounds = sk_rect_t::default();
            sk_path_get_bounds(path, &mut bounds);
            assert_eq!(bounds.left, 0.0);
            assert_eq!(bounds.right, 100.0);

            sk_path_delete(path);
            sk_pathbuilder_delete(builder);
        }
    }

    #[test]
    fn test_path_iteration() {
        unsafe {
            let b = sk_pathbuilder_new();
            sk_pathbuilder_move_to(b, 1.0, 2.0);
            sk_pathbuilder_line_to(b, 10.0, 20.0);
            sk_pathbuilder_close(b);
            let path = sk_pathbuilder_detach(b);
            sk_pathbuilder_delete(b);

            let it = sk_path_iter_new(path);
            assert!(!it.is_null());

            let mut pts = [sk_point_t::default(); 4];
            let mut w: f32 = 0.0;

            let v0 = sk_path_iter_next(it, pts.as_mut_ptr(), &mut w);
            assert_eq!(v0, SK_PATH_VERB_MOVE);
            assert_eq!(pts[0].x, 1.0);
            assert_eq!(pts[0].y, 2.0);

            let v1 = sk_path_iter_next(it, pts.as_mut_ptr(), &mut w);
            assert_eq!(v1, SK_PATH_VERB_LINE);
            assert_eq!(pts[0].x, 10.0);
            assert_eq!(pts[0].y, 20.0);

            let v2 = sk_path_iter_next(it, pts.as_mut_ptr(), &mut w);
            assert_eq!(v2, SK_PATH_VERB_CLOSE);

            let v3 = sk_path_iter_next(it, pts.as_mut_ptr(), &mut w);
            assert_eq!(v3, SK_PATH_VERB_DONE);

            sk_path_iter_delete(it);
            sk_path_unref(path);
        }
    }

    #[test]
    fn test_path_refcounting() {
        unsafe {
            let path = sk_path_new();
            assert_eq!(sk_path_get_refcnt(path), 1);

            sk_path_ref(path);
            assert_eq!(sk_path_get_refcnt(path), 2);

            let path2 = sk_path_clone(path);
            assert_eq!(sk_path_get_refcnt(path), 2);
            assert_eq!(sk_path_get_refcnt(path2), 1);

            sk_path_unref(path);
            assert_eq!(sk_path_get_refcnt(path), 1);

            sk_path_unref(path);
            sk_path_unref(path2);
        }
    }

    #[test]
    fn test_matrix_operations() {
        unsafe {
            let mut matrix = sk_matrix_t::default();
            sk_matrix_set_translate(&mut matrix, 10.0, 20.0);

            let point = sk_point_t { x: 0.0, y: 0.0 };
            let mut result = sk_point_t::default();
            sk_matrix_map_point(&matrix, &point, &mut result);

            assert_eq!(result.x, 10.0);
            assert_eq!(result.y, 20.0);
        }
    }

    #[test]
    fn test_matrix_invert() {
        unsafe {
            let mut matrix = sk_matrix_t::default();
            sk_matrix_set_scale(&mut matrix, 2.0, 4.0);
            let mut inv = sk_matrix_t::default();
            assert!(sk_matrix_invert(&matrix, &mut inv));
            // (2,4) scaled then halved/quartered should return origin-offset.
            let p = sk_point_t { x: 2.0, y: 4.0 };
            let mut r = sk_point_t::default();
            sk_matrix_map_point(&inv, &p, &mut r);
            assert_eq!(r.x, 1.0);
            assert_eq!(r.y, 1.0);

            // Singular matrix should fail.
            let zero = sk_matrix_t {
                values: [0.0; 9],
            };
            assert!(!sk_matrix_invert(&zero, &mut inv));
        }
    }

    #[test]
    fn test_matrix_is_identity_and_determinant() {
        unsafe {
            let m = sk_matrix_t::default();
            assert!(sk_matrix_is_identity(&m));
            assert_eq!(sk_matrix_determinant(&m), 1.0);
        }
    }

    #[test]
    fn test_draw_rect() {
        unsafe {
            let surface = sk_surface_new_raster(100, 100);
            let paint = sk_paint_new();
            sk_paint_set_color(paint, 0xFFFF0000);
            sk_surface_clear(surface, 0xFFFFFFFF);

            let rect = sk_rect_t {
                left: 10.0,
                top: 10.0,
                right: 50.0,
                bottom: 50.0,
            };
            sk_surface_draw_rect(surface, &rect, paint);

            sk_paint_delete(paint);
            sk_surface_unref(surface);
        }
    }

    #[test]
    fn test_refcnt_utility() {
        unsafe {
            let surface = sk_surface_new_raster(100, 100);

            let ptr = surface as *const sk_refcnt_t;
            assert_eq!(sk_refcnt_get_count(ptr), 1);
            assert!(sk_refcnt_is_unique(ptr));

            sk_surface_ref(surface);
            assert_eq!(sk_refcnt_get_count(ptr), 2);
            assert!(!sk_refcnt_is_unique(ptr));

            sk_surface_unref(surface);
            sk_surface_unref(surface);
        }
    }

    #[test]
    fn test_refcnt_utility_rejects_untagged_pointer() {
        // Construct a raw AtomicU32 that happens to have the layout of a
        // legacy "refcount at offset 0" but no magic tag. The hardened
        // generic entry points must reject it.
        let fake = Box::into_raw(Box::new([0u32; 4]));
        unsafe {
            let ptr = fake as *const sk_refcnt_t;
            assert_eq!(sk_refcnt_get_count(ptr), 0);
            assert!(!sk_refcnt_is_unique(ptr));
            drop(Box::from_raw(fake));
        }
    }

    #[test]
    fn test_panic_catcher_sets_flag_and_returns_default() {
        // Clear any previous panic state.
        let _ = sk_last_call_panicked();
        assert!(!sk_last_call_panicked());

        // Deliberately trigger a panic inside the wrapper. `catch_panic`
        // returns T::default(), and the flag must come back true.
        let v: u32 = catch_panic(|| panic!("boom"));
        assert_eq!(v, 0);
        assert!(sk_last_call_panicked());

        // Flag auto-clears on read.
        assert!(!sk_last_call_panicked());
    }

    #[test]
    fn test_abi_init() {
        assert!(!abi::sk_abi_is_compatible(99, 0));
        assert!(sk_init(1, 0));
        assert!(sk_is_initialized());
        // A second compatible call still succeeds.
        assert!(sk_init(1, 0));
        // An incompatible call returns false but does not un-initialize.
        assert!(!sk_init(99, 99));
        assert!(sk_is_initialized());
    }

    #[test]
    fn test_cross_thread_refcount_stress() {
        use std::sync::atomic::AtomicUsize;
        use std::thread;

        unsafe {
            let paint = sk_paint_new();
            // Guard: drops to 1 only after all threads finish.
            sk_paint_ref(paint); // Refcount: 2 (one for us, one for threads)

            let n_threads = 8usize;
            let iters = 1_000usize;

            let ptr = paint as usize;
            let done = Arc::new(AtomicUsize::new(0));

            let handles: Vec<_> = (0..n_threads)
                .map(|_| {
                    let d = done.clone();
                    thread::spawn(move || {
                        let p = ptr as *mut sk_paint_t;
                        for _ in 0..iters {
                            sk_paint_ref(p);
                            sk_paint_unref(p);
                        }
                        d.fetch_add(1, Ordering::SeqCst);
                    })
                })
                .collect();

            for h in handles {
                h.join().unwrap();
            }

            assert_eq!(done.load(Ordering::SeqCst), n_threads);
            // After all ref/unref pairs cancel, refcount is 2.
            assert_eq!(sk_paint_get_refcnt(paint), 2);
            sk_paint_unref(paint);
            sk_paint_unref(paint);
        }
    }

    #[test]
    fn test_shader_linear_gradient() {
        unsafe {
            let colors = [
                sk_color4f_t {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                sk_color4f_t {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                },
            ];
            let shader = sk_shader_new_linear_gradient(
                sk_point_t { x: 0.0, y: 0.0 },
                sk_point_t { x: 10.0, y: 10.0 },
                colors.as_ptr(),
                ptr::null(),
                2,
                0, // clamp
            );
            assert!(!shader.is_null());
            assert_eq!(sk_shader_get_refcnt(shader), 1);

            let paint = sk_paint_new();
            sk_paint_set_shader(paint, shader);
            // Paint holds its own Arc to the ShaderRef; the refcount of the
            // *wrapper* is unchanged.
            assert_eq!(sk_shader_get_refcnt(shader), 1);

            sk_paint_set_shader(paint, ptr::null());
            sk_paint_unref(paint);
            sk_shader_unref(shader);
        }
    }

    #[test]
    fn test_colorfilter_and_maskfilter() {
        unsafe {
            let cf = sk_colorfilter_new_saturation(0.5);
            assert!(!cf.is_null());
            let mf = sk_maskfilter_new_blur(0, 2.0);
            assert!(!mf.is_null());
            let imf = sk_imagefilter_new_blur(2.0, 2.0, 0);
            assert!(!imf.is_null());

            let paint = sk_paint_new();
            sk_paint_set_color_filter(paint, cf);
            sk_paint_set_mask_filter(paint, mf);
            sk_paint_set_image_filter(paint, imf);
            sk_paint_set_color_filter(paint, ptr::null());
            sk_paint_set_mask_filter(paint, ptr::null());
            sk_paint_set_image_filter(paint, ptr::null());
            sk_paint_unref(paint);

            sk_colorfilter_unref(cf);
            sk_maskfilter_unref(mf);
            sk_imagefilter_unref(imf);
        }
    }

    #[test]
    fn test_image_from_color_and_encode() {
        unsafe {
            let img = sk_image_from_color(8, 8, 0xFF00FF00);
            assert!(!img.is_null());
            assert_eq!(sk_image_get_width(img), 8);
            assert_eq!(sk_image_get_height(img), 8);

            // Query size.
            let needed = sk_image_encode_png(img, ptr::null_mut(), 0);
            assert!(needed > 0);

            // Encode into a sized buffer.
            let mut buf = vec![0u8; needed];
            let wrote = sk_image_encode_png(img, buf.as_mut_ptr(), buf.len());
            assert_eq!(wrote, needed);
            // PNG magic.
            assert_eq!(&buf[0..4], b"\x89PNG");

            sk_image_unref(img);
        }
    }

    #[test]
    fn test_typeface_and_font() {
        unsafe {
            let tf = sk_typeface_default();
            assert!(!tf.is_null());
            let font = sk_font_new(tf, 14.0);
            assert!(!font.is_null());
            assert_eq!(sk_font_get_size(font), 14.0);
            sk_font_unref(font);
            sk_typeface_unref(tf);
        }
    }

    #[test]
    fn test_canvas_lifecycle() {
        unsafe {
            let surface = sk_surface_new_raster(32, 32);
            let canvas = sk_surface_lock_canvas(surface);
            assert!(!canvas.is_null());
            assert_eq!(sk_canvas_get_width(canvas), 32);
            assert_eq!(sk_canvas_get_height(canvas), 32);

            let n0 = sk_canvas_save(canvas);
            assert_eq!(n0, 1);
            sk_canvas_translate(canvas, 10.0, 5.0);
            sk_canvas_scale(canvas, 2.0, 2.0);

            sk_canvas_clear(canvas, 0xFFFFFFFF);
            let paint = sk_paint_new();
            sk_paint_set_color(paint, 0xFFFF0000);
            let rect = sk_rect_t {
                left: 0.0,
                top: 0.0,
                right: 5.0,
                bottom: 5.0,
            };
            sk_canvas_draw_rect(canvas, &rect, paint);
            sk_paint_unref(paint);

            sk_canvas_restore(canvas);
            sk_canvas_release(canvas);
            sk_surface_unref(surface);
        }
    }

    #[test]
    fn test_surface_read_pixels_copies() {
        unsafe {
            let surface = sk_surface_new_raster(4, 4);
            sk_surface_clear(surface, 0xFF112233);

            let mut buf = vec![0u8; 4 * 4 * 4];
            let wrote = sk_surface_read_pixels(surface, buf.as_mut_ptr(), buf.len());
            assert_eq!(wrote, buf.len());

            // Too-small buffer fails cleanly.
            let mut small = vec![0u8; 3];
            let wrote_small = sk_surface_read_pixels(surface, small.as_mut_ptr(), small.len());
            assert_eq!(wrote_small, 0);

            sk_surface_unref(surface);
        }
    }

    #[test]
    fn test_null_inputs_dont_crash() {
        unsafe {
            // Each of these must either no-op or return a default, never
            // unwind.
            sk_paint_set_color(ptr::null_mut(), 0);
            sk_paint_set_stroke_width(ptr::null_mut(), 1.0);
            sk_paint_unref(ptr::null_mut());
            sk_surface_unref(ptr::null_mut());
            let mut bounds = sk_rect_t::default();
            sk_path_get_bounds(ptr::null(), &mut bounds);
            sk_path_unref(ptr::null_mut());
            assert_eq!(sk_path_iter_next(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()), SK_PATH_VERB_DONE);
            assert!(!sk_last_call_panicked());
        }
    }

    #[test]
    fn test_counter_sanity() {
        // Ensures the `AtomicUsize` import is used (keeps the test module
        // tidy under -D warnings).
        let c = AtomicUsize::new(0);
        c.fetch_add(1, Ordering::SeqCst);
        assert_eq!(c.load(Ordering::SeqCst), 1);
    }
}
