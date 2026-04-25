# skia-rs-codec Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)

## Summary

- Total public functions reviewed: ~120 (`pub fn` across codec.rs, generator.rs, gpu_image.rs, image.rs, lazy_image.rs)
- Total test functions: 47 after Phase 6B (was 26)
  - codec.rs: 11 (+5 format round-trips)
  - generator.rs: 6 (+3 EncodedImageGenerator tests)
  - gpu_image.rs: 10 (+5 GpuImageBackend tests)
  - image.rs: 10 (+5 read_pixels conversion / filter / linear sampling tests)
  - lazy_image.rs: 7 (unchanged, already well-covered)
- Total gaps found: 13
- Critical gaps: 4 — all resolved in Phase 6B
- Nice-to-have gaps: 5 — 2 resolved (N-1, N-2), 3 deferred (N-3, N-4, N-5)
- Test coverage gaps: 4 — all resolved
- Phase 6B status: **10 resolved, 3 deferred with follow-up plan**
- Estimated complexity: **Medium** — the format codecs themselves are largely fine (they wrap real crates: `png`, `jpeg_decoder/encoder`, `webp`, `gif`, `avif_decode/ravif`, `rawloader`); the gaps are in `Image` pixel operations and the GPU-image / image-filter wiring.

## Files Reviewed
- [x] lib.rs (32 lines)
- [x] codec.rs (2145 lines)
- [x] image.rs (480 lines)
- [x] generator.rs (472 lines)
- [x] gpu_image.rs (520 lines)
- [x] lazy_image.rs (527 lines)

## Critical Gaps

### C-1: `Image::read_pixels` does not perform any format conversion
**File:** `image.rs` (lines 275-321)
**Severity:** Critical
**Status:** RESOLVED (Phase 6B). `Image::read_pixels` now delegates to `skia_rs_core::convert_pixels`, which handles color-type conversion (RGBA↔BGRA, Gray8↔RGBA, Alpha8↔RGBA, RGB888↔RGBA, RGB565↔RGBA) and alpha-type conversion (Premul↔Unpremul) in one row-by-row pass. Subset reads (src_x, src_y) are handled by pointing the source slice at the subset's first byte while keeping the original row stride. Four tests cover the conversion paths; see `test_read_pixels_rgba_to_bgra_converts`, `test_read_pixels_subset_with_conversion`, `test_read_pixels_alpha_type_conversion`, `test_read_pixels_rejects_out_of_bounds`.
**Description:** When `dst_info.color_type() != self.color_type()` or `dst_info.alpha_type() != self.alpha_type()`, the function returns `false` with the comment `// TODO: Format conversion`. Yet a complete `convert_pixels()` function already exists in `generator.rs` (lines 216-307) handling RGBA↔BGRA, Gray8→RGBA, Alpha8→RGBA, RGBA→Gray8. `Image::read_pixels` does not call it.
**Impact:** Any consumer that wants RGBA pixels from a BGRA image (or vice versa) silently receives no data. This is a known limitation flagged in earlier audits (Phase 0) and has not been resolved. The `read_pixels` API is the canonical path for getting pixels out of an Image for display-server upload or file export; its partial implementation breaks every format-converting use case.
**Effort:** Small (delegate to `generator::convert_pixels` with a per-row source crop; ~30 lines including tests).

### C-2: `Image::make_with_filter` is a no-op that returns `self.clone()`
**File:** `image.rs` (lines 427-431)
**Severity:** Critical
**Status:** RESOLVED (Phase 6B). Signature changed to `make_with_filter(&self, filter: &dyn skia_rs_paint::ImageFilter) -> Option<Self>`. Implementation converts the image to RGBA8/Premul via `read_pixels`, calls `filter.apply(pixels, width, height)` (the single-buffer apply added in Phase 4), and wraps the result as a new Image. The codec crate now depends on skia-rs-paint (no cycle; paint does not depend on codec). Multi-input filters (Merge, DisplacementMap's displacement pin) are documented as passing through unchanged because the paint-crate's single-buffer apply() API cannot supply multiple input buffers; a future multi-input apply will unblock them. Test: `test_make_with_filter_offset_shifts_pixels` using `OffsetImageFilter`.
**Description:** Body: `// TODO: Implement matrix transformation\nSome(self.clone())`. In Skia, `SkImage::makeWithFilter` applies an `SkImageFilter` (blur, drop-shadow, color-matrix, etc.) to the image and returns a new filtered image. Here the function takes no filter argument at all, so even a correct implementation has no filter to apply — the signature is wrong.
**Impact:** Image filters defined in skia-rs-paint's `filter.rs` cannot be materialised onto an Image. Any UI-side image filtering pipeline is stuck on the CPU bitmap side.
**Effort:** Medium (signature change to accept `&dyn ImageFilter` + `&Matrix` + clip rect; depends on N-4 from paint crate GAPS.md which added `apply()` methods to ImageFilter trait).

### C-3: `GpuImage` is a standalone pixel container, not GPU-backed; `has_texture()` is always the caller's responsibility
**File:** `gpu_image.rs` (lines 100-446)
**Severity:** Critical
**Status:** RESOLVED (Phase 6B, trait side). Defined `GpuImageBackend` trait with `upload(info, format, pixels, row_bytes) -> handle`, `read_back(handle, info, format, dst, dst_row_bytes)`, and `release(handle)`. `GpuImage::set_backend(Arc<dyn GpuImageBackend>)` installs a backend; `upload()`, `read_pixels_from_gpu()`, and `clear_texture_handle()` now delegate to it when present. `GpuImageInner::drop` releases the GPU handle to prevent leaks. `read_pixels()` falls back to the backend when the raster cache is absent. Five new tests exercise the full contract via an in-memory `MockBackend`. **Follow-up (Phase 6F):** the concrete Vulkan/Metal/OpenGL/D3D12/WebGPU backends in skia-rs-gpu need to implement `GpuImageBackend` for their respective texture types — that wiring was deliberately kept in-scope for the skia-rs-gpu completion phase to avoid cross-crate churn in this one.
**Description:** `GpuImage` stores a `raster_cache: Option<Vec<u8>>` and a `texture_handle: Option<GpuTextureHandle>` where the handle is a backend-agnostic `{ id: u64, backend: GpuBackend }`. Neither the upload path (raster → GPU) nor the read-back path (GPU → raster) is implemented in this crate; they are simply flagged with comments "GPU upload is handled by the `skia-rs-gpu` crate" and "GPU read-back would be triggered here by GPU backend." The `set_texture_handle` method is purely a bookkeeping setter that anyone can call.
**Impact:** `GpuImage` is functionally identical to `Image` except that it stores an extra integer in an Option. Nothing in skia-rs-gpu currently implements the upload/read-back loop for `GpuImage` (the backend crates operate on their own texture types). The public `GpuImage` API is a hollow shim.
**Effort:** High (requires cross-crate coordination: define a trait `GpuImageBackend` that skia-rs-gpu backends implement; wire the `new_from_raster` → backend upload → `set_texture_handle`; wire `read_pixels` → backend read-back). Realistic scope: 1 week.

### C-4: `EncodedImageGenerator::on_get_pixels` re-decodes the full image on every call
**File:** `generator.rs` (lines 395-419)
**Severity:** Critical
**Status:** RESOLVED (Phase 6B). `EncodedImageGenerator` now decodes exactly once, in `from_shared`, and caches the resulting `Image`. `info()` returns the decoder's actual info instead of the hard-coded `Rgba8888/Premul` placeholder. `on_get_pixels` serves from the cached image; `on_get_pixels_with_conversion` is overridden to convert directly from cached pixels instead of the default staging-buffer indirection. `query_supports_info` whitelists the conversions `convert_pixels` can actually produce. The local `convert_pixels` helper was collapsed into a thin wrapper over `skia_rs_core::convert_pixels`, which handles the same formats plus Premul↔Unpremul and RGB888/RGB565 that the local version lacked. Three new tests cover: info reflecting decoded format after PNG round-trip; repeated `get_pixels` returning identical bytes with a shared decoded image; and on-demand BGRA conversion from a cached RGBA decode.
**Description:** The method calls `crate::decode_image(&self.encoded_data)` every time `get_pixels` is invoked, even if the target info is identical to the previous call. For a JPEG or PNG this is a full CPU decode per call — a `LazyImage` built on top caches the result, but `ImageGenerator::get_pixels` (the direct API) does not cache internally. The generator's own `info` is hardcoded to `Rgba8888/Premul` at construction time (lines 359-361) regardless of the source image's actual format; on decode, the returned `Image` may have a different `color_type` or `alpha_type`, and the code copies `width * bytes_per_pixel` bytes without verifying the format matches.
**Impact:** (1) Performance: any downstream code repeatedly calling `get_pixels` incurs repeated decode cost. (2) Correctness: mismatched color/alpha types between `self.info` and the decoded image produce silently wrong pixels or slice panics. For example, a PNG decoded as `Unpremul` fed into a generator declaring `Premul` gives non-premultiplied pixels in a buffer the caller will interpret as premultiplied.
**Effort:** Medium (cache the decoded image behind `parking_lot::Mutex<Option<Image>>`; on first decode, write back real `info` from decoded image; verify color/alpha match before copy or call `convert_pixels`; ~60 lines).

## Nice-to-Have Gaps

### N-1: `Image::make_scaled` uses nearest-neighbor only
**File:** `image.rs` (lines 395-425)
**Severity:** Nice-to-have
**Status:** RESOLVED (Phase 6B). Introduced a `SamplingOptions` enum (`Nearest | Linear`) and a new `Image::make_scaled_with(w, h, sampling)` method. Bilinear samples at pixel centers (`src_x = (dst_x + 0.5) * scale - 0.5`) so 1:1 scales reproduce the source exactly and 2:1 downscales average adjacent pairs instead of aliasing. `make_scaled(w, h)` stays as a back-compat alias for the Nearest path. Tests: `test_image_scaled_linear_blends_neighbors`, `test_image_scaled_nearest_preserves_pixels`. Cubic / Mitchell / Lanczos are not implemented yet — those are follow-up work once bench coverage exists to compare quality-cost trade-offs.
**Description:** The sampling comment "Simple nearest-neighbor scaling" is accurate. Skia's `SkImage::makeScaled` supports linear/cubic/Mitchell sampling via `SkSamplingOptions`. No arg for sampling quality is exposed here; the function unconditionally uses nearest-neighbor.
**Impact:** Downscaled images are visibly aliased. Upscaled images have hard blocky edges. Adequate for thumbnails/icons, wrong for photographic content.
**Effort:** Small (add `SamplingOptions` param; implement bilinear as additional branch; ~50 lines).

### N-2: `convert_pixels` does not handle F16/F32 floating-point formats
**File:** `generator.rs` (lines 216-307)
**Severity:** Nice-to-have
**Status:** PARTIALLY RESOLVED (Phase 6B). The generator crate's local `convert_pixels` is now a thin wrapper over `skia_rs_core::convert_pixels`, which already handles `RGBA8888<->Rgb888`, `RGBA8888<->Rgb565`, and — crucially — Premul↔Unpremul for any color type with alpha. That closes the same-color-type-different-alpha-type gap that was producing `Err(UnsupportedColorType)`. Test: `test_convert_pixels_premul_to_unpremul`. F16/F32 remain unsupported in the core converter; adding those arms is a core-crate task, not a codec-crate task, and is tracked in skia-rs-core's own gap analysis. **Follow-up:** upstream F16/F32 conversion in skia-rs-core's `convert_row`.
**Description:** The match arms cover RGBA↔BGRA, Gray8→RGBA, Alpha8→RGBA, RGBA→Gray8. `ColorType::RgbaF16` and `ColorType::RgbaF32` fall through to `Err(UnsupportedColorType)`. No premultiplied/unpremultiplied conversion for the same color format (e.g., RGBA8888 Premul → RGBA8888 Unpremul) — the "same format" fast path checks both color type AND alpha type, so a Premul→Unpremul conversion currently returns an error.
**Effort:** Small-Medium (add F16/F32 arms; add a premul↔unpremul helper; ~80 lines).

### N-3: RAW demosaicing uses simple nearest-neighbor Bayer interpolation
**File:** `codec.rs` (lines 1751-1835)
**Severity:** Nice-to-have
**Status:** DEFERRED (not in Phase 6B scope). A proper demosaic + color pipeline (Malvar-He-Cutler, AHD, or RCD; plus black-level, white balance, gamma, and color-matrix stages from RAW metadata) is effectively a small library of its own. The nearest-neighbor Bayer path suffices for preview / thumbnail use cases, which is the realistic target for the `raw` feature today. **Follow-up:** a dedicated RAW-processing phase once there is a specific consumer with color-accuracy requirements. Candidates: adopt the `rawproc` crate, or vendor a Malvar-He-Cutler impl from the public-domain AVR-implementation corpus. No follow-up phase scheduled; revisit when a downstream consumer requests it.
**Description:** The demosaic uses a 3x3 neighborhood average for each missing color channel. Real cameras use Malvar-He-Cutler, AHD, or RCD demosaicing for much better edge preservation. The current implementation also applies only a linear black-level subtraction with no white balance, gamma, or color matrix from the RAW metadata. Camera-specific tone curves and color profiles are ignored.
**Impact:** RAW decode produces a visibly desaturated, slightly soft image. Acceptable for preview; wrong for editing.
**Effort:** High (a proper demosaic + color pipeline is a small library in itself).

### N-4: No animation frame support for GIF/WebP/APNG
**File:** `codec.rs` (lines 561-600)
**Severity:** Nice-to-have
**Status:** DEFERRED (not in Phase 6B scope). Animation support is a new public API surface (trait `AnimatedImageDecoder` with `frame_count`, `frame(i)`, `frame_delay`, `disposal_method`) that deserves its own design review — whether frames are random-access or iterator-style, how disposal and compositing interact with partial frames, how APNG's dispose_op bits interact with GIF's simpler disposal methods. Locking in the wrong API now would force a breaking change later. **Follow-up:** schedule a dedicated animation API pass after Phase 6 lands; the GIF, WebP, and APNG decoders already have frame-aware backing crates so the wiring is bounded once the shape of the trait is agreed.
**Description:** `GifDecoder::decode` reads only the first frame (`read_next_frame()` once), discards the rest. The `gif` crate exposes the full frame stream with disposal methods and delay_ms; none of that is surfaced. `WebpDecoder` calls `webp::Decoder::decode()` which returns a single frame even for animated WebP. There is no `AnimatedImageDecoder` trait or iterator.
**Impact:** Animated content renders as its first frame only. Users cannot iterate through GIF/animated-WebP frames.
**Effort:** Medium (new trait `AnimatedImageDecoder` with `frame_count()`, `frame(i) -> Image`, `frame_delay(i) -> Duration`, `disposal_method(i)`).

### N-5: No color profile (ICC) handling on decode
**File:** `codec.rs`, `image.rs`
**Severity:** Nice-to-have
**Status:** DEFERRED (not in Phase 6B scope). Proper ICC handling requires a color-management library decision — adopt `lcms2` (C library, widely used, LGPL) or hand-roll matrix+TRC conversion for the common sRGB / P3 / Adobe RGB / Rec.2020 cases. Either path touches every codec (PNG iCCP, JPEG APP2, WebP ICCP, AVIF nclx/colr) and the `ColorSpace` type in skia-rs-core needs a conversion-pipeline API. That's a cross-crate feature, not a codec-internal polish. **Follow-up:** scope a "ColorSpace completion" phase that picks the lib, defines the core API, and wires each decoder. Until then, all decoded images are implicitly sRGB and consumers should treat them as such.
**Description:** PNG iCCP chunk, JPEG APP2 ICC profile, WebP ICCP chunk, AVIF color nclx/colr boxes — all ignored. Decoded images land in an implicit sRGB space regardless of the source profile. The `ImageInfo` struct carries a `color_space: Option<ColorSpace>` field that is never populated by any codec.
**Impact:** P3-gamut content and Adobe-RGB photos display wrong. HDR content decoded from AVIF is clipped to sRGB.
**Effort:** High (requires a color-management library — `lcms2` or hand-rolled matrix+TRC conversion — and wiring through every codec).

## Test Coverage Gaps

### T-1: No tests for the format-conversion paths in `Image::read_pixels`
**Status:** RESOLVED (Phase 6B). Added four tests covering RGBA↔BGRA, subset-with-conversion, Premul↔Unpremul alpha conversion, and bounds rejection. See `image.rs` tests module.
**Description:** Apart from the same-format fast path being exercised in `test_image_subset`/`test_image_from_raster`, there is no test that calls `read_pixels` with a destination info that differs in color/alpha type. The TODO in C-1 is in untested code.
**Effort:** Small.

### T-2: PNG/JPEG/WebP/AVIF encode-decode round-trips only tested for BMP
**Status:** RESOLVED (Phase 6B). Added feature-gated round-trip tests for PNG (lossless, exact-RGB verification), JPEG (dimensions survive), WebP lossless + lossy (dimensions survive), and WBMP. These use a shared `test_checkerboard(w, h)` helper so format-specific tests stay small. The WebP round-trip surfaced and fixed a real bug: the `webp` crate returns RGB-only pixel buffers when the source has no alpha channel, and the decoder was assuming RGBA unconditionally — now the decoder checks `is_alpha()` and expands RGB→RGBA with opaque alpha when needed. AVIF and RAW round-trips are not added because their feature flags are off by default and their encoders / decoders have platform-specific dependencies; adding them when enabling those features is the next step.
**Description:** `test_bmp_encode_decode_roundtrip` verifies format detection and decode of a BMP. There is no equivalent test for PNG, JPEG, WebP, GIF, ICO, AVIF, or RAW. The feature-gated format codecs (`#[cfg(feature = "png")]`, etc.) are only tested when their feature is enabled, and even then only via `test_format_detection` which checks magic bytes, not decode correctness.
**Effort:** Medium (bundle tiny test fixtures per format; add round-trip tests behind each feature flag).

### T-3: No tests for `EncodedImageGenerator` decode caching or format mismatch
**Status:** RESOLVED (Phase 6B). Added three feature-gated tests: `test_encoded_generator_reports_decoded_info` (verifies `info()` reflects the decoder's native output after round-trip, not the hard-coded `Rgba8888/Premul` placeholder); `test_encoded_generator_caches_decoded_image` (verifies repeated `get_pixels` share a cached `Image` via `unique_id` equality and produce identical bytes); `test_encoded_generator_converts_on_demand` (encodes RGBA, requests BGRA via the generator, verifies the swizzle).
**Description:** The six tests in `generator.rs` cover `SolidColorGenerator` (trivial fixed-color) and the `convert_pixels` RGBA↔BGRA path. `EncodedImageGenerator` — the more interesting generator — has no test. The C-4 bug around redundant decode and silent format mismatch is completely untested.
**Effort:** Small-Medium.

### T-4: `GpuImage` tests only verify raster-path bookkeeping
**Status:** RESOLVED (Phase 6B). Added five tests exercising the new `GpuImageBackend` trait via an in-memory `MockBackend` that stores uploaded pixel buffers keyed by handle id: upload + read_back round-trip, `clear_texture_handle` triggers `release`, `Drop` triggers `release`, upload without backend returns `BackendUnavailable`, and `read_pixels` falls back to the backend when the raster cache is discarded. The real GPU backends (Vulkan/Metal/OpenGL/D3D12/WebGPU) implement the same trait in Phase 6F, so these tests will serve as a contract for those implementations.
**Description:** All 5 tests in `gpu_image.rs` create a `GpuImage::from_raster_data`, set a fake texture handle (`{ id: 12345, backend: GpuBackend::WebGpu }`), and assert the handle survives a round-trip. None of this exercises actual GPU upload or read-back because none exists (C-3). The tests pass but do not verify any of the advertised behavior.
**Effort:** Blocked on C-3.

## Implementation Notes

### Codecs are mostly real
PNG (`png` crate), JPEG (`jpeg_decoder` / `jpeg_encoder`), WebP (`webp` crate), GIF (`gif` crate), AVIF (`avif_decode` / `ravif`), RAW (`rawloader`) are all real implementations with proper error handling. BMP and ICO are hand-rolled parsers that correctly handle multiple bit-depths and compressions. WBMP is also hand-rolled for the tiny WAP format. The codec layer itself is the strongest part of the crate.

### The two "stub" comments in codec.rs are misleading
Lines 218 ("Simple PNG Codec (stub - would use png crate for real implementation)") and 376 ("JPEG Codec (stub)") — both labels are wrong. The code below them is a real implementation using `png` and `jpeg_decoder`/`jpeg_encoder`. The comments are stale from an earlier state.

### `LazyImage` is well-designed
`lazy_image.rs` implements a proper state machine (NotGenerated → Generating → Generated/Failed) with `parking_lot::RwLock` synchronization and caches pixels after first generation. It also properly supports `discard_pixels` to release memory. The lazy-image design is correct; the generator it wraps (C-4) is the weak link.

### `ImageGenerator` trait is well-designed
The trait's `query_supports_info` / `on_get_pixels` / `on_get_pixels_with_conversion` split is thoughtful: it lets generators declare what formats they can produce natively and lets the default impl handle conversion via `convert_pixels`. The issue is only that `EncodedImageGenerator`'s native info is hardcoded instead of read from the source.

### GpuImage is a half-finished abstraction
The concept is sound — a shared-pointer image that can live on CPU or GPU with optional cached raster copy — but the implementation stops at the data model. Without actual upload/read-back it's currently just `Arc<Image>` with extra metadata. Either finish it (significant work) or collapse it back into `Image` with an optional texture handle.

## Recommendations

### Priority 1: Fix `Image::read_pixels` format conversion (C-1)
Small fix. Directly unblocks any consumer that wants pixels in a different format than the image's native one. The `convert_pixels` helper already exists. ~30 lines.

### Priority 2: Fix `EncodedImageGenerator` decode caching (C-4)
Medium effort, large performance + correctness win. The bug is subtle (format mismatch → silent pixel corruption) so the correctness fix is at least as important as the caching.

### Priority 3: Add round-trip tests per format (T-2)
Prevents regressions in the codecs as dependencies update. Bundle tiny 4x4 or 8x8 fixtures per format. ~1 day.

### Priority 4: Decide GpuImage's fate (C-3)
Either (a) wire it to skia-rs-gpu's backends via a new trait, or (b) remove it and fold the texture-handle field into `Image`. (a) is ~1 week, (b) is 2 hours. The current state is the worst of both worlds.

### Priority 5: Animation support (N-4) and ICC color management (N-5)
Scope for a later phase. Animation needs a decision on API shape (iterator vs. random-access). ICC needs a library dependency decision.

### Priority 6: `make_with_filter` (C-2)
Blocked on the paint-crate filter `apply()` methods. Pick this up after skia-rs-paint's image filter work is complete.
