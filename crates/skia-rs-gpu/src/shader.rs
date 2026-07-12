//! Shader compilation and management for GPU rendering.
//!
//! This module provides WGSL shader compilation, validation, and caching.

use std::collections::HashMap;

/// Shader stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// Vertex shader.
    Vertex,
    /// Fragment shader.
    Fragment,
    /// Compute shader.
    Compute,
}

/// Compiled shader module.
#[derive(Debug)]
pub struct ShaderModule {
    /// Source code.
    pub source: String,
    /// Entry points.
    pub entry_points: Vec<ShaderEntryPoint>,
    /// Shader stage.
    pub stage: ShaderStage,
}

/// Shader entry point.
#[derive(Debug, Clone)]
pub struct ShaderEntryPoint {
    /// Name.
    pub name: String,
    /// Stage.
    pub stage: ShaderStage,
}

/// Built-in WGSL shaders for common operations.
pub mod builtin {
    /// Solid color fill vertex shader.
    pub const SOLID_COLOR_VS: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct Uniforms {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    return output;
}
";

    /// Solid color fill fragment shader.
    pub const SOLID_COLOR_FS: &str = r"
struct Uniforms {
    color: vec4<f32>,
};

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Output premultiplied color: the pixel pipeline blends premultiplied
    // sources (matches Skia's raster pipeline / Ganesh). The uniform holds
    // straight (unpremultiplied) rgba, so premultiply here.
    let c = uniforms.color;
    return vec4<f32>(c.rgb * c.a, c.a);
}
";

    /// Textured quad vertex shader.
    pub const TEXTURED_VS: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

struct Uniforms {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    return output;
}
";

    /// Textured quad fragment shader.
    pub const TEXTURED_FS: &str = r"
@group(0) @binding(1)
var t_texture: texture_2d<f32>;
@group(0) @binding(2)
var s_sampler: sampler;

struct Uniforms {
    tint: vec4<f32>,
};

@group(0) @binding(3)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(@location(0) tex_coord: vec2<f32>) -> @location(0) vec4<f32> {
    let color = textureSample(t_texture, s_sampler, tex_coord);
    return color * uniforms.tint;
}
";

    /// Linear gradient vertex shader.
    pub const GRADIENT_VS: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_position: vec2<f32>,
};

struct Uniforms {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    output.local_position = input.position;
    return output;
}
";

    /// Linear gradient fragment shader.
    pub const LINEAR_GRADIENT_FS: &str = r"
struct GradientUniforms {
    start: vec2<f32>,
    end: vec2<f32>,
    color0: vec4<f32>,
    color1: vec4<f32>,
};

@group(0) @binding(1)
var<uniform> gradient: GradientUniforms;

@fragment
fn fs_main(@location(0) local_position: vec2<f32>) -> @location(0) vec4<f32> {
    let dir = gradient.end - gradient.start;
    let len = length(dir);
    if len < 0.0001 {
        return vec4<f32>(gradient.color0.rgb * gradient.color0.a, gradient.color0.a);
    }
    let norm_dir = dir / len;
    let t = clamp(dot(local_position - gradient.start, norm_dir) / len, 0.0, 1.0);
    let c = mix(gradient.color0, gradient.color1, t);
    // Emit premultiplied color for the premultiplied blend pipeline.
    return vec4<f32>(c.rgb * c.a, c.a);
}
";

    /// Radial gradient fragment shader.
    pub const RADIAL_GRADIENT_FS: &str = r"
struct GradientUniforms {
    center: vec2<f32>,
    radius: f32,
    _padding: f32,
    color0: vec4<f32>,
    color1: vec4<f32>,
};

@group(0) @binding(1)
var<uniform> gradient: GradientUniforms;

@fragment
fn fs_main(@location(0) local_position: vec2<f32>) -> @location(0) vec4<f32> {
    let dist = length(local_position - gradient.center);
    let t = clamp(dist / gradient.radius, 0.0, 1.0);
    let c = mix(gradient.color0, gradient.color1, t);
    // Emit premultiplied color for the premultiplied blend pipeline.
    return vec4<f32>(c.rgb * c.a, c.a);
}
";

    /// Blur compute shader.
    pub const BLUR_CS: &str = r"
@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var output_texture: texture_storage_2d<rgba8unorm, write>;

struct BlurParams {
    radius: i32,
    direction: vec2<f32>,
    _padding: f32,
};

@group(0) @binding(2)
var<uniform> params: BlurParams;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coord = vec2<i32>(global_id.xy);

    if coord.x >= i32(dims.x) || coord.y >= i32(dims.y) {
        return;
    }

    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;

    for (var i = -params.radius; i <= params.radius; i++) {
        let offset = vec2<i32>(params.direction * f32(i));
        let sample_coord = clamp(coord + offset, vec2<i32>(0), vec2<i32>(dims) - 1);
        let weight = 1.0 - abs(f32(i)) / f32(params.radius + 1);
        color += textureLoad(input_texture, sample_coord, 0) * weight;
        weight_sum += weight;
    }

    textureStore(output_texture, coord, color / weight_sum);
}
";

    /// Blit vertex shader (full-screen quad).
    pub const BLIT_VS: &str = r"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    var tex_coords = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.tex_coord = tex_coords[vertex_index];
    return output;
}
";

    /// Blit fragment shader.
    pub const BLIT_FS: &str = r"
@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_sampler: sampler;

@fragment
fn fs_main(@location(0) tex_coord: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_sampler, tex_coord);
}
";

    /// Path fill vertex shader (for stencil-then-cover).
    pub const PATH_FILL_VS: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct Uniforms {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    return output;
}
";

    /// Path fill fragment shader (for stencil).
    pub const PATH_STENCIL_FS: &str = r"
@fragment
fn fs_main() {
    // No color output, only writing to stencil
}
";

    /// Path cover fragment shader (for final color).
    pub const PATH_COVER_FS: &str = r"
struct Uniforms {
    color: vec4<f32>,
};

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Premultiplied output for the stencil-then-cover color pass.
    let c = uniforms.color;
    return vec4<f32>(c.rgb * c.a, c.a);
}
";
}

/// Detailed validation result for WGSL shaders.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// True if naga successfully parsed the source.
    pub valid: bool,
    /// Human-readable error message if parsing failed.
    pub error: Option<String>,
}

impl ValidationReport {
    /// Convenience: true when the shader parses.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    /// Get the error message, if any.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Shader compiler for WGSL shaders.
pub struct ShaderCompiler {
    /// Cached shader source validation results.
    validation_cache: parking_lot::RwLock<HashMap<u64, ValidationReport>>,
}

impl ShaderCompiler {
    /// Create a new shader compiler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            validation_cache: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Validate WGSL shader source.
    ///
    /// Returns true if the shader is valid WGSL according to naga's WGSL
    /// front-end parser. This is the same parser that wgpu uses internally,
    /// so a passing shader here will also be accepted by wgpu's pipeline
    /// creation (modulo target-specific limits).
    pub fn validate(&self, source: &str) -> bool {
        self.validate_detailed(source).valid
    }

    /// Validate WGSL and return a detailed report with the first parse error.
    ///
    /// Cached by source hash so repeated calls on the same source are free.
    pub fn validate_detailed(&self, source: &str) -> ValidationReport {
        let hash = Self::hash_source(source);

        if let Some(report) = self.validation_cache.read().get(&hash) {
            return report.clone();
        }

        let report = Self::naga_validate(source);
        self.validation_cache.write().insert(hash, report.clone());
        report
    }

    /// Parse WGSL via naga and produce a `ValidationReport`.
    ///
    /// Historically this function used substring matching ("@vertex" + "fn ")
    /// which accepted essentially any text. That defeated the point of
    /// advertising shader validation: malformed shaders only surfaced at
    /// pipeline-creation time with cryptic backend errors.
    ///
    /// We now run two passes:
    ///   1. `naga::front::wgsl::parse_str` — catches syntax errors and
    ///      undeclared-identifier errors.
    ///   2. `naga::valid::Validator` with the default capabilities — catches
    ///      semantic errors (wrong return type, bad bind-group layout,
    ///      incompatible operations, etc.) that only surface after the
    ///      module IR has been built.
    fn naga_validate(source: &str) -> ValidationReport {
        let module = match naga::front::wgsl::parse_str(source) {
            Ok(m) => m,
            Err(err) => {
                return ValidationReport {
                    valid: false,
                    error: Some(err.emit_to_string(source)),
                };
            }
        };

        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        match validator.validate(&module) {
            Ok(_info) => ValidationReport {
                valid: true,
                error: None,
            },
            Err(err) => ValidationReport {
                valid: false,
                error: Some(format!("WGSL validation failed: {err}")),
            },
        }
    }

    /// Hash shader source for caching.
    fn hash_source(source: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    /// Preprocess shader source with includes and defines.
    pub fn preprocess(&self, source: &str, defines: &HashMap<String, String>) -> String {
        let mut result = source.to_string();

        // Replace defines
        for (key, value) in defines {
            result = result.replace(&format!("${{{key}}}"), value);
        }

        result
    }

    /// Get a combined shader module source.
    pub fn combine_shaders(&self, vertex: &str, fragment: &str) -> String {
        format!("// Vertex Shader\n{vertex}\n\n// Fragment Shader\n{fragment}")
    }
}

impl Default for ShaderCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Shader library for managing commonly used shaders.
pub struct ShaderLibrary {
    /// Named shaders.
    shaders: HashMap<String, String>,
}

impl ShaderLibrary {
    /// Create a new shader library with built-in shaders.
    #[must_use]
    pub fn new() -> Self {
        let mut shaders = HashMap::new();

        // Add built-in shaders
        shaders.insert(
            "solid_color_vs".to_string(),
            builtin::SOLID_COLOR_VS.to_string(),
        );
        shaders.insert(
            "solid_color_fs".to_string(),
            builtin::SOLID_COLOR_FS.to_string(),
        );
        shaders.insert("textured_vs".to_string(), builtin::TEXTURED_VS.to_string());
        shaders.insert("textured_fs".to_string(), builtin::TEXTURED_FS.to_string());
        shaders.insert("gradient_vs".to_string(), builtin::GRADIENT_VS.to_string());
        shaders.insert(
            "linear_gradient_fs".to_string(),
            builtin::LINEAR_GRADIENT_FS.to_string(),
        );
        shaders.insert(
            "radial_gradient_fs".to_string(),
            builtin::RADIAL_GRADIENT_FS.to_string(),
        );
        shaders.insert("blur_cs".to_string(), builtin::BLUR_CS.to_string());
        shaders.insert("blit_vs".to_string(), builtin::BLIT_VS.to_string());
        shaders.insert("blit_fs".to_string(), builtin::BLIT_FS.to_string());
        shaders.insert(
            "path_fill_vs".to_string(),
            builtin::PATH_FILL_VS.to_string(),
        );
        shaders.insert(
            "path_stencil_fs".to_string(),
            builtin::PATH_STENCIL_FS.to_string(),
        );
        shaders.insert(
            "path_cover_fs".to_string(),
            builtin::PATH_COVER_FS.to_string(),
        );

        Self { shaders }
    }

    /// Get a shader by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.shaders.get(name).map(std::string::String::as_str)
    }

    /// Add a custom shader.
    pub fn add(&mut self, name: impl Into<String>, source: impl Into<String>) {
        self.shaders.insert(name.into(), source.into());
    }

    /// Check if a shader exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.shaders.contains_key(name)
    }

    /// List all shader names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.shaders.keys().map(std::string::String::as_str)
    }
}

impl Default for ShaderLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_validation() {
        let compiler = ShaderCompiler::new();

        // Valid vertex shader
        assert!(compiler.validate(builtin::SOLID_COLOR_VS));

        // Valid fragment shader
        assert!(compiler.validate(builtin::SOLID_COLOR_FS));

        // Invalid: not WGSL at all.
        assert!(!compiler.validate("let x = 1;"));
    }

    #[test]
    fn test_shader_validation_rejects_malformed_wgsl() {
        // Regression test for C-4/T-4: the old substring check accepted any
        // string containing "@vertex" and "fn ". naga-backed validation
        // must reject shaders with real syntax/type errors.
        let compiler = ShaderCompiler::new();

        // Syntax error: missing closing brace.
        let bad_syntax = r"
@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
";
        assert!(!compiler.validate(bad_syntax));

        // Undeclared identifier.
        let undeclared = r"
@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    return undeclared_variable;
}
";
        assert!(!compiler.validate(undeclared));

        // Wrong type in return.
        let type_error = r"
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return 42;
}
";
        assert!(!compiler.validate(type_error));

        // The detailed report should include an error string for these.
        let report = compiler.validate_detailed(bad_syntax);
        assert!(!report.is_valid());
        assert!(report.error().is_some());
    }

    #[test]
    fn test_shader_validation_accepts_all_builtins() {
        // Every built-in WGSL shader must actually parse — previously this
        // was implicitly true because the substring check was trivial.
        let compiler = ShaderCompiler::new();
        for (name, src) in [
            ("SOLID_COLOR_VS", builtin::SOLID_COLOR_VS),
            ("SOLID_COLOR_FS", builtin::SOLID_COLOR_FS),
            ("TEXTURED_VS", builtin::TEXTURED_VS),
            ("TEXTURED_FS", builtin::TEXTURED_FS),
            ("GRADIENT_VS", builtin::GRADIENT_VS),
            ("LINEAR_GRADIENT_FS", builtin::LINEAR_GRADIENT_FS),
            ("RADIAL_GRADIENT_FS", builtin::RADIAL_GRADIENT_FS),
            ("BLUR_CS", builtin::BLUR_CS),
            ("BLIT_VS", builtin::BLIT_VS),
            ("BLIT_FS", builtin::BLIT_FS),
            ("PATH_FILL_VS", builtin::PATH_FILL_VS),
            ("PATH_STENCIL_FS", builtin::PATH_STENCIL_FS),
            ("PATH_COVER_FS", builtin::PATH_COVER_FS),
        ] {
            let report = compiler.validate_detailed(src);
            assert!(
                report.is_valid(),
                "built-in shader {} failed WGSL validation: {:?}",
                name,
                report.error()
            );
        }
    }

    #[test]
    fn test_color_shaders_output_premultiplied() {
        // Regression: solid + gradient fragment shaders must premultiply their
        // color output so it feeds the premultiplied blend pipeline. Assert on
        // the WGSL content and that it still validates.
        let compiler = ShaderCompiler::new();
        for (name, src) in [
            ("SOLID_COLOR_FS", builtin::SOLID_COLOR_FS),
            ("LINEAR_GRADIENT_FS", builtin::LINEAR_GRADIENT_FS),
            ("RADIAL_GRADIENT_FS", builtin::RADIAL_GRADIENT_FS),
            ("PATH_COVER_FS", builtin::PATH_COVER_FS),
        ] {
            assert!(
                src.contains(".rgb * ") && src.contains(".a"),
                "shader {name} must premultiply rgb by alpha"
            );
            assert!(
                compiler.validate(src),
                "shader {name} must remain valid WGSL"
            );
        }
    }

    #[test]
    fn test_shader_library() {
        let library = ShaderLibrary::new();

        assert!(library.contains("solid_color_vs"));
        assert!(library.contains("solid_color_fs"));
        assert!(library.contains("textured_vs"));
        assert!(library.contains("blur_cs"));

        let vs = library.get("solid_color_vs").unwrap();
        assert!(vs.contains("@vertex"));
    }

    #[test]
    fn test_shader_preprocess() {
        let compiler = ShaderCompiler::new();
        let source = "const VALUE: f32 = ${MY_VALUE};";
        let mut defines = HashMap::new();
        defines.insert("MY_VALUE".to_string(), "42.0".to_string());

        let processed = compiler.preprocess(source, &defines);
        assert_eq!(processed, "const VALUE: f32 = 42.0;");
    }

    #[test]
    fn test_builtin_shaders() {
        // Verify all builtin shaders have required components
        let compiler = ShaderCompiler::new();

        assert!(compiler.validate(builtin::SOLID_COLOR_VS));
        assert!(compiler.validate(builtin::SOLID_COLOR_FS));
        assert!(compiler.validate(builtin::TEXTURED_VS));
        assert!(compiler.validate(builtin::TEXTURED_FS));
        assert!(compiler.validate(builtin::GRADIENT_VS));
        assert!(compiler.validate(builtin::LINEAR_GRADIENT_FS));
        assert!(compiler.validate(builtin::RADIAL_GRADIENT_FS));
        assert!(compiler.validate(builtin::BLUR_CS));
        assert!(compiler.validate(builtin::BLIT_VS));
        assert!(compiler.validate(builtin::BLIT_FS));
    }
}
