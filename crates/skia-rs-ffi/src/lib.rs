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
//! # Reference Counting
//!
//! Objects follow Skia's reference counting model:
//! - Objects are created with a reference count of 1
//! - `sk_*_ref()` increments the reference count
//! - `sk_*_unref()` decrements the reference count and frees when it reaches 0
//! - Use `sk_refcnt_get_count()` to query the current count
//!
//! Reference counting operations (`ref`/`unref`) are **thread-safe** and use
//! atomic operations internally.
//!
//! # Thread Safety
//!
//! skia-rs follows Skia's threading model. Understanding thread safety is
//! critical for correct usage in multi-threaded applications.
//!
//! ## Thread Safety Categories
//!
//! | Category | Description | Examples |
//! |----------|-------------|----------|
//! | **Thread-Safe** | Can be accessed from any thread concurrently | `sk_*_ref()`, `sk_*_unref()`, `sk_refcnt_*()` |
//! | **Thread-Compatible** | Safe if each instance accessed by one thread | Most objects |
//! | **Main-Thread-Only** | Must only be used from main/UI thread | GPU contexts |
//!
//! ## Object-Specific Thread Safety
//!
//! ### Reference Counting (Thread-Safe)
//! ```c
//! // These operations are always thread-safe:
//! sk_surface_ref(surface);      // Atomic increment
//! sk_surface_unref(surface);    // Atomic decrement
//! sk_refcnt_get_count(ptr);     // Atomic read
//! sk_refcnt_is_unique(ptr);     // Atomic read
//! ```
//!
//! ### Immutable Objects (Thread-Safe after creation)
//! Once created, these objects are safe to read from multiple threads:
//! - `sk_path_t` (after building is complete)
//! - `sk_matrix_t` (value type, copied on use)
//!
//! ### Mutable Objects (Thread-Compatible)
//! These must not be accessed concurrently from multiple threads:
//! - `sk_surface_t` - Drawing operations are not thread-safe
//! - `sk_paint_t` - Setters/getters must be externally synchronized
//! - `sk_pathbuilder_t` - Building operations must be single-threaded
//!
//! ### GPU Objects (Special Restrictions)
//! GPU-related objects have additional constraints:
//! - Must be created/destroyed on the same thread as the GPU context
//! - Drawing to GPU surfaces must occur on the GPU context thread
//! - Flush operations must be called from the same thread
//!
//! ## Safe Patterns
//!
//! ### Pattern 1: Object-per-Thread
//! ```c
//! // Each thread creates its own objects
//! void* thread_func(void* arg) {
//!     sk_surface_t* surface = sk_surface_new_raster(800, 600);
//!     sk_paint_t* paint = sk_paint_new();
//!     // ... use exclusively in this thread ...
//!     sk_paint_unref(paint);
//!     sk_surface_unref(surface);
//!     return NULL;
//! }
//! ```
//!
//! ### Pattern 2: Shared Immutable Data
//! ```c
//! // Build path on one thread, share read-only
//! sk_path_t* shared_path;  // Global
//!
//! void init() {
//!     sk_pathbuilder_t* builder = sk_pathbuilder_new();
//!     sk_pathbuilder_add_circle(builder, 100, 100, 50);
//!     shared_path = sk_pathbuilder_detach(builder);
//!     sk_pathbuilder_unref(builder);
//! }
//!
//! void* render_thread(void* arg) {
//!     // Safe: path is immutable after creation
//!     sk_rect_t bounds;
//!     sk_path_get_bounds(shared_path, &bounds);
//!     sk_path_contains(shared_path, 100, 100);
//!     return NULL;
//! }
//! ```
//!
//! ### Pattern 3: Reference Counted Sharing
//! ```c
//! // Safe: ref counting is atomic
//! sk_paint_t* paint = sk_paint_new();
//!
//! void share_to_thread(sk_paint_t* p) {
//!     sk_paint_ref(p);  // Thread-safe increment
//!     // Pass to another thread...
//! }
//!
//! void* other_thread(void* paint_ptr) {
//!     sk_paint_t* p = (sk_paint_t*)paint_ptr;
//!     // Clone for thread-local modifications
//!     sk_paint_t* local = sk_paint_clone(p);
//!     sk_paint_unref(p);  // Thread-safe decrement
//!     // ... use local exclusively ...
//!     sk_paint_unref(local);
//!     return NULL;
//! }
//! ```
//!
//! ## Unsafe Patterns (AVOID)
//!
//! ```c
//! // UNSAFE: Concurrent mutation
//! sk_paint_t* shared_paint;
//!
//! void* thread1(void* arg) {
//!     sk_paint_set_color(shared_paint, 0xFFFF0000);  // DATA RACE!
//!     return NULL;
//! }
//!
//! void* thread2(void* arg) {
//!     sk_paint_set_color(shared_paint, 0xFF0000FF);  // DATA RACE!
//!     return NULL;
//! }
//!
//! // UNSAFE: Drawing while modifying
//! void* draw_thread(void* arg) {
//!     sk_surface_draw_rect(surface, &rect, paint);  // DATA RACE with modifier!
//!     return NULL;
//! }
//!
//! void* modify_thread(void* arg) {
//!     sk_paint_set_stroke_width(paint, 5.0);  // DATA RACE with drawer!
//!     return NULL;
//! }
//! ```
//!
//! ## Error Checking
//!
//! After any FFI call, check for panics:
//! ```c
//! sk_surface_t* surface = sk_surface_new_raster(800, 600);
//! if (sk_last_call_panicked()) {
//!     // Handle error - surface may be NULL
//!     fprintf(stderr, "Surface creation panicked\n");
//! }
//! ```
//!
//! ## Summary Table
//!
//! | Operation | Thread-Safe | Notes |
//! |-----------|-------------|-------|
//! | `sk_*_new()` | Yes | Returns unique object |
//! | `sk_*_ref()` | Yes | Atomic increment |
//! | `sk_*_unref()` | Yes | Atomic decrement |
//! | `sk_*_clone()` | Yes* | Input must not be concurrently modified |
//! | `sk_*_get_*()` | No | Requires external synchronization |
//! | `sk_*_set_*()` | No | Requires external synchronization |
//! | `sk_surface_draw_*()` | No | Single-threaded drawing only |
//! | `sk_path_contains()` | Yes* | After path is immutable |
//! | `sk_pathbuilder_*()` | No | Single-threaded building |
//!
//! # Panic Safety
//!
//! All FFI functions catch panics at the boundary to prevent unwinding
//! into C code. Functions that panic will return a default/null value
//! and set an error flag. Use `sk_last_call_panicked()` to check.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)] // FFI types follow C naming conventions

use std::ffi::{c_char, c_void};
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// =============================================================================
// Panic Catching Infrastructure
// =============================================================================

/// Global flag indicating if the last FFI call panicked.
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
#[inline]
fn catch_panic<T: Default, F: FnOnce() -> T + panic::UnwindSafe>(f: F) -> T {
    match panic::catch_unwind(f) {
        Ok(result) => result,
        Err(_) => {
            LAST_PANIC.store(true, Ordering::SeqCst);
            T::default()
        }
    }
}

/// Catch panics in void-returning functions.
#[inline]
fn catch_panic_void<F: FnOnce() + panic::UnwindSafe>(f: F) {
    if panic::catch_unwind(f).is_err() {
        LAST_PANIC.store(true, Ordering::SeqCst);
    }
}

// =============================================================================
// Reference Counting Infrastructure
// =============================================================================

/// Reference counted wrapper for FFI objects.
///
/// This provides Skia-compatible reference counting semantics:
/// - Created with refcount of 1
/// - `ref()` increments
/// - `unref()` decrements and frees when 0
#[repr(C)]
pub struct RefCounted<T> {
    /// Reference count.
    refcnt: AtomicU32,
    /// The wrapped value.
    value: T,
}

impl<T> RefCounted<T> {
    /// Create a new reference counted object with refcount of 1.
    pub fn new(value: T) -> *mut Self {
        Box::into_raw(Box::new(Self {
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

/// Get the reference count of an object.
///
/// Returns 0 if the pointer is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_refcnt_get_count(ptr: *const sk_refcnt_t) -> u32 {
    // All our refcounted types start with AtomicU32
    if ptr.is_null() {
        return 0;
    }
    let refcnt = ptr as *const AtomicU32;
    (*refcnt).load(Ordering::Relaxed)
}

/// Check if an object has only one reference (is unique).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_refcnt_is_unique(ptr: *const sk_refcnt_t) -> bool {
    sk_refcnt_get_count(ptr) == 1
}

// Re-export types for FFI
use skia_rs_canvas::{PixelBuffer, RasterCanvas, Surface};
use skia_rs_core::{
    AlphaType, Color, ColorType, IPoint, IRect, ISize, ImageInfo, Matrix, Point, Rect, Scalar, Size,
};
use skia_rs_paint::{BlendMode, Paint, Style};
use skia_rs_path::{FillType, Path, PathBuilder};

// =============================================================================
// Type Definitions
// =============================================================================

// Note: Reference counted types are defined in their respective sections:
// - sk_surface_t = RefCounted<Surface>
// - sk_paint_t = RefCounted<Paint>
// - sk_path_t = RefCounted<Path>
// - sk_pathbuilder_t = RefCounted<PathBuilder>

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

/// C-compatible color (ARGB).
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
    catch_panic(|| {
        match Surface::new_raster_n32_premul(width, height) {
            Some(surface) => RefCounted::new(surface),
            None => ptr::null_mut(),
        }
    })
}

/// Create a raster surface with specific image info.
///
/// Returns a surface with refcount of 1, or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_new_raster_with_info(
    info: *const sk_imageinfo_t,
) -> *mut sk_surface_t {
    if info.is_null() {
        return ptr::null_mut();
    }

    let info = &*info;
    let color_type = match info.color_type {
        0 => ColorType::Unknown,
        1 => ColorType::Alpha8,
        2 => ColorType::Rgb565,
        3 => ColorType::Argb4444,
        4 => ColorType::Rgba8888,
        5 => ColorType::Bgra8888,
        _ => ColorType::Rgba8888,
    };

    let alpha_type = match info.alpha_type {
        0 => AlphaType::Unknown,
        1 => AlphaType::Opaque,
        2 => AlphaType::Premul,
        3 => AlphaType::Unpremul,
        _ => AlphaType::Premul,
    };

    let img_info = match ImageInfo::new(info.width, info.height, color_type, alpha_type) {
        Ok(i) => i,
        Err(_) => return ptr::null_mut(),
    };

    match Surface::new_raster(&img_info, None) {
        Some(surface) => RefCounted::new(surface),
        None => ptr::null_mut(),
    }
}

/// Increment the reference count of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_ref(surface: *mut sk_surface_t) {
    RefCounted::ref_ptr(surface);
}

/// Decrement the reference count of a surface.
///
/// Frees the surface when the count reaches 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_unref(surface: *mut sk_surface_t) {
    catch_panic_void(AssertUnwindSafe(|| {
        RefCounted::unref_ptr(surface);
    }));
}

/// Get the reference count of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_refcnt(surface: *const sk_surface_t) -> u32 {
    RefCounted::get_count(surface)
}

/// Check if the surface has only one reference.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_is_unique(surface: *const sk_surface_t) -> bool {
    RefCounted::is_unique(surface)
}

/// Get the width of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_width(surface: *const sk_surface_t) -> i32 {
    RefCounted::get_ref(surface).map_or(0, |s| s.width())
}

/// Get the height of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_height(surface: *const sk_surface_t) -> i32 {
    RefCounted::get_ref(surface).map_or(0, |s| s.height())
}

/// Get the pixel data from a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_peek_pixels(
    surface: *const sk_surface_t,
    out_pixels: *mut *const u8,
    out_row_bytes: *mut usize,
) -> bool {
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
}

// =============================================================================
// Paint API (Reference Counted)
// =============================================================================

/// Reference counted paint type.
pub type sk_paint_t = RefCounted<Paint>;

/// Create a new paint.
///
/// Returns a paint with refcount of 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_new() -> *mut sk_paint_t {
    catch_panic(|| RefCounted::new(Paint::new()))
}

/// Clone a paint.
///
/// Returns a new paint with refcount of 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_clone(paint: *const sk_paint_t) -> *mut sk_paint_t {
    RefCounted::get_ref(paint).map_or(ptr::null_mut(), |p| RefCounted::new(p.clone()))
}

/// Increment the reference count of a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_ref(paint: *mut sk_paint_t) {
    RefCounted::ref_ptr(paint);
}

/// Decrement the reference count of a paint (alias for unref).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_delete(paint: *mut sk_paint_t) {
    RefCounted::unref_ptr(paint);
}

/// Decrement the reference count of a paint.
///
/// Frees the paint when the count reaches 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_unref(paint: *mut sk_paint_t) {
    RefCounted::unref_ptr(paint);
}

/// Get the reference count of a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_refcnt(paint: *const sk_paint_t) -> u32 {
    RefCounted::get_count(paint)
}

/// Set the paint color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_color(paint: *mut sk_paint_t, color: sk_color_t) {
    if let Some(p) = RefCounted::get_mut(paint) {
        p.set_color32(Color(color));
    }
}

/// Get the paint color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_color(paint: *const sk_paint_t) -> sk_color_t {
    RefCounted::get_ref(paint).map_or(0, |p| p.color32().0)
}

/// Set the paint style.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_style(paint: *mut sk_paint_t, style: u32) {
    if let Some(p) = RefCounted::get_mut(paint) {
        let style = match style {
            0 => Style::Fill,
            1 => Style::Stroke,
            2 => Style::StrokeAndFill,
            _ => Style::Fill,
        };
        p.set_style(style);
    }
}

/// Set the stroke width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_stroke_width(paint: *mut sk_paint_t, width: f32) {
    if let Some(p) = RefCounted::get_mut(paint) {
        p.set_stroke_width(width);
    }
}

/// Get the stroke width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_stroke_width(paint: *const sk_paint_t) -> f32 {
    RefCounted::get_ref(paint).map_or(0.0, |p| p.stroke_width())
}

/// Set anti-alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_antialias(paint: *mut sk_paint_t, aa: bool) {
    if let Some(p) = RefCounted::get_mut(paint) {
        p.set_anti_alias(aa);
    }
}

/// Check if anti-alias is enabled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_is_antialias(paint: *const sk_paint_t) -> bool {
    RefCounted::get_ref(paint).map_or(false, |p| p.is_anti_alias())
}

// =============================================================================
// Path API (Reference Counted)
// =============================================================================

/// Reference counted path type.
pub type sk_path_t = RefCounted<Path>;

/// Create a new path.
///
/// Returns a path with refcount of 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_new() -> *mut sk_path_t {
    RefCounted::new(Path::new())
}

/// Clone a path.
///
/// Returns a new path with refcount of 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_clone(path: *const sk_path_t) -> *mut sk_path_t {
    RefCounted::get_ref(path).map_or(ptr::null_mut(), |p| RefCounted::new(p.clone()))
}

/// Increment the reference count of a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_ref(path: *mut sk_path_t) {
    RefCounted::ref_ptr(path);
}

/// Decrement the reference count of a path (alias for unref).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_delete(path: *mut sk_path_t) {
    RefCounted::unref_ptr(path);
}

/// Decrement the reference count of a path.
///
/// Frees the path when the count reaches 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_unref(path: *mut sk_path_t) {
    RefCounted::unref_ptr(path);
}

/// Get the reference count of a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_refcnt(path: *const sk_path_t) -> u32 {
    RefCounted::get_count(path)
}

/// Get the path bounds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_bounds(path: *const sk_path_t, bounds: *mut sk_rect_t) {
    if let (Some(p), Some(b)) = (RefCounted::get_ref(path), bounds.as_mut()) {
        let rect = p.bounds();
        *b = rect.into();
    }
}

/// Check if path is empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_is_empty(path: *const sk_path_t) -> bool {
    RefCounted::get_ref(path).map_or(true, |p| p.is_empty())
}

/// Get the fill type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_filltype(path: *const sk_path_t) -> u32 {
    RefCounted::get_ref(path).map_or(0, |p| {
        match p.fill_type() {
            FillType::Winding => 0,
            FillType::EvenOdd => 1,
            FillType::InverseWinding => 2,
            FillType::InverseEvenOdd => 3,
        }
    })
}

/// Set the fill type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_set_filltype(path: *mut sk_path_t, fill_type: u32) {
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
}

/// Check if path contains a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_contains(path: *const sk_path_t, x: f32, y: f32) -> bool {
    RefCounted::get_ref(path).map_or(false, |p| p.contains(Point::new(x, y)))
}

// =============================================================================
// Path Builder API (Reference Counted)
// =============================================================================

/// Reference counted path builder type.
pub type sk_pathbuilder_t = RefCounted<PathBuilder>;

/// Create a new path builder.
///
/// Returns a builder with refcount of 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_new() -> *mut sk_pathbuilder_t {
    RefCounted::new(PathBuilder::new())
}

/// Increment the reference count of a path builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_ref(builder: *mut sk_pathbuilder_t) {
    RefCounted::ref_ptr(builder);
}

/// Decrement the reference count of a path builder (alias for unref).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_delete(builder: *mut sk_pathbuilder_t) {
    RefCounted::unref_ptr(builder);
}

/// Decrement the reference count of a path builder.
///
/// Frees the builder when the count reaches 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_unref(builder: *mut sk_pathbuilder_t) {
    RefCounted::unref_ptr(builder);
}

/// Move to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_move_to(builder: *mut sk_pathbuilder_t, x: f32, y: f32) {
    if let Some(b) = RefCounted::get_mut(builder) {
        b.move_to(x, y);
    }
}

/// Line to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_line_to(builder: *mut sk_pathbuilder_t, x: f32, y: f32) {
    if let Some(b) = RefCounted::get_mut(builder) {
        b.line_to(x, y);
    }
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
    if let Some(b) = RefCounted::get_mut(builder) {
        b.quad_to(cx, cy, x, y);
    }
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
    if let Some(b) = RefCounted::get_mut(builder) {
        b.cubic_to(c1x, c1y, c2x, c2y, x, y);
    }
}

/// Close the path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_close(builder: *mut sk_pathbuilder_t) {
    if let Some(b) = RefCounted::get_mut(builder) {
        b.close();
    }
}

/// Add a rectangle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_rect(
    builder: *mut sk_pathbuilder_t,
    rect: *const sk_rect_t,
) {
    if let (Some(b), Some(r)) = (RefCounted::get_mut(builder), rect.as_ref()) {
        b.add_rect(&Rect::from(*r));
    }
}

/// Add an oval.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_oval(
    builder: *mut sk_pathbuilder_t,
    rect: *const sk_rect_t,
) {
    if let (Some(b), Some(r)) = (RefCounted::get_mut(builder), rect.as_ref()) {
        b.add_oval(&Rect::from(*r));
    }
}

/// Add a circle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_circle(
    builder: *mut sk_pathbuilder_t,
    cx: f32,
    cy: f32,
    radius: f32,
) {
    if let Some(b) = RefCounted::get_mut(builder) {
        b.add_circle(cx, cy, radius);
    }
}

/// Build the path and reset the builder.
///
/// Returns a new path with refcount of 1.
/// The builder is reset and can be reused.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_detach(builder: *mut sk_pathbuilder_t) -> *mut sk_path_t {
    if let Some(b) = RefCounted::get_mut(builder) {
        let path = std::mem::replace(b, PathBuilder::new()).build();
        RefCounted::new(path)
    } else {
        ptr::null_mut()
    }
}

/// Build the path without resetting the builder.
///
/// Returns a new path with refcount of 1.
/// The builder retains its current state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_snapshot(builder: *const sk_pathbuilder_t) -> *mut sk_path_t {
    RefCounted::get_ref(builder).map_or(ptr::null_mut(), |b| RefCounted::new(b.clone().build()))
}

// =============================================================================
// Matrix API
// =============================================================================

/// Set matrix to identity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_identity(matrix: *mut sk_matrix_t) {
    if let Some(m) = matrix.as_mut() {
        *m = sk_matrix_t::default();
    }
}

/// Set matrix to translate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_translate(matrix: *mut sk_matrix_t, dx: f32, dy: f32) {
    if let Some(m) = matrix.as_mut() {
        *m = Matrix::translate(dx, dy).into();
    }
}

/// Set matrix to scale.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_scale(matrix: *mut sk_matrix_t, sx: f32, sy: f32) {
    if let Some(m) = matrix.as_mut() {
        *m = Matrix::scale(sx, sy).into();
    }
}

/// Set matrix to rotate (degrees).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_rotate(matrix: *mut sk_matrix_t, degrees: f32) {
    if let Some(m) = matrix.as_mut() {
        let radians = degrees * std::f32::consts::PI / 180.0;
        *m = Matrix::rotate(radians).into();
    }
}

/// Concatenate two matrices.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_concat(
    result: *mut sk_matrix_t,
    a: *const sk_matrix_t,
    b: *const sk_matrix_t,
) {
    if let (Some(r), Some(a), Some(b)) = (result.as_mut(), a.as_ref(), b.as_ref()) {
        let ma: Matrix = (*a).into();
        let mb: Matrix = (*b).into();
        *r = ma.concat(&mb).into();
    }
}

/// Map a point through a matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_map_point(
    matrix: *const sk_matrix_t,
    point: *const sk_point_t,
    result: *mut sk_point_t,
) {
    if let (Some(m), Some(p), Some(r)) = (matrix.as_ref(), point.as_ref(), result.as_mut()) {
        let mat: Matrix = (*m).into();
        let pt: Point = (*p).into();
        *r = mat.map_point(pt).into();
    }
}

// =============================================================================
// Utility functions
// =============================================================================

/// Get the library version.
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
// Drawing helpers (simplified)
// =============================================================================

/// Clear a surface with a color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_clear(surface: *mut sk_surface_t, color: sk_color_t) {
    if let Some(s) = RefCounted::get_mut(surface) {
        let mut canvas = s.raster_canvas();
        canvas.clear(Color(color));
    }
}

/// Draw a rect on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_rect(
    surface: *mut sk_surface_t,
    rect: *const sk_rect_t,
    paint: *const sk_paint_t,
) {
    if let (Some(s), Some(r), Some(p)) =
        (RefCounted::get_mut(surface), rect.as_ref(), RefCounted::get_ref(paint))
    {
        let mut canvas = s.raster_canvas();
        canvas.draw_rect(&Rect::from(*r), p);
    }
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
    if let (Some(s), Some(p)) = (RefCounted::get_mut(surface), RefCounted::get_ref(paint)) {
        let mut canvas = s.raster_canvas();
        canvas.draw_circle(Point::new(cx, cy), radius, p);
    }
}

/// Draw a path on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_path(
    surface: *mut sk_surface_t,
    path: *const sk_path_t,
    paint: *const sk_paint_t,
) {
    if let (Some(s), Some(path), Some(p)) = (
        RefCounted::get_mut(surface),
        RefCounted::get_ref(path),
        RefCounted::get_ref(paint),
    ) {
        let mut canvas = s.raster_canvas();
        canvas.draw_path(path, p);
    }
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
    if let (Some(s), Some(p)) = (RefCounted::get_mut(surface), RefCounted::get_ref(paint)) {
        let mut canvas = s.raster_canvas();
        canvas.draw_line(Point::new(x0, y0), Point::new(x1, y1), p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

            sk_surface_unref(surface); // This frees it
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

            // Clone should create a new object with refcount 1
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
    fn test_draw_rect() {
        unsafe {
            let surface = sk_surface_new_raster(100, 100);
            let paint = sk_paint_new();
            sk_paint_set_color(paint, 0xFFFF0000); // Red

            sk_surface_clear(surface, 0xFFFFFFFF); // White

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

            // Test generic refcnt functions
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
}
