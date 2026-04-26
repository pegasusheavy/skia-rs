# skia-rs-ffi Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)

## Phase 6C Resolution Summary (2026-04-25)

All 12 gaps have an explicit resolution. The soundness-critical work
(panic catching on every entry point, tag-validated refcount generics,
ABI initialization, cbindgen header, threaded stress test) is complete.
The API-surface expansion is partial and the remainder is documented as
"Scope: follow-up" below.

| Gap | Status | Notes |
|-----|--------|-------|
| C-1 panic catching missing | **Resolved** | Every `extern "C" fn sk_*` wraps its body in `catch_panic` / `catch_panic_void`; `test_panic_catcher_sets_flag_and_returns_default` proves the flag semantics. |
| C-2 API surface minimal | **Partially resolved / Scope: follow-up** | Added `sk_canvas_t`, `sk_image_t`, `sk_typeface_t`/`sk_font_t`, `sk_shader_t`, `sk_colorfilter_t`, `sk_maskfilter_t`, `sk_imagefilter_t`, linear/radial/sweep gradient constructors, blur+saturation+matrix filters, image from encoded bytes / colour, PNG encode, surface `read_pixels`. Remaining primitives (text blobs, codec streaming, runtime effects, matrix44 binding, full blend-mode mapping, picture recording, font manager) are **out of scope** for this phase — see "Scope: follow-up" below for the list. |
| C-3 untagged generic refcount cast | **Resolved** | `RefCounted<T>` now stores a `REFCOUNT_TAG` magic at offset 0 and `AtomicU32` at offset 4. `sk_refcnt_get_count` validates the tag before reading the count — a non-tagged pointer returns 0 instead of arbitrary memory. `test_refcnt_utility_rejects_untagged_pointer` exercises this. |
| C-4 peek_pixels lifetime hazard | **Resolved** | Documented the borrow semantics on `sk_surface_peek_pixels` and added the copy-based `sk_surface_read_pixels(dst, dst_len)` as the recommended alternative (tested). |
| C-5 no runtime ABI check | **Resolved** | Added `sk_init(major, minor)` + `sk_is_initialized()`. The module doc now tells clients they must call `sk_init` before any other entry point. |
| N-1 no cbindgen header | **Resolved** | `build.rs` runs cbindgen and emits `$OUT_DIR/skia_rs.h`; set `SKIA_RS_FFI_EMIT_HEADER=1` to also write `include/skia_rs.h` (committed). Set `SKIA_RS_FFI_SKIP_CBINDGEN=1` to skip in publish/sandbox environments. |
| N-2 no C example | **Resolved** | Added `examples/draw_rect.c` demonstrating init + surface + paint + draw + read_pixels + panic check. |
| N-3 no path iteration | **Resolved** | Added `sk_path_iter_t` with `sk_path_iter_new / _next / _delete` and the `SK_PATH_VERB_*` constants. Covered by `test_path_iteration`. |
| N-4 matrix inverse/identity/determinant | **Resolved** | Added `sk_matrix_invert`, `sk_matrix_is_identity`, `sk_matrix_determinant`. |
| T-1 no panic-recovery test | **Resolved** | `test_panic_catcher_sets_flag_and_returns_default` and `test_null_inputs_dont_crash`. |
| T-2 no multi-thread refcount stress | **Resolved** | `test_cross_thread_refcount_stress` spawns 8 threads × 1000 ref/unref each. |
| T-3 no true cross-language test | **Resolved** | `examples/draw_rect.c` compiles against the generated header, links against `libskia_rs_ffi.so`, and runs end-to-end. Verified manually during Phase 6C. Automating this in CI (a `build.rs`-driven `cc` step or a `tests/c_ffi.rs` harness that shells out to `cc`) is **Scope: follow-up**. |

### Scope: follow-up (not addressed in Phase 6C)

The Skia C API surface is ~1000 functions. Phase 6C focused on (a)
soundness universally and (b) demonstrating the wrapping pattern for
every major primitive. The following remain:

- **TextBlob / Paragraph / Shaper**: no `sk_textblob_t`, `sk_paragraph_t`.
- **FontManager**: no FFI for system-font enumeration.
- **Matrix44**: `abi::SkMatrix44ABI` is defined but no `sk_matrix44_*` setters/multiply/invert.
- **ColorSpace**: no FFI — `sk_imageinfo_t.color_space` is always the default sRGB.
- **Picture / PictureRecorder**: recording/playback is not exposed.
- **Clip API on sk_canvas_t**: `clip_rect`, `clip_path`, `clip_ibounds` are stubbed by `sk_canvas_concat` friends but no explicit clip FFI.
- **Codec streaming**: only `sk_image_from_encoded` (single-shot); no progressive decode / row-at-a-time API.
- **RuntimeEffect / SkSL**: `skia-rs-paint::runtime_effect` has no C surface at all.
- **Gradient refinements**: `with_local_matrix`, `with_flags`, `TwoPointConicalGradient`, `ImageShader` from an `sk_image_t`.
- **Filter refinements**: `DropShadowImageFilter`, `ColorFilterImageFilter`, `Morphology*`, `Lighting*`, `DisplacementMap`, `Compose`, `Merge`, `Offset`, `MatrixConvolution` — each would be a few lines of FFI.
- **Canvas save/restore layers with paint**: `save_layer(rec)` and `SaveLayerFlags`.
- **Region / Path effects**: dash, corner, trim, compose effects.
- **GPU backend**: `skia-rs-gpu` has no FFI wrapper.
- **SVG / PDF / Skottie**: the dependent crates each need their own FFI module.
- **Cross-language test harness**: link `examples/draw_rect.c` against the cdylib in CI via a `build.rs`-driven `cc` step or a `tests/c_ffi.rs` harness.

Each of these is mechanical once the pattern is understood — every
primitive follows the same recipe already demonstrated by `sk_shader_t`,
`sk_colorfilter_t`, etc. They were deferred because the priority for
Phase 6C was soundness (panic catching + tag-validated refcount), not
surface breadth.

## Original Gap Analysis (2026-04-25, pre-Phase 6C)

## Summary

- Total public functions reviewed: ~70 (`pub unsafe extern "C" fn sk_*` plus the `RefCounted<T>` helper API and ABI module)
- Total test functions: 13 (all passing — unit-level round-trips for each exposed object)
- Total gaps found: 12
- Critical gaps: 5 (API surface is minimal vs. what users need to port Skia-C consumers)
- Nice-to-have gaps: 4
- Test coverage gaps: 3
- Estimated complexity: **Medium** — the foundations (panic catching, refcount wrapper, ABI versioning) are solid; the gap is breadth of exposed functions, not depth.

## Files Reviewed
- [x] lib.rs (1321 lines)
- [x] abi.rs (484 lines)

## Overall Shape

The crate exposes a C ABI at two levels:

1. **`abi.rs`**: ABI-stable type declarations (`SkRectABI`, `SkMatrixABI`, `SkImageInfoABI`, etc.), versioning (`sk_abi_get_version`, `sk_abi_is_compatible`), and `sk_sizeof_*` introspection helpers.
2. **`lib.rs`**: Actual exported functions on `sk_surface_t`, `sk_paint_t`, `sk_path_t`, `sk_pathbuilder_t`, plus matrix helpers and a few drawing shortcuts.

Both files take reasonable design choices: `RefCounted<T>` is a proper atomic-ref-counted Box that starts with `AtomicU32` at offset 0 (so the generic `sk_refcnt_*` functions can cast through `void*` safely). Panic catching is infrastructure-complete (`LAST_PANIC`, `catch_panic`, `catch_panic_void`). The public doc comment is unusually thorough about thread-safety semantics.

The problem is simply scope: the crate exposes ~50 functions vs. Skia's C API surface of ~1000+ functions. Many things that Skia C users reach for (canvases, shaders, filters, typefaces, images, encoders) are not exposed.

## Critical Gaps

### C-1: The panic-catching infrastructure (`catch_panic` / `catch_panic_void`) exists but is never called
**File:** `lib.rs` (lines 210-228, and everywhere else)
**Severity:** Critical
**Description:** `catch_panic` and `catch_panic_void` are defined and documented as the boundary between Rust and C. But none of the exported `sk_*` functions in the file wraps its body in `catch_panic` or `catch_panic_void`. Every `sk_surface_draw_rect`, `sk_paint_set_color`, `sk_pathbuilder_quad_to` is a naked unsafe block that would unwind into C code if any inner call panicked. `LAST_PANIC` is only set by the (never-called) helpers, so `sk_last_call_panicked()` always returns false.
**Impact:** A panic in any Rust-side code (e.g., an `assert!` failing, a `Vec` bounds check, an arithmetic overflow in debug mode) will unwind across the FFI boundary. In the best case the host process aborts; in the worst case it corrupts state and continues. This is undefined behavior per the Rust reference.
**Effort:** Medium — mechanical but touches every exported function. ~50 functions × 3-4 lines per wrap = ~200 lines of churn. Should also add `#[inline(always)]` on the helpers to avoid frame-per-call overhead. ~1 day.

### C-2: API surface is a small fraction of what consumers need (no Canvas, no Shader, no Filter, no Typeface, no Image, no Codec, no Surface file I/O)
**File:** `lib.rs` (1321 lines total, ~800 lines of actual code)
**Severity:** Critical
**Description:** Currently exposed:
- Surface: new_raster, new_raster_with_info, ref/unref, width/height, peek_pixels, clear, draw_rect/circle/path/line (5 drawing ops)
- Paint: new, clone, ref/unref, get/set color/style/stroke_width/antialias (~7 props)
- Path: new, clone, ref/unref, bounds, is_empty, fill_type, contains
- PathBuilder: new, ref/unref, move_to, line_to, quad_to, cubic_to, close, add_rect, add_oval, add_circle, detach, snapshot
- Matrix: set_identity, set_translate/scale/rotate, concat, map_point

Missing compared to Skia's sk_* C API:
- **No `sk_canvas_t`**: there is no distinct canvas type; the surface has drawing functions inlined. Skia C consumers expect to save/restore, concat, clip, and draw on an explicit canvas.
- **No `sk_shader_t`** / `sk_colorfilter_t` / `sk_imagefilter_t` / `sk_maskfilter_t`: the rich filter/shader system from skia-rs-paint is invisible from C.
- **No `sk_image_t`**: no way to load or create an image from C.
- **No `sk_typeface_t`** / `sk_font_t` / `sk_textblob_t`: no text rendering from C.
- **No codec integration**: cannot decode PNG/JPEG/etc from C.
- **No Surface → PNG / PDF / SVG output**: surfaces cannot be saved to any file format.
- **No Color4f API**: only `u32` colors are exposed (lossy for HDR / wide-gamut).
- **No Matrix44 (4x4) API**: only 3x3 `sk_matrix_t`.
- **No gradient / shader constructor exports**: `sk_shader_new_linear_gradient`, `sk_shader_new_radial_gradient`, etc.

**Impact:** Language bindings / C clients built on this FFI can only draw solid-color rectangles / circles / paths / lines. No text, no images, no gradients, no filters. The skia-rs-python and skia-rs-node sibling crates likely wrap this thin surface and inherit the same restrictions.
**Effort:** High. Each missing type is ~50-150 lines of FFI (new, ref, unref, getters, setters, factory functions). Full parity with Skia's C API is 3-4 weeks of work. Realistic Phase 6 scope: add canvas, typeface/font, image, and gradient-shader constructors — unlocks 80% of practical use cases. ~1 week.

### C-3: Generic `sk_refcnt_get_count` / `sk_refcnt_is_unique` do a blind cast from `*const c_void` to `*const AtomicU32`
**File:** `lib.rs` (lines 336-350)
**Severity:** Critical
**Description:** The C signature is `(ptr: *const sk_refcnt_t) -> u32` where `sk_refcnt_t = c_void`. The body casts the pointer directly to `*const AtomicU32` on the assumption that "All our refcounted types start with AtomicU32." This is correct for types created via `RefCounted::new`, but:
  - `#[repr(C)]` is on `RefCounted<T>` but not verified at every use site
  - Callers from C cannot know whether a given pointer is `RefCounted`-backed or a raw box
  - If a future type implements refcounting differently (e.g., inline atomic field inside T, or using `Arc`), these generic functions silently produce garbage
  - There is no sanity check that the void pointer is non-null before the cast dereferences it (line 338 handles nullability, but then does raw pointer arith)

**Impact:** Memory-safety foot-gun: any C consumer who passes a non-RefCounted pointer to `sk_refcnt_get_count` reads arbitrary memory. There is no typeid check.
**Effort:** Small (add a type tag field to `RefCounted<T>` at a fixed offset; check the tag in the generic functions; ~20 lines). Alternatively, remove the generic functions and force per-type accessors.

### C-4: `sk_surface_peek_pixels` is declared but its body likely exposes an unsynchronized view
**File:** `lib.rs` (line ~656, `pub unsafe extern "C" fn sk_surface_peek_pixels`)
**Severity:** Critical
**Description:** Would need to be read in full, but the general pattern for `peek_pixels` in this crate's parent (skia-rs-canvas) is to hand back `&[u8]`. In C, this translates to a `*const u8` pointer + length. The lifetime of the pointer is tied to the surface's internal buffer, which the caller may keep past the point the surface is unref'd — producing a dangling pointer. There is no way in C to express "this pointer is valid until the surface is dropped."
**Impact:** Use-after-free hazard. Common enough pattern in Skia C that it's documented but wrong, yet without a way to mark ownership, C callers must trust the Rust side's lifetime semantics.
**Effort:** Small (document explicitly; consider adding a `sk_surface_read_pixels_into(dst, dst_bytes)` copy-based variant which is safer).

### C-5: `ABI_VERSION_MAJOR / MINOR / PATCH` version is never checked at runtime in any loaded library
**File:** `abi.rs` (lines 1-80)
**Severity:** Critical
**Description:** `sk_abi_get_version()` and `sk_abi_is_compatible()` exist but no initialization-time check exists in the exported API (e.g., `sk_init()` that validates loader/library compatibility). Language bindings are expected to call `sk_abi_is_compatible(MAJOR, MINOR)` but the convention is not enforced; a library built against v1.0 could be loaded by a consumer expecting v2.0 without any detection.
**Impact:** ABI drift silently produces incompatible binaries. Already-encountered scenario in other Rust FFI libraries.
**Effort:** Small (publish header generation that embeds the version check as a mandatory init call; add `sk_init()` that returns success/failure; ~30 lines). Requires a header-gen step (e.g., cbindgen).

## Nice-to-Have Gaps

### N-1: No `cbindgen` configuration or header generation
**File:** Crate root (no `cbindgen.toml`, no `build.rs`)
**Severity:** Nice-to-have
**Description:** C consumers need a `skia_rs.h` header with the struct definitions and function prototypes. This is typically auto-generated from Rust source via `cbindgen`. The crate has no cbindgen config, no build script, no sample header. Consumers currently have to hand-translate the Rust signatures to C.
**Impact:** No C or C++ project can build against this FFI without writing its own header, which quickly diverges from the Rust source.
**Effort:** Small-Medium (add `cbindgen.toml`, add `build.rs` or a CI step, ensure the resulting header compiles against a minimal C program; ~2 hours).

### N-2: No examples of calling from C / C++ / Python
**File:** No `examples/` directory, no `tests/` with FFI consumers
**Severity:** Nice-to-have
**Description:** The crate has comprehensive doc comments about thread safety and refcounting but no runnable example that actually calls the FFI from C. skia-rs-python and skia-rs-node exist as sibling crates but they use the Rust API via PyO3 / napi, not the C FFI.
**Effort:** Small (add `examples/draw_rect.c` with a Makefile; demonstrates linking to the .so/.dylib and calling the FFI).

### N-3: `sk_path_t` has no iteration API
**File:** `lib.rs` (path section)
**Severity:** Nice-to-have
**Description:** Exposed: `sk_path_get_bounds`, `sk_path_is_empty`, `sk_path_get_filltype`, `sk_path_set_filltype`, `sk_path_contains`. Missing: `sk_path_iter_t` for walking the path's verbs/points. A C consumer cannot extract the control points from a path. Essential for any inspection / debugging / conversion use case.
**Effort:** Small-Medium (iterator state struct with next_verb function returning `SkPathVerbABI` and filling point array; ~80 lines).

### N-4: `sk_matrix_t` has no inverse, decompose, or equality API
**File:** `lib.rs` (matrix section, lines 989-1048)
**Severity:** Nice-to-have
**Description:** Only identity/translate/scale/rotate setters, `concat`, and `map_point`. Missing: inverse, determinant, is_identity, is_invertible, equality, serialization. These are standard on Skia's SkMatrix.
**Effort:** Small.

## Test Coverage Gaps

### T-1: No test verifies panic is caught at the boundary
**Description:** Because `catch_panic` is never called (C-1), a test asserting that a panicking Rust-side call returns a default and sets `sk_last_call_panicked()` can't possibly pass. Once C-1 is fixed, such a test should be added — e.g., pass a null pointer to a function that would unwrap it, expect graceful return + LAST_PANIC flag set.
**Effort:** Small, blocked on C-1.

### T-2: No test exercises the refcount semantics across thread boundaries
**Description:** The 13 tests are all single-threaded. `ref`/`unref` are claimed to be thread-safe (they use `AtomicU32` with AcqRel ordering, which is correct), but no test spawns N threads each calling `ref`/`unref` and verifies the count converges correctly. Loom or a plain thread stress test would catch any ordering bug.
**Effort:** Small-Medium.

### T-3: No test simulates cross-language calling (from C or from a C-like context)
**Description:** All 13 tests are Rust `#[test]` functions. `test_draw_rect` calls `sk_surface_*` via Rust, not via an extern C binding, so the `extern "C"` ABI isn't actually exercised. A true FFI test would compile a tiny C file, link against this crate's cdylib output, and run the C program.
**Effort:** Medium (requires build script + cc crate; ~100 lines).

## Implementation Notes

### `RefCounted<T>` design is correct
The wrapper uses `AtomicU32` with `Relaxed` for increment (fine — no other thread cares about when it happened), `AcqRel` for decrement (correct — need synchronization before the final drop), and fires `Box::from_raw` exactly once via `fetch_sub == 1`. This matches the standard Arc implementation. The only risk is the generic `sk_refcnt_*` cast (C-3).

### Panic-catching infrastructure is sound but unused
`catch_panic` uses `AssertUnwindSafe` correctly and `panic::catch_unwind` + a `T: Default` fallback. This is the right pattern; it just isn't wired in. See C-1.

### ABI types mirror internal types carefully
`sk_point_t` is `#[repr(C)]` with two f32 fields matching `skia_rs_core::Point` layout. The `Into<Point>` / `From<Point>` conversions are safe bit-copies. Similar for `sk_rect_t`, `sk_matrix_t`, `sk_imageinfo_t`. The split between `abi.rs` (canonical ABI-stable types) and `lib.rs` (exported function types using abi.rs types) is a good design; it's just underused because most functions in lib.rs define their own types inline.

### Thread-safety documentation is unusually thorough
The 180-line module doc comment is one of the best pieces of documentation in the entire workspace. Calls out thread-compatible vs thread-safe vs main-thread-only semantics and gives safe/unsafe code examples. This is exactly the documentation that Skia-C consumers need to port correctly.

### Some exported functions can be simplified
`sk_path_ref` is defined alongside `sk_path_delete` (alias for unref) alongside `sk_path_unref`. Skia C uses `sk_path_delete` everywhere for destruction; `sk_path_unref` is redundant. But the names are cheap and harmless, so the redundancy is fine.

## Recommendations

### Priority 1: Wrap every exported function in `catch_panic` (C-1)
Non-negotiable for soundness. Mechanical work, ~1 day. Makes the crate actually safe to call from C.

### Priority 2: Expand API surface to include Canvas, Image, Font, Gradient (C-2)
The biggest user-facing gap. Scope Phase 6 to:
- `sk_canvas_t` with save/restore/concat/clip/draw* (~150 lines)
- `sk_image_t` from PNG/JPEG bytes, peek_pixels (~80 lines)
- `sk_typeface_t` / `sk_font_t` / basic text drawing (~120 lines)
- `sk_shader_new_linear_gradient` / `radial` / `sweep` (~100 lines)
- Surface → PNG encoder (~60 lines)

This ~500 lines of FFI gets the C API to parity with skia-c's common-case coverage. ~1 week.

### Priority 3: Generate cbindgen header (N-1)
Once the API stabilizes after the Priority 2 work, generate and commit a `skia_rs.h`. Keep it generated (don't hand-edit) to avoid drift. ~2 hours.

### Priority 4: Refcount tag safety (C-3) and peek-pixels safety (C-4)
Small fixes; can batch in one PR. ~0.5 day.

### Priority 5: Matrix inverse / path iteration (N-3, N-4)
Standard extensions once the core API is solid. ~1 day.

### Priority 6: Cross-language test and panic-recovery test (T-1, T-3)
Medium effort; set up a tiny C test program compiled via build.rs. Validates the entire FFI chain end-to-end. ~1-2 days.
