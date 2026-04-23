# skia-rs-paint Gap Analysis

**Date:** 2026-04-23
**Reviewer:** Claude (Opus 4.6)

## Summary
- Total public functions reviewed: 182 (`pub fn` declarations; includes trait methods)
- Total test functions: 17 (`#[test]` annotations)
- Total test cases: 17 (cargo test output, all passing)
- Total gaps found: 31
- Critical gaps: 12
- Nice-to-have gaps: 12
- Test coverage gaps: 7
- Estimated complexity: Medium-High (shader sample() stubs + blend operations + WGSL backend incomplete)

## Files Reviewed
- [x] lib.rs (26 lines)
- [x] paint.rs (443 lines)
- [x] shader.rs (1277 lines)
- [x] filter.rs (810 lines)
- [x] blend.rs (164 lines)
- [x] runtime_effect.rs (1036 lines)
- [x] sksl.rs (1745 lines)

## Critical Gaps

### C-1: Paint missing mask_filter, color_filter, image_filter, and path_effect fields
**File:** `paint.rs` (lines 48-69)
**Severity:** Critical
**Description:** The `Paint` struct stores `shader` but has no fields for `mask_filter`, `color_filter`, `image_filter`, or `path_effect` (stroke dash/trim). Skia's `SkPaint` supports all of these. Without them, filter types defined in `filter.rs` cannot be attached to a Paint and are dead data structures.
**Impact:** The entire filter module (`filter.rs`, 810 lines) is unreachable from the drawing pipeline. Users cannot apply blur, drop shadow, lighting, morphology, or color matrix filters through Paint.
**Effort:** Small (add 4 fields + getters/setters, ~60 lines)

### C-2: Paint serialization does not round-trip shader, mask_filter, color_filter, or image_filter
**File:** `paint.rs` (lines 289-394)
**Severity:** Critical
**Description:** `serialize()`/`deserialize()` only encode color, blend mode, style, stroke params, and flags. The shader field is silently dropped (line 384 comment: "Shaders are not serialized"). Once C-1 is fixed, filters would also be lost.
**Impact:** Any Paint with a shader or filter loses that information on serialization round-trip.
**Effort:** Medium (need a serialization scheme for trait objects, possibly via ShaderKind discriminant)

### C-3: TwoPointConicalGradient has no sample() implementation
**File:** `shader.rs` (lines 654-666)
**Severity:** Critical
**Description:** `TwoPointConicalGradient` implements `Shader` but does not override the default `sample()` method. The default returns `Color4f::transparent()` for all inputs. All other gradient types (Linear, Radial, Sweep) have real sample() implementations.
**Impact:** Two-point conical gradients render as fully transparent in software rasterization.
**Effort:** Medium (requires solving quadratic equation for conical gradient t-value)

### C-4: BlendShader has no sample() implementation
**File:** `shader.rs` (lines 846-859)
**Severity:** Critical
**Description:** `BlendShader` does not override `sample()`. It stores `dst` and `src` shaders plus a `BlendMode`, but never samples them or applies the blend operation. Returns transparent by default.
**Impact:** Blend shaders render as fully transparent in software rasterization.
**Effort:** Medium (requires BlendMode::apply(src, dst) operation, see C-6)

### C-5: PerlinNoiseShader has no sample() implementation
**File:** `shader.rs` (lines 955-967)
**Severity:** Critical
**Description:** `PerlinNoiseShader` does not override `sample()`. Claims `is_opaque() == true` but returns transparent from the default sample(). No Perlin noise generation algorithm is implemented.
**Impact:** Perlin noise shaders render as fully transparent. The `is_opaque()` returning true is semantically incorrect given the actual sample() behavior.
**Effort:** High (requires implementing Perlin noise algorithm with octaves, both fractal noise and turbulence variants)

### C-6: BlendMode has no apply/blend operation
**File:** `blend.rs` (lines 1-164)
**Severity:** Critical
**Description:** `BlendMode` is a pure enum with classification methods (`is_porter_duff`, `is_separable`, `is_non_separable`) but no `fn apply(&self, src: Color4f, dst: Color4f) -> Color4f` method. Without this, blend modes are metadata only -- they cannot actually blend colors.
**Impact:** The entire blend system is non-functional for software rendering. BlendShader, ComposeShader, and any filter that uses BlendMode cannot produce correct results.
**Effort:** Medium-High (29 blend modes to implement with correct formulas; Porter-Duff modes are straightforward, separable modes moderate, non-separable Hue/Saturation/Color/Luminosity require HSL conversion)

### C-7: ComposeShader and LocalMatrixShader have no sample() implementations
**File:** `shader.rs` (lines 1001-1013, 1054-1066)
**Severity:** Critical
**Description:** Neither `ComposeShader` nor `LocalMatrixShader` override `sample()`. `LocalMatrixShader` should transform coordinates through its matrix then delegate to the inner shader. `ComposeShader` should sample both children and blend the results.
**Impact:** Composed and matrix-transformed shaders render as fully transparent.
**Effort:** Small-Medium (LocalMatrixShader just needs matrix inverse transform + delegation; ComposeShader needs blend operation from C-6)

### C-8: ImageShader has no sample() implementation and holds no image data
**File:** `shader.rs` (lines 672-805)
**Severity:** Critical
**Description:** `ImageShader` stores bounds, tile modes, and sampling options, but holds no actual image/pixel data. It cannot sample because there are no pixels to read from. The `sample()` method is not overridden.
**Impact:** Image shaders are non-functional -- they are metadata containers with no image data.
**Effort:** High (requires defining an image/pixmap data type and implementing tiled sampling with filtering)

### C-9: RuntimeShader.sample() returns hardcoded magenta
**File:** `runtime_effect.rs` (lines 932-936)
**Severity:** Critical
**Description:** `RuntimeShader::sample()` returns `Color4f::new(1.0, 0.0, 1.0, 1.0)` (magenta) with a comment "would need interpreter." This is an acknowledged stub.
**Impact:** Runtime shaders cannot produce correct colors in software rendering.
**Effort:** Very High (requires implementing a SkSL bytecode interpreter or tree-walking evaluator)

### C-10: RuntimeColorFilter.filter_color() is a no-op
**File:** `runtime_effect.rs` (lines 958-961)
**Severity:** Critical
**Description:** `RuntimeColorFilter::filter_color()` returns the input color unchanged with a comment "would need interpreter." This is a passthrough stub.
**Impact:** Runtime color filters have no effect.
**Effort:** Very High (same interpreter requirement as C-9)

### C-11: WGSL code generation is incomplete
**File:** `runtime_effect.rs` (lines 633-691)
**Severity:** Critical
**Description:** `stmt_to_wgsl()` handles only Return, VarDecl, and Expr statements. All other statements (If, For, While, DoWhile, Block, Break, Continue, Discard) emit `// Unsupported statement`. The `expr_to_wgsl()` also falls back to `/* unsupported */` for Unary, Assign, CompoundAssign, Index, Ternary, PostIncDec, and PreIncDec expressions.
**Impact:** Any non-trivial SkSL program compiled to WGSL will have broken/missing logic.
**Effort:** Medium (GLSL backend is complete and can be used as a template; mostly mechanical translation)

### C-12: MSL code generation reuses GLSL statement emission
**File:** `runtime_effect.rs` (line 775)
**Severity:** Critical
**Description:** `function_to_msl()` calls `self.stmt_to_glsl(&func.body, 0)` to emit the function body. While GLSL and MSL syntax are similar for basic constructs, this produces invalid MSL for type constructors (GLSL uses `vec4(...)` while MSL uses `float4(...)`), `discard` vs `discard_fragment()`, and other MSL-specific syntax.
**Impact:** MSL output is syntactically incorrect for many programs. Simple shaders may accidentally work, but any shader using type constructors will fail Metal compilation.
**Effort:** Medium (needs dedicated MSL expression/statement emitters, similar to WGSL work)

## Nice-to-Have Gaps

### N-1: ColorMatrixFilter missing convenience constructors
**File:** `filter.rs` (lines 14-64)
**Severity:** Nice-to-have
**Description:** Only `identity()` and `saturation()` convenience constructors exist. Skia provides `brightness()`, `contrast()`, `hue_rotate()`, `invert()`, `sepia()`, and `grayscale()` as common color matrix presets.
**Effort:** Small (pure math, ~50 lines)

### N-2: GradientFlags not using bitflags crate
**File:** `shader.rs` (lines 100-107)
**Severity:** Nice-to-have
**Description:** `GradientFlags` is a manual `struct(u32)` with const values. The crate already depends on `bitflags` (per Cargo.toml) but does not use it for this type. The `INTERPOLATE_PREMUL` flag is stored but never consulted during gradient interpolation.
**Impact:** The premultiplied interpolation flag is ignored in all gradient sample() implementations.
**Effort:** Small (switch to bitflags! macro, add premul path to interpolate_gradient_color)

### N-3: MaskFilter trait is too narrow
**File:** `filter.rs` (lines 94-97)
**Severity:** Nice-to-have
**Description:** The `MaskFilter` trait only has `fn blur_radius(&self) -> Option<Scalar>`. It has no method to actually apply the mask filter to a mask/alpha channel. `ShaderMaskFilter` and `TableMaskFilter` implement the trait but can only return `None` for blur_radius.
**Effort:** Medium (needs `fn apply_mask(&self, mask: &mut [u8], width: usize, height: usize)` or similar)

### N-4: ImageFilter trait only computes bounds, cannot apply filter
**File:** `filter.rs` (line 130-133)
**Severity:** Nice-to-have
**Description:** The `ImageFilter` trait only has `fn filter_bounds(&self, src: &Rect) -> Rect`. There is no `fn apply()` or `fn filter_image()` method. All 12 ImageFilter implementations compute correct bounds but cannot actually filter pixel data.
**Effort:** High (requires pixel buffer type and filter kernels for blur, morphology, convolution, etc.)

### N-5: BlurImageFilter stores tile_mode but never uses it
**File:** `filter.rs` (lines 137-161)
**Severity:** Nice-to-have
**Description:** Compiler warning confirms `tile_mode` field is never read. The `filter_bounds()` implementation does not consult tile mode.
**Effort:** Trivial (either remove field or use it when apply() is implemented per N-4)

### N-6: Many filter struct fields are never read (compiler warnings)
**File:** `filter.rs` (multiple locations)
**Severity:** Nice-to-have
**Description:** The compiler emits 12 dead_code warnings for fields across `DropShadowImageFilter`, `MorphologyImageFilter`, `ColorFilterImageFilter`, `DisplacementMapImageFilter`, `LightingImageFilter`, `OffsetImageFilter`, `MatrixConvolutionImageFilter`, `TileImageFilter`, `BlendImageFilter`, `ArithmeticImageFilter`. These fields are stored but only used by `filter_bounds()` when relevant.
**Impact:** Not a correctness issue yet, but indicates the filter types are data containers without operational methods.
**Effort:** Resolves automatically when apply() methods are added per N-4

### N-7: RuntimeEffect caches are never populated
**File:** `runtime_effect.rs` (lines 195-197, 267-275)
**Severity:** Nice-to-have
**Description:** `glsl_cache` and `wgsl_cache` fields exist on `RuntimeEffect` but are always initialized to `None` and never set. The `compile_to()` method regenerates output on every call.
**Impact:** Minor performance issue for repeated compilation.
**Effort:** Trivial (populate caches in compile_to, requires interior mutability or &mut self)

### N-8: RuntimeEffect ignores EffectKind during compilation
**File:** `runtime_effect.rs` (line 216)
**Severity:** Nice-to-have
**Description:** The `_kind: EffectKind` parameter in `compile()` is prefixed with underscore and unused. Shaders, color filters, and blenders should have different entry point validation (e.g., shaders must have `main(vec2) -> vec4`, color filters `main(vec4) -> vec4`).
**Effort:** Small (add entry point validation per kind)

### N-9: SPIR-V compilation returns error
**File:** `runtime_effect.rs` (lines 310-312)
**Severity:** Nice-to-have
**Description:** `compile_to(ShaderTarget::SpirV)` always returns `Err("SPIR-V compilation not yet implemented")`.
**Impact:** Cannot target Vulkan directly from SkSL.
**Effort:** Very High (SPIR-V is a binary format requiring a full compiler backend)

### N-10: SkSL parser does not validate semantic correctness
**File:** `sksl.rs` (entire module)
**Severity:** Nice-to-have
**Description:** The parser builds a syntactic AST but performs no type checking, scope resolution, or semantic validation. Undeclared variables, type mismatches, and invalid operations are silently accepted.
**Effort:** High (requires implementing a type checker and symbol table)

### N-11: SkSL parser does not handle `layout` qualifier or `#define`/`#ifdef` preprocessor directives
**File:** `sksl.rs` (lines 53-55)
**Severity:** Nice-to-have
**Description:** `Layout` token is defined but the parser does not handle `layout(...)` annotations. No preprocessor is implemented.
**Effort:** Medium

### N-12: `thiserror` and `bitflags` crate dependencies are declared but unused
**File:** `Cargo.toml` (lines 25-26)
**Severity:** Nice-to-have
**Description:** `RuntimeEffectError` implements `Display` and `Error` manually rather than using `thiserror`. `GradientFlags` is a manual bitfield rather than using `bitflags`. Both dependencies are unused dead weight.
**Effort:** Trivial (either use them or remove from Cargo.toml)

## Test Coverage Gaps

### T-1: No tests for blend.rs module
**Description:** The `BlendMode` enum has zero test coverage. `name()`, `from_u8()`, `is_porter_duff()`, `is_separable()`, and `is_non_separable()` are all untested.
**Effort:** Small

### T-2: No tests for filter.rs module
**Description:** All 3 traits and 12 filter struct types have zero test coverage. No test verifies `filter_color()` on `ColorMatrixFilter` or `filter_bounds()` on any `ImageFilter`.
**Effort:** Medium

### T-3: Shader sample() methods have zero test coverage
**Description:** The actual color computation in `LinearGradient::sample()`, `RadialGradient::sample()`, `SweepGradient::sample()`, and `ColorShader::sample()` is never tested. Existing shader tests only check `is_opaque()` and `shader_kind()`.
**Effort:** Medium (need to verify gradient interpolation at known positions)

### T-4: Paint serialization edge cases untested
**Description:** Only one round-trip and one invalid-input test exist. Missing: maximum values, boundary blend modes, NaN/infinity stroke widths, exact 17-byte minimum, trailing garbage bytes.
**Effort:** Small

### T-5: SkSL parser tested only for trivial programs
**Description:** Parser tests cover: basic function, uniforms. Missing: struct declarations, for/while/do-while loops, if/else, ternary, compound assignment, arrays, nested expressions, error recovery, edge cases (empty programs, multiple functions).
**Effort:** Medium

### T-6: Runtime effect compilation output not validated for correctness
**Description:** Tests check that output `contains` certain strings but do not verify the generated code is syntactically valid GLSL/WGSL/MSL. No test feeds the output to a shader validator.
**Effort:** Medium-High (would need optional dev-dependency on naga or similar for validation)

### T-7: No proptest/fuzz tests despite proptest dev-dependency
**Description:** `proptest` is listed in `[dev-dependencies]` but no property-based tests exist anywhere in the crate.
**Effort:** Medium

## Implementation Notes

### Architecture
The crate follows a clean trait-based design: `Shader`, `ColorFilter`, `MaskFilter`, `ImageFilter` are trait objects behind `Arc`. This is good for extensibility but makes serialization harder (C-2).

### Pattern: Data containers without operations
Many types (especially in `filter.rs`) are well-structured data containers with correct field definitions but lack the operational methods to actually perform their function. The `filter_bounds()` implementations are correct and non-trivial, showing good understanding of each filter's spatial behavior. The gap is in pixel-level application.

### SkSL pipeline quality
The SkSL lexer and parser are genuinely functional -- they correctly tokenize and parse real SkSL programs with uniforms, functions, expressions, and control flow. The GLSL code generation backend is complete and produces valid output. The WGSL and MSL backends are partial.

### Shader sampling
The gradient shaders (Linear, Radial, Sweep) have correct, non-trivial sample() implementations that properly handle tile modes and color interpolation. The helper functions `apply_tile_mode()` and `interpolate_gradient_color()` are correct. The gap is in the remaining shader types (Conical, Blend, Noise, Compose, LocalMatrix, Image).

### Compiler warnings
15 compiler warnings are emitted, all in `filter.rs` and `runtime_effect.rs`. Predominantly dead-code warnings for fields that exist but are only used for construction, not computation.

## Recommendations

### Priority 1: Make the Paint usable (C-1, C-6)
1. Add `mask_filter`, `color_filter`, `image_filter` fields to `Paint` with getters/setters (C-1)
2. Implement `BlendMode::apply(src, dst) -> Color4f` for all 29 modes (C-6)

These two changes unlock the rest of the system. Estimated effort: 1-2 days.

### Priority 2: Complete shader sample() implementations (C-3, C-4, C-5, C-7, C-8)
1. `TwoPointConicalGradient::sample()` -- medium math (C-3)
2. `LocalMatrixShader::sample()` -- small, needs matrix inverse (C-7)
3. `BlendShader::sample()` -- small once C-6 is done (C-4)
4. `ComposeShader::sample()` -- small once C-6 is done (C-7)
5. `PerlinNoiseShader::sample()` -- high, standalone algorithm (C-5)
6. `ImageShader` -- high, needs pixel data type (C-8)

Estimated effort: 3-5 days.

### Priority 3: Complete cross-compilation backends (C-11, C-12)
1. WGSL statement/expression codegen (C-11)
2. MSL dedicated codegen (C-12)

Estimated effort: 1-2 days (GLSL backend provides reference).

### Priority 4: Test coverage (T-1 through T-7)
Add tests for blend modes, filters, shader sampling, and SkSL edge cases. The proptest dependency is already available. Estimated effort: 2-3 days.

### Priority 5: Runtime shader interpretation (C-9, C-10)
Implementing a SkSL interpreter is a large undertaking. Consider whether software fallback is needed or if GPU compilation (GLSL/WGSL/MSL) is sufficient. Estimated effort: 1-2 weeks if pursued.
