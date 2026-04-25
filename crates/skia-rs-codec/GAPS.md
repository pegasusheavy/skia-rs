# skia-rs-codec Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)

## Summary

- Total public functions reviewed: ~120 (`pub fn` across codec.rs, generator.rs, gpu_image.rs, image.rs, lazy_image.rs)
- Total test functions: 26 (all passing)
  - codec.rs: 6
  - generator.rs: 3
  - gpu_image.rs: 5
  - image.rs: 5
  - lazy_image.rs: 7
- Total gaps found: 13
- Critical gaps: 4 (functional correctness blockers)
- Nice-to-have gaps: 5
- Test coverage gaps: 4
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
**Description:** When `dst_info.color_type() != self.color_type()` or `dst_info.alpha_type() != self.alpha_type()`, the function returns `false` with the comment `// TODO: Format conversion`. Yet a complete `convert_pixels()` function already exists in `generator.rs` (lines 216-307) handling RGBA↔BGRA, Gray8→RGBA, Alpha8→RGBA, RGBA→Gray8. `Image::read_pixels` does not call it.
**Impact:** Any consumer that wants RGBA pixels from a BGRA image (or vice versa) silently receives no data. This is a known limitation flagged in earlier audits (Phase 0) and has not been resolved. The `read_pixels` API is the canonical path for getting pixels out of an Image for display-server upload or file export; its partial implementation breaks every format-converting use case.
**Effort:** Small (delegate to `generator::convert_pixels` with a per-row source crop; ~30 lines including tests).

### C-2: `Image::make_with_filter` is a no-op that returns `self.clone()`
**File:** `image.rs` (lines 427-431)
**Severity:** Critical
**Description:** Body: `// TODO: Implement matrix transformation\nSome(self.clone())`. In Skia, `SkImage::makeWithFilter` applies an `SkImageFilter` (blur, drop-shadow, color-matrix, etc.) to the image and returns a new filtered image. Here the function takes no filter argument at all, so even a correct implementation has no filter to apply — the signature is wrong.
**Impact:** Image filters defined in skia-rs-paint's `filter.rs` cannot be materialised onto an Image. Any UI-side image filtering pipeline is stuck on the CPU bitmap side.
**Effort:** Medium (signature change to accept `&dyn ImageFilter` + `&Matrix` + clip rect; depends on N-4 from paint crate GAPS.md which added `apply()` methods to ImageFilter trait).

### C-3: `GpuImage` is a standalone pixel container, not GPU-backed; `has_texture()` is always the caller's responsibility
**File:** `gpu_image.rs` (lines 100-446)
**Severity:** Critical
**Description:** `GpuImage` stores a `raster_cache: Option<Vec<u8>>` and a `texture_handle: Option<GpuTextureHandle>` where the handle is a backend-agnostic `{ id: u64, backend: GpuBackend }`. Neither the upload path (raster → GPU) nor the read-back path (GPU → raster) is implemented in this crate; they are simply flagged with comments "GPU upload is handled by the `skia-rs-gpu` crate" and "GPU read-back would be triggered here by GPU backend." The `set_texture_handle` method is purely a bookkeeping setter that anyone can call.
**Impact:** `GpuImage` is functionally identical to `Image` except that it stores an extra integer in an Option. Nothing in skia-rs-gpu currently implements the upload/read-back loop for `GpuImage` (the backend crates operate on their own texture types). The public `GpuImage` API is a hollow shim.
**Effort:** High (requires cross-crate coordination: define a trait `GpuImageBackend` that skia-rs-gpu backends implement; wire the `new_from_raster` → backend upload → `set_texture_handle`; wire `read_pixels` → backend read-back). Realistic scope: 1 week.

### C-4: `EncodedImageGenerator::on_get_pixels` re-decodes the full image on every call
**File:** `generator.rs` (lines 395-419)
**Severity:** Critical
**Description:** The method calls `crate::decode_image(&self.encoded_data)` every time `get_pixels` is invoked, even if the target info is identical to the previous call. For a JPEG or PNG this is a full CPU decode per call — a `LazyImage` built on top caches the result, but `ImageGenerator::get_pixels` (the direct API) does not cache internally. The generator's own `info` is hardcoded to `Rgba8888/Premul` at construction time (lines 359-361) regardless of the source image's actual format; on decode, the returned `Image` may have a different `color_type` or `alpha_type`, and the code copies `width * bytes_per_pixel` bytes without verifying the format matches.
**Impact:** (1) Performance: any downstream code repeatedly calling `get_pixels` incurs repeated decode cost. (2) Correctness: mismatched color/alpha types between `self.info` and the decoded image produce silently wrong pixels or slice panics. For example, a PNG decoded as `Unpremul` fed into a generator declaring `Premul` gives non-premultiplied pixels in a buffer the caller will interpret as premultiplied.
**Effort:** Medium (cache the decoded image behind `parking_lot::Mutex<Option<Image>>`; on first decode, write back real `info` from decoded image; verify color/alpha match before copy or call `convert_pixels`; ~60 lines).

## Nice-to-Have Gaps

### N-1: `Image::make_scaled` uses nearest-neighbor only
**File:** `image.rs` (lines 395-425)
**Severity:** Nice-to-have
**Description:** The sampling comment "Simple nearest-neighbor scaling" is accurate. Skia's `SkImage::makeScaled` supports linear/cubic/Mitchell sampling via `SkSamplingOptions`. No arg for sampling quality is exposed here; the function unconditionally uses nearest-neighbor.
**Impact:** Downscaled images are visibly aliased. Upscaled images have hard blocky edges. Adequate for thumbnails/icons, wrong for photographic content.
**Effort:** Small (add `SamplingOptions` param; implement bilinear as additional branch; ~50 lines).

### N-2: `convert_pixels` does not handle F16/F32 floating-point formats
**File:** `generator.rs` (lines 216-307)
**Severity:** Nice-to-have
**Description:** The match arms cover RGBA↔BGRA, Gray8→RGBA, Alpha8→RGBA, RGBA→Gray8. `ColorType::RgbaF16` and `ColorType::RgbaF32` fall through to `Err(UnsupportedColorType)`. No premultiplied/unpremultiplied conversion for the same color format (e.g., RGBA8888 Premul → RGBA8888 Unpremul) — the "same format" fast path checks both color type AND alpha type, so a Premul→Unpremul conversion currently returns an error.
**Effort:** Small-Medium (add F16/F32 arms; add a premul↔unpremul helper; ~80 lines).

### N-3: RAW demosaicing uses simple nearest-neighbor Bayer interpolation
**File:** `codec.rs` (lines 1751-1835)
**Severity:** Nice-to-have
**Description:** The demosaic uses a 3x3 neighborhood average for each missing color channel. Real cameras use Malvar-He-Cutler, AHD, or RCD demosaicing for much better edge preservation. The current implementation also applies only a linear black-level subtraction with no white balance, gamma, or color matrix from the RAW metadata. Camera-specific tone curves and color profiles are ignored.
**Impact:** RAW decode produces a visibly desaturated, slightly soft image. Acceptable for preview; wrong for editing.
**Effort:** High (a proper demosaic + color pipeline is a small library in itself).

### N-4: No animation frame support for GIF/WebP/APNG
**File:** `codec.rs` (lines 561-600)
**Severity:** Nice-to-have
**Description:** `GifDecoder::decode` reads only the first frame (`read_next_frame()` once), discards the rest. The `gif` crate exposes the full frame stream with disposal methods and delay_ms; none of that is surfaced. `WebpDecoder` calls `webp::Decoder::decode()` which returns a single frame even for animated WebP. There is no `AnimatedImageDecoder` trait or iterator.
**Impact:** Animated content renders as its first frame only. Users cannot iterate through GIF/animated-WebP frames.
**Effort:** Medium (new trait `AnimatedImageDecoder` with `frame_count()`, `frame(i) -> Image`, `frame_delay(i) -> Duration`, `disposal_method(i)`).

### N-5: No color profile (ICC) handling on decode
**File:** `codec.rs`, `image.rs`
**Severity:** Nice-to-have
**Description:** PNG iCCP chunk, JPEG APP2 ICC profile, WebP ICCP chunk, AVIF color nclx/colr boxes — all ignored. Decoded images land in an implicit sRGB space regardless of the source profile. The `ImageInfo` struct carries a `color_space: Option<ColorSpace>` field that is never populated by any codec.
**Impact:** P3-gamut content and Adobe-RGB photos display wrong. HDR content decoded from AVIF is clipped to sRGB.
**Effort:** High (requires a color-management library — `lcms2` or hand-rolled matrix+TRC conversion — and wiring through every codec).

## Test Coverage Gaps

### T-1: No tests for the format-conversion paths in `Image::read_pixels`
**Description:** Apart from the same-format fast path being exercised in `test_image_subset`/`test_image_from_raster`, there is no test that calls `read_pixels` with a destination info that differs in color/alpha type. The TODO in C-1 is in untested code.
**Effort:** Small.

### T-2: PNG/JPEG/WebP/AVIF encode-decode round-trips only tested for BMP
**Description:** `test_bmp_encode_decode_roundtrip` verifies format detection and decode of a BMP. There is no equivalent test for PNG, JPEG, WebP, GIF, ICO, AVIF, or RAW. The feature-gated format codecs (`#[cfg(feature = "png")]`, etc.) are only tested when their feature is enabled, and even then only via `test_format_detection` which checks magic bytes, not decode correctness.
**Effort:** Medium (bundle tiny test fixtures per format; add round-trip tests behind each feature flag).

### T-3: No tests for `EncodedImageGenerator` decode caching or format mismatch
**Description:** The six tests in `generator.rs` cover `SolidColorGenerator` (trivial fixed-color) and the `convert_pixels` RGBA↔BGRA path. `EncodedImageGenerator` — the more interesting generator — has no test. The C-4 bug around redundant decode and silent format mismatch is completely untested.
**Effort:** Small-Medium.

### T-4: `GpuImage` tests only verify raster-path bookkeeping
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
