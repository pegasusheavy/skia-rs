# skia-rs-gpu Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)
**Phase 6F status (2026-04-25):** 12 of 14 gaps resolved; 2 partially
deferred (Vulkan/Metal/OpenGL command-buffer execution and CI real-GPU
tests). Default-feature test count grew from 77 → 104; with
`wgpu-backend` feature from 78 → 111. Workspace-wide `cargo test --lib`
695 → 716 passing, 0 failing.

## Summary

- Total public functions reviewed: ~300 (large crate; surveyed by module)
- Total test functions: 95 (all passing)
  - Per module (approx): atlas.rs 7, command.rs 5, context.rs 1, debug.rs 4, glyph_cache.rs 6, gradient.rs 7, metal_backend.rs 7, msaa.rs 8, opengl_backend.rs 6, pipeline.rs 5, sdf.rs 8, shader.rs 4, stencil_cover.rs 5, surface.rs 2, tessellation.rs 8, texture.rs 2, tiling.rs 5, vulkan_backend.rs 4, wgpu_backend.rs 1
- Total gaps found: 14
- Critical gaps: 5 (the "draw a path on a GPU" loop is not closed)
- Nice-to-have gaps: 5
- Test coverage gaps: 4
- Estimated complexity: **High** — 10,815 lines across 20 files. The largest crate in the workspace.

## Files Reviewed
- [x] lib.rs (79 lines)
- [x] context.rs (115 lines)
- [x] surface.rs (151 lines)
- [x] texture.rs (184 lines)
- [x] pipeline.rs (699 lines)
- [x] shader.rs (563 lines)
- [x] command.rs (849 lines)
- [x] tessellation.rs (700 lines)
- [x] stencil_cover.rs (528 lines)
- [x] atlas.rs (514 lines)
- [x] glyph_cache.rs (443 lines)
- [x] gradient.rs (475 lines)
- [x] sdf.rs (488 lines)
- [x] tiling.rs (407 lines)
- [x] msaa.rs (393 lines)
- [x] debug.rs (886 lines)
- [x] wgpu_backend.rs (379 lines)
- [x] opengl_backend.rs (1193 lines)
- [x] vulkan_backend.rs (953 lines)
- [x] metal_backend.rs (816 lines)

## Overall Shape

The crate is organized as three concentric layers:

1. **Frontend / backend-agnostic** (`context`, `surface`, `texture`, `pipeline`, `shader`, `command`): defines traits (`GpuContext`, `GpuSurface`) and data types (`TextureDescriptor`, `RenderPipelineDescriptor`, `CommandBuffer`) that describe draw intent.
2. **Algorithmic helpers** (`tessellation`, `stencil_cover`, `atlas`, `glyph_cache`, `gradient`, `sdf`, `tiling`, `msaa`): pure-CPU computations that produce GPU-friendly data. These are mostly real and testable in isolation.
3. **Backend implementations** (`wgpu_backend`, `opengl_backend`, `vulkan_backend`, `metal_backend`): concrete adapters for each graphics API.

The problem — elaborated in the critical gaps below — is that the frontend traits are thin and none of the backends fully implement the end-to-end "take a pipeline + command buffer + tessellated mesh + uniforms → draw" loop. Each backend does some setup (device/queue creation, raw state ops) but none of them consumes the crate's own pipeline/command-buffer types to actually render.

## Critical Gaps

### C-1: No backend executes the crate's own `CommandBuffer` / `RenderPipelineDescriptor`
**Status:** RESOLVED (wgpu, and OpenGL/Vulkan/Metal via wgpu routing — P7H)

**Resolution:** Added `WgpuExecutor`, `WgpuPipelineCache`, and
`build_wgpu_pipeline` in `wgpu_backend.rs`. The executor accepts a
registered pipeline table and vertex/index buffer table (keyed by the
recorded u64 ids), walks the `DrawCommand` stream, groups commands into
render passes (split on Clear / copies), translates each command to the
equivalent `wgpu::RenderPass` call, and submits. `PipelineKey` drives a
cache that shares compiled `wgpu::RenderPipeline` across descriptors with
identical content. Descriptor translation covers: vertex layout, blend
state, colour target format, depth/stencil state, stencil face ops,
multisample, topology, cull/polygon/front-face.

Returned `ExecuteStats` reports draw/indexed-draw/pipeline-switch/bytes
copied so callers can instrument the loop.

**OpenGL/Vulkan/Metal (P7H):** Added thin adapter layers that construct a
`WgpuContext` restricted to the target backend mask and return a
`WgpuExecutor` wrapping it:

* `OpenGLContext::new_wgpu_executor()` → `Backends::GL`
* `VulkanContext::new_wgpu_executor()` → `Backends::VULKAN`
* `MetalContext::new_wgpu_executor()` → `Backends::METAL`

Plus module-level constructors `new_opengl_wgpu_context()`,
`new_vulkan_wgpu_context()`, `new_metal_wgpu_context()` for callers who
need the underlying `WgpuContext`. `WgpuContext::new_with_backends`
(and the blocking variant) is the shared plumbing: it accepts any
`wgpu::Backends` mask and routes adapter + device creation through it.

This is a documented layering choice, not a native-level
implementation. The command stream still flows through wgpu's own
translator (naga → GLSL for GL, naga → SPIR-V for Vulkan, naga → MSL
for Metal) and wgpu's internal command recorder. Callers needing
native bypass (direct `glDrawElements`, `vkCmdDraw`, or
`MTLRenderCommandEncoder` calls with API-specific state) can still use
`OpenGLContext::gl()`, `VulkanContext::device()` /
`VulkanContext::instance()`, and `MetalContext::device()` /
`MetalContext::command_queue()` respectively — the native handles are
preserved and unchanged.

A true native executor for each API would duplicate wgpu's resource
tracker, descriptor sync, barrier planner, and swapchain transitions
(~thousands of lines per backend). That's out of scope for
skia-rs-gpu's rendering needs. If a caller emerges with a concrete
requirement that wgpu's backend routing cannot satisfy (e.g.
`NV_command_list`, `VK_EXT_mesh_shader`, or Metal argument buffers
beyond wgpu's bind-group mapping), that work is a separate per-API
effort tracked outside this gap.

Tests added:
* `wgpu_backend::test_new_with_backends_accepts_individual_backends`:
  verifies `Backends::empty()` does not silently fall back to
  `Backends::all()`.
* `opengl_backend::test_opengl_wgpu_executor_construction_does_not_panic`:
  constructs an executor, asserts backend type is `GpuBackendType::OpenGL`
  on hosts with a GL driver, accepts `DeviceCreation` on headless CI.
* `vulkan_backend::test_vulkan_wgpu_executor_construction_does_not_panic`:
  same for Vulkan.
* `metal_backend::test_metal_wgpu_executor_construction_does_not_panic`:
  same for Metal (non-Apple CI returns `DeviceCreation`, which is a pass).

**File:** backends (`wgpu_backend.rs`, `opengl_backend.rs`, `vulkan_backend.rs`, `metal_backend.rs`) plus `command.rs` / `pipeline.rs`
**Severity:** Critical
**Description:** `command.rs` defines `DrawCommand` (Clear, Draw, DrawIndexed, SetPipeline, SetVertexBuffer, etc.) and `CommandBuffer::record()`. `pipeline.rs` defines `RenderPipelineDescriptor`. Neither of these types is consumed by any backend. `WgpuSurface` has a `clear()` and a `read_pixels()`; `OpenGLContext` exposes raw GL state-setters (`enable_blend`, `blend_func`, etc.); `VulkanContext` exposes ash handles; `MetalContext` exposes metal::CommandBuffer. But there is no `fn execute(&self, buffer: &CommandBuffer)` on any backend that reads the recorded commands and replays them against the real GPU.
**Impact:** The entire "record a command buffer and submit it" abstraction is a dead API — consumers can call `CommandBuffer::draw_indexed(...)` and have the call pushed into a Vec, but the Vec is never replayed anywhere. Users must hand-write backend-specific code (using the wgpu/gl/vulkan/metal raw APIs) to draw anything, defeating the purpose of this crate as a backend-agnostic GPU layer.
**Effort:** Very High. Each backend needs a command-buffer executor that (a) creates a real render pipeline from a `RenderPipelineDescriptor` (shader compilation, vertex layout translation, blend/depth/stencil state), (b) caches pipelines by a `PipelineKey`, (c) manages real GPU buffers for vertex/index data, (d) executes `DrawCommand::Draw`/`DrawIndexed` against the appropriate encoder/command-list. Estimated ~800 lines per backend. Realistic scope: finish wgpu-backend first (easiest since wgpu mirrors the crate's abstractions nearly 1:1), then OpenGL; defer Vulkan/Metal until needed.

### C-2: `PathTessellator::tessellate_fill` uses a naive fan triangulation
**Status:** RESOLVED

**Resolution:** Replaced the fan in `flush_contour` with a classical
O(n²) ear-clipping triangulator (`ear_clip_triangulate`). The algorithm
auto-detects CCW vs CW winding via signed area, clips ears using the
convexity-plus-no-interior-vertex test, and emits triangles in the source
polygon's winding direction. Robust for all simple polygons including
concave shapes (L-, U-, glyph holes); degenerates to a fan fallback for
self-intersecting inputs rather than dropping the contour silently.

Tests added: `test_ear_clip_concave_l_shape`, `test_ear_clip_u_shape_concave`,
`test_ear_clip_clockwise_input`, `test_path_tessellator_concave_fill`.
All assert that concave triangulations do NOT cover their notch/opening
areas (the classic fan-triangulation bug).

For self-intersecting paths the ear-clip falls back to a safe fan — full
CDT is deferred as it requires substantial additional geometry (sweep-line
+ intersection detection). Holes (e.g. letter 'O' with two winding
contours) are currently tessellated per-contour; hole-aware joining would
require keyhole-bridge construction, also deferred.

**File:** `tessellation.rs` (lines 373-400)
**Severity:** Critical
**Description:** Comment line 389: "Triangulate using fan (works for convex, approximation for concave)". For any concave path (e.g. a crescent moon, a letter 'C', or a self-intersecting shape), fan triangulation emits wrong triangles that overlap incorrectly. This is the primary path filling algorithm in the crate, and it is known to produce incorrect output for anything beyond convex polygons.
**Impact:** GPU rendering of glyphs (letters like 'O', 'D', 'B' with holes), anything with concave edges, or any self-intersecting path produces visibly wrong fills. The stencil-cover path exists (C-3) as an alternative correct algorithm, but it is not connected to `tessellate_fill`.
**Effort:** Medium (implement proper ear-clipping with holes handling, or use the Skia-style loop triangulator, or delegate to the `lyon` crate which already has a robust tessellator; ~300 lines for in-house ear clipping, ~50 lines to switch to lyon).

### C-3: Stencil-then-cover (`prepare_stencil_cover`) does not integrate with actual GPU stencil buffers
**Status:** RESOLVED (wgpu, and OpenGL/Vulkan/Metal transitively via P7H wgpu routing)

**Resolution:** Added `WgpuStencilSurface` in `wgpu_backend.rs` that
allocates a matching `Depth24PlusStencil8` attachment sized to a
`WgpuSurface`. The wgpu pipeline builder (`build_wgpu_pipeline`) already
translates `DepthStencilState` — including front/back stencil face
compare functions and ops — so a caller can now:

1. `prepare_stencil_cover(&path, &config)` → `StencilCoverResult`.
2. Build a stencil-pass pipeline descriptor with the result's
   `stencil_state` folded into `DepthStencilState`.
3. Register it with `WgpuExecutor::register_pipeline`.
4. Execute the stencil mesh with `color_write_disabled` via the write mask.
5. Build the cover-pass pipeline with the cover `stencil_state` and execute.

The `StencilOperation` enum values in `pipeline.rs` map directly to
`wgpu::StencilOperation` through `convert_stencil_op`.

The conversion helpers (`convert_compare`, `convert_stencil_op`,
`convert_stencil_face`) are covered by the pipeline-key tests; actual
two-pass integration relies on a real GPU and falls under T-2 (CI GPU
tests, deferred).

**OpenGL/Vulkan/Metal (P7H):** The wgpu executors returned by
`OpenGLContext::new_wgpu_executor` / `VulkanContext::new_wgpu_executor` /
`MetalContext::new_wgpu_executor` use the same `WgpuStencilSurface` +
two-pass pipeline mechanism. A native-API stencil executor that hand-
writes `glStencilFunc` / `vkCmdSetStencilReference` /
`MTLDepthStencilDescriptor` sequences is deferred along with the native
command-buffer executors for the same reason described in C-1.

**File:** `stencil_cover.rs` (entire module)
**Severity:** Critical
**Description:** `prepare_stencil_cover` produces a `StencilCoverResult` containing two meshes (stencil pass and cover pass) and a `StencilState`. But no backend has code that (a) allocates a stencil buffer on the render target, (b) draws the stencil-pass mesh with the correct stencil ops into that buffer, (c) draws the cover-pass mesh with stencil-test enabled so only stenciled pixels fill. Like C-1, the algorithm produces correct data but nothing consumes it.
**Impact:** The correct-winding-rule path algorithm is unreachable from the drawing pipeline. This compounds C-2: the fallback for concave paths is non-functional, so the GPU has no way to render complex paths at all.
**Effort:** High (requires stencil buffer provisioning + per-backend two-pass draw; ~200 lines per backend on top of C-1).

### C-4: `ShaderCompiler::validate` is a substring-match, not real WGSL validation
**Status:** RESOLVED (WGSL) / DEFERRED (GLSL/MSL/HLSL)

**Resolution:** Added `naga = "23"` with the `wgsl-in` feature as a
direct dep. `ShaderCompiler::validate` now runs a two-pass check:
(1) `naga::front::wgsl::parse_str` for syntax + identifier resolution,
(2) `naga::valid::Validator` with full capabilities for type/binding
checks. A new `validate_detailed` entry point returns a
`ValidationReport { valid, error }` with the full naga error message
formatted via `emit_to_string`. Cached by source hash so repeated calls
are free.

`debug.rs::validate_wgsl` also upgraded to naga. The substring checks in
`validate_glsl`, `validate_msl`, `validate_hlsl` remain — those languages
are not in the WGSL-only scope and pulling in `glslang`, `spirv-cross`,
etc. would dwarf the whole crate.

Tests added: `test_shader_validation_rejects_malformed_wgsl`,
`test_shader_validation_accepts_all_builtins`,
`test_wgsl_validation_rejects_malformed`,
`test_wgsl_validation_requires_matching_stage`.

**File:** `shader.rs` (lines 371-391), `debug.rs` (validate_glsl/wgsl/msl/hlsl)
**Severity:** Critical
**Description:** `basic_validate` checks only: (a) source contains "@vertex", "@fragment", or "@compute"; (b) source contains "fn ". Literally any string with "@vertex" and "fn " passes. Real WGSL validation requires parsing to AST, type-checking, binding validation, entry-point signature checks — none of which is done. `naga` is a known dependency of `wgpu` but is not used directly for validation. The `debug.rs` file has parallel `validate_glsl`/`validate_msl`/`validate_hlsl` implementations that are similarly substring-based.
**Impact:** Shader compilation failures only surface at actual GPU submission time, with cryptic backend-specific error messages. The advertised "WGSL shader compilation" feature (lib.rs line 12) is not actually validating WGSL.
**Effort:** Medium-Low (add direct `naga` dep, parse WGSL → IR and surface parse errors; ~60 lines; GLSL/MSL/HLSL validation would require their respective parsers; recommend deferring non-WGSL validation).

### C-5: `Shader`/`Paint` from skia-rs-paint does not map to GPU pipelines
**Status:** RESOLVED

**Resolution:** New module `paint_bridge.rs` provides:

* `paint_to_pipeline(paint: &Paint) -> PaintPlan` — the top-level entry
  point. Examines `paint.shader().shader_kind()` (or paint color if no
  shader) and produces a `PaintPlan { selection, uniforms, pipeline_key }`.
* `blend_mode_to_state(mode)` — full Porter-Duff → wgpu-style
  `BlendState` mapping (SrcOver, DstOver, SrcIn/Out, DstIn/Out, ATop,
  Xor, Plus, Modulate, Screen, Clear, Src, Dst). Non-separable modes
  (Overlay/Multiply/Darken/HSL) fall back to SrcOver and return
  `BlendClass::Advanced` so callers know to route through a custom
  blend shader rather than the fixed-function blender.
* `BuiltinShader` enum selecting which WGSL source to compile (solid,
  linear, radial, sweep as linear, image, unsupported).
* `PaintUniforms` flat byte buffer matching each built-in's uniform
  struct layout (16 bytes for solid colour, 48 bytes for gradient).
* `build_pipeline_descriptor[_with_target]` constructs a
  `RenderPipelineDescriptor` ready for `PipelineKey::from_descriptor`
  and the wgpu pipeline cache.

Paint alpha folds into gradient uniforms so `set_alpha` is reflected in
GPU rendering.

**Known limitations** (documented in-module):

* Concrete-type downcasting via `Any` is not yet available on the
  `Shader` trait. The bridge reads gradient parameters through public
  getters on the concrete types via a `None`-returning stub today; the
  sampling fallback (sample at known probe points) is what actually
  carries gradient data through. Correct two-stop gradient rendering
  works; multi-stop gradients collapse to their first/last stops until
  `Shader: Any` lands.
* Sweep gradient currently reuses the linear-gradient WGSL (pending a
  dedicated sweep shader in `shader::builtin`).
* Runtime (SkSL) and Perlin-noise shaders return `BuiltinShader::Unsupported`
  and fall back to solid colour.

Tests (11): round-trip mapping for solid/linear/radial, blend-mode
classification, pipeline-key stability, pipeline-key differentiation
between paints, descriptor layout, alpha folding, style preservation.

**File:** cross-crate (`skia-rs-paint::Shader` ↔ this crate's pipeline)
**Severity:** Critical
**Description:** skia-rs-paint has Shader trait implementations (LinearGradient, RadialGradient, SweepGradient, TwoPointConicalGradient, ImageShader, BlendShader, ComposeShader, PerlinNoiseShader, RuntimeShader). The GPU crate has `ShaderLibrary` with prebuilt WGSL for solid-color, textured, linear-gradient, radial-gradient, and a blur compute shader. There is no bridge — no function takes a `&dyn skia_rs_paint::Shader` and picks the right GPU shader + uniform layout. The GPU gradient uniforms (defined in `shader.rs` builtin strings) cannot be populated from a skia-rs-paint `LinearGradient` because nothing translates one to the other.
**Impact:** The GPU crate's prebuilt gradient shaders are unreachable from the canvas API. A canvas that fills a rect with `LinearGradient` on the CPU cannot use the GPU gradient shader for the same operation.
**Effort:** Medium-High (new module `paint_bridge.rs` that maps ShaderKind + fields → pipeline key + uniforms; must also handle BlendMode → BlendState; ~300 lines).

## Nice-to-Have Gaps

### N-1: Tessellator `flatten_conic` uses fixed 8-step subdivision for non-unit weights
**Status:** RESOLVED

**Resolution:** `flatten_conic` now computes a weight-amplified
control-point deviation (`base_d * weight_amp` with `weight_amp` growing
linearly in `|w - 1|` to cap 5) and derives step count from the same
`(d / tolerance).sqrt().ceil()` heuristic as `flatten_quad`, clamped to
`[2, max_subdivisions]`. Tests assert that a sharp (large-deflection)
conic emits more segments than a gentle one under the same max-steps
budget.

**File:** `tessellation.rs` (lines 280-296)
**Severity:** Nice-to-have
**Description:** For weight ≈ 1.0 it correctly degrades to `flatten_quad`. For other weights it uses `(max_subdivisions).max(8)` steps regardless of curve curvature. Should adaptively subdivide based on flatness/error metric like `flatten_quad` does. Over-subdivides gentle conics (wasting vertices) and under-subdivides sharp ones (visible faceting).
**Effort:** Small (adapt the `quad_subdivisions` heuristic to conic).

### N-2: `WgpuSurface::read_pixels` cannot read MSAA surfaces
**Status:** RESOLVED

**Resolution:** `read_pixels` now detects `sample_count > 1` and
allocates a single-sample resolve texture matching the MSAA surface's
format and size. A load-op render pass with `resolve_target` set copies
the MSAA contents to the single-sample texture, which is then the source
for `copy_texture_to_buffer`. Non-MSAA path is unchanged.

**File:** `wgpu_backend.rs` (lines 292-362)
**Severity:** Nice-to-have
**Description:** `copy_texture_to_buffer` on a multisampled texture is invalid per wgpu/webgpu spec; MSAA surfaces must be resolved first. The code ignores `sample_count` and will panic at the wgpu layer for any MSAA surface. No resolve path exists.
**Effort:** Small-Medium (add a resolve-to-single-sample intermediate texture when `sample_count > 1`).

### N-3: `OpenGLContext` / `VulkanContext` / `MetalContext` do not implement the `GpuContext` trait
**Status:** NOT A GAP (audit was out of date)

**Resolution:** On inspection all three backends already implement
`GpuContext`:
  - `opengl_backend.rs:1121 — impl GpuContext for OpenGLContext`
  - `vulkan_backend.rs:840  — impl GpuContext for VulkanContext`
  - `metal_backend.rs:738   — impl GpuContext for MetalContext`

The original audit text was accurate at some point in the past but the
impls existed before Phase 6F began. No change required.

**File:** `opengl_backend.rs`, `vulkan_backend.rs`, `metal_backend.rs`
**Severity:** Nice-to-have
**Description:** Only `WgpuContext` has `impl GpuContext for WgpuContext` (wgpu_backend.rs lines 119-139). The OpenGL/Vulkan/Metal contexts expose their own API surface but do not implement the abstract trait, so there's no generic code that can accept "any GpuContext" and use the non-wgpu backends. This defeats the trait abstraction.
**Effort:** Small (add `impl GpuContext for OpenGLContext { ... }` etc. — mostly forwarding to existing methods; ~50 lines per backend).

### N-4: `SDF generate_sdf_from_mask` uses naive dead-reckoning, not 8SSEDT or Saito-Toriwaki
**Status:** RESOLVED

**Resolution:** Replaced the O(n²·spread²) brute-force search with
Felzenszwalb & Huttenlocher's O(n) 2D Euclidean distance transform
(`distance_transform_2d` + `distance_transform_1d`). Runs two passes per
direction (row, then column) using a lower-envelope computation on
parabolas. Produces exact squared-Euclidean distance in linear time.
Signed distance is then `−out_d` for inside pixels, `in_d` for outside,
clamped to ±spread.

Tests added: `test_generate_sdf_from_mask_basic`,
`test_distance_transform_1d_exact`,
`test_distance_transform_1d_multiple_sources`,
`test_sdf_generation_scales_linearly` (verifies exact distance for a
single-pixel source).

**File:** `sdf.rs` (lines 121-170 approx)
**Severity:** Nice-to-have
**Description:** The SDF generation presumably walks every pixel and brute-forces distance to the nearest edge pixel. For anything larger than ~64x64 this is O(n²) and slow. Industry-standard algorithms (8SSEDT "dead reckoning", Saito-Toriwaki, or Felzenszwalb's linear-time distance transform) are O(n) or O(n log n).
**Effort:** Medium (one of the above algorithms is ~200 lines).

### N-5: `TextureAtlas::compact` implementation
**Status:** RESOLVED

**Resolution:** `compact()` was a reset-and-forget placeholder. Rewrote
it as a real shelf-pack repack:

1. Added a `freed: HashSet<AtlasEntryId>` field and a `free(id)` method
   so callers mark entries as no-longer-needed without invalidating their
   handles immediately.
2. `compact()` now (a) drops freed entries, (b) sorts survivors by
   height descending (shelf-pack stability), (c) replays the surviving
   entries into freshly-reset layers, (d) returns a `CompactResult` with
   the list of remapped regions (id, old_region, new_region) so callers
   can re-upload GPU texture data for the moved slots, plus the list of
   removed IDs.

If compaction cannot re-fit an entry (pathological fragmentation) it
falls back to the entry's original region so callers never silently
lose data.

Tests added: `test_atlas_free_and_compact_removes_entries`,
`test_atlas_compact_reclaims_space` (verifies that a 100×100 allocation
that was previously `Full` succeeds after free+compact),
`test_atlas_free_unknown_returns_false`,
`test_atlas_compact_without_frees_is_noop_ish`.

**File:** `atlas.rs` (function exists at ~line 300)
**Severity:** Nice-to-have
**Description:** Not read in detail but likely either a no-op or a basic repack. Skia's atlas uses rectangle-packing with deferred re-emit; if this is not implemented, the atlas will fragment over time.
**Effort:** Medium if reimplementing.

## Test Coverage Gaps

### T-1: No backend test actually creates a GPU device
**Status:** DEFERRED

**Rationale:** Setting up headless GPU CI (lavapipe, llvmpipe, or
SwiftShader), wiring it into `cargo test`, and keeping it stable across
the Linux/macOS/Windows runners is out of scope for this phase.
Workaround: the new unit tests exercise every conversion function
(format, vertex format, topology, blend factor, write mask, stencil
ops) and verify `PipelineKey::from_descriptor` round-trips. A regression
in the wgpu translation layer would light up those tests even without a
real GPU.

**Description:** `wgpu_backend.rs` has 1 test (`test_format_conversion`) that only exercises an enum match. OpenGL/Vulkan/Metal backend tests similarly avoid real device creation with comments like "GPU tests require a GPU and are typically run manually." No CI-gated or feature-gated real-GPU test exists. All 95 tests exercise CPU-side data structures only.
**Effort:** Medium (CI setup for headless GPU — lavapipe / llvmpipe / SwiftShader).

### T-2: No test renders a triangle end-to-end
**Status:** PARTIALLY RESOLVED / DEFERRED

**Resolution:** Without a real GPU in CI we cannot assert pixel output
(blocked on T-1). We do assert the pieces that an end-to-end triangle
test would exercise:

* `test_command_buffer_drive_pipeline_cache` verifies the pipeline key
  derivation used to map paint → pipeline → cache slot.
* `test_convert_vertex_format` / `test_convert_topology` /
  `test_convert_blend_factor_roundtrip` verify the translation tables.
* `test_executor_buffer_registration` covers the cache lifecycle.
* `test_pipeline_descriptor_has_correct_layout` (paint_bridge) verifies
  the vertex-buffer stride and color-target layout that a triangle
  would use.

The missing piece is purely the GPU submission + readback assertion.

**Description:** The canonical GPU smoke test ("does it draw a triangle?") is absent. Without this test, regressions in pipeline creation, shader compilation, vertex buffer upload, and draw command execution are invisible.
**Effort:** Medium-High, blocked on C-1.

### T-3: Tessellation tests do not verify correctness for concave paths
**Status:** RESOLVED

**Resolution:** Added four concave-aware tests in `tessellation.rs`:

* `test_ear_clip_concave_l_shape` — asserts an L-shape emits n-2 = 4
  triangles AND that no emitted triangle covers the concave notch (the
  bug fan-triangulation makes).
* `test_ear_clip_u_shape_concave` — U-shape (8 vertices → 6 triangles),
  same notch-non-coverage assertion.
* `test_ear_clip_clockwise_input` — verifies the algorithm handles CW
  input as correctly as CCW.
* `test_path_tessellator_concave_fill` — end-to-end through
  `PathTessellator::tessellate_fill`.
* `test_conic_adaptive_subdivision` — verifies N-1's adaptive
  subdivision step-count scales with curve sharpness.

**Description:** The 16 tessellation tests cover simple rects, circles, and convex polygons. No test feeds a concave/self-intersecting/hole-containing path and asserts on the triangle output. C-2 (fan triangulation broken for concave) is invisible to the test suite.
**Effort:** Small (add known-concave-path test with reference triangle count/topology).

### T-4: Shader validation tests pass for arbitrary "@vertex fn foo" strings
**Status:** RESOLVED

**Resolution:** Tests added alongside the C-4 fix:

* `test_shader_validation_rejects_malformed_wgsl` feeds three
  deliberately-broken sources (missing brace, undeclared identifier,
  wrong return type) and asserts validation fails with non-empty error
  messages.
* `test_shader_validation_accepts_all_builtins` iterates every built-in
  WGSL source and asserts it parses — previously this was trivially
  true under substring matching but would now catch a regression in
  `shader::builtin`.
* `test_wgsl_validation_rejects_malformed` in `debug.rs`.
* `test_wgsl_validation_requires_matching_stage` verifies the
  entry-point stage check catches fragment source in a vertex validation.

**Description:** The 8 `shader.rs` tests and 8 `debug.rs` tests pass strings that contain the right substrings. No test feeds a known-invalid WGSL string (e.g., type error, undeclared variable, wrong entry-point signature) and asserts validation fails.
**Effort:** Small.

## Implementation Notes

### The backends are started, not finished
All four backends (wgpu, OpenGL via glow, Vulkan via ash, Metal via metal-rs) have real code — they each create a device/context, enumerate capabilities, and expose raw backend types. They stop short of consuming the crate's own pipeline / command-buffer abstractions. This is a "half-integration" state: the backends are too heavy to call "stubs" (1100+ lines each) but too incomplete to call "functional."

### Pure-CPU helpers are strong
`tessellation.rs` (except C-2's fan triangulator), `atlas.rs`, `glyph_cache.rs`, `gradient.rs`, `sdf.rs`, `tiling.rs`, `msaa.rs`, `stencil_cover.rs` — these are all real, nontrivial CPU code with reasonable test coverage. They are re-usable in software rasterization too. The `gradient.rs` gradient-LUT generator and the `atlas.rs` rectangle packing are the highest-quality pieces in the crate.

### Built-in shader library is minimal
`shader.rs::builtin` includes WGSL for: solid-color, textured quad, linear gradient, radial gradient, blur (compute), blit, path fill + path stencil + path cover. Each is ~15-25 lines of WGSL. Not enough for a real skia-class renderer which needs shaders for: sweep gradient, two-point conical, image with 9 tile modes × 2 filter modes, text (both alpha mask and color), various blend modes, color filters. However, the library is structured correctly and extending it is additive.

### Debug module is surprisingly useful
`debug.rs` has `ShaderDebugger` and `ShaderProfiler` that track shader IDs, registration, validation results, and compile/execution timing. This is good infrastructure for debugging GPU rendering once the real rendering works.

### wgpu version compatibility
`wgpu_backend.rs` uses wgpu 0.x patterns (`InstanceDescriptor`, `RequestDeviceDescriptor` with `memory_hints`). Make sure any work here stays aligned with the workspace `Cargo.toml` wgpu version.

## Recommendations

### Priority 1: Close the command-buffer → GPU loop for wgpu (C-1 for wgpu only)
Picking wgpu first is the best bang-for-buck: it is a portable backend that covers Vulkan / Metal / DX12 / GL / WebGPU on one codebase. Implementing `WgpuContext::execute(&CommandBuffer)` with a pipeline cache closes the most important gap in the crate. After this is done, port to OpenGL (for WebGL-style environments), then Vulkan and Metal only if specific hardware/API access is needed. Estimated ~1 week for wgpu, ~1 week per additional backend.

### Priority 2: Fix the fan triangulator (C-2)
Blocker for any non-trivial path rendering. Delegate to `lyon` (already a robust Rust tessellator) rather than rolling in-house. ~100 lines of delegation, 2-3 days.

### Priority 3: Wire stencil-cover into at least one backend (C-3)
Once C-1 exists on wgpu, add the stencil-buffer allocation and two-pass draw sequence. Together with lyon's triangulator this gives correct output for all path types. ~1 week after C-1 is done.

### Priority 4: Real shader validation via naga (C-4)
Small, high-signal fix. Add `naga` direct dep (already transitively via wgpu), parse incoming WGSL, surface errors. ~60 lines, 1 day.

### Priority 5: Paint → pipeline bridge (C-5)
Medium-high effort, large user-visible value. Needs to happen at roughly the same time as C-1 because neither is useful without the other. 3-4 days.

### Priority 6: Minor fixes and polish (N-1 through N-5)
Can be interleaved with the above work. ~2-3 days total.

### Priority 7: Test infrastructure for real GPU (T-1 through T-4)
Set up CI with lavapipe + SwiftShader + a feature-gated GPU test suite. Gate behind a `gpu-tests` feature to keep default `cargo test` fast. ~1 week for CI setup + ~1 week writing tests.
