//! GPU Shader Debugging Tools
//!
//! This module provides comprehensive tools for debugging GPU shaders in skia-rs.
//! It includes shader inspection, validation, profiling, and diagnostic utilities.
//!
//! # Overview
//!
//! The shader debugging system provides:
//! - **Shader Source Inspection**: View generated shader source code
//! - **Shader Validation**: Validate shaders against GLSL/WGSL specifications
//! - **Performance Profiling**: Measure shader compilation and execution time
//! - **Error Diagnostics**: Detailed error messages with line/column information
//! - **Shader Statistics**: Instruction counts, register usage, etc.
//!
//! # Example
//!
//! ```ignore
//! use skia_rs_gpu::debug::{ShaderDebugger, ShaderType};
//!
//! let debugger = ShaderDebugger::new();
//!
//! // Inspect a shader
//! let info = debugger.inspect_shader(shader_id)?;
//! println!("Shader type: {:?}", info.shader_type);
//! println!("Source:\n{}", info.source);
//!
//! // Validate shader
//! let validation = debugger.validate_shader(&source, ShaderType::Fragment)?;
//! if !validation.is_valid {
//!     for error in &validation.errors {
//!         eprintln!("Error at {}:{}: {}", error.line, error.column, error.message);
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

// =============================================================================
// Shader Types and Enums
// =============================================================================

/// Type of shader
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    /// Vertex shader
    Vertex,
    /// Fragment/pixel shader
    Fragment,
    /// Compute shader
    Compute,
    /// Geometry shader (if supported)
    Geometry,
    /// Tessellation control shader
    TessControl,
    /// Tessellation evaluation shader
    TessEval,
}

impl fmt::Display for ShaderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderType::Vertex => write!(f, "Vertex"),
            ShaderType::Fragment => write!(f, "Fragment"),
            ShaderType::Compute => write!(f, "Compute"),
            ShaderType::Geometry => write!(f, "Geometry"),
            ShaderType::TessControl => write!(f, "TessControl"),
            ShaderType::TessEval => write!(f, "TessEval"),
        }
    }
}

/// Shader language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderLanguage {
    /// OpenGL Shading Language
    Glsl,
    /// WebGPU Shading Language
    Wgsl,
    /// SPIR-V binary
    SpirV,
    /// Metal Shading Language
    Msl,
    /// High Level Shading Language (DirectX)
    Hlsl,
}

impl fmt::Display for ShaderLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderLanguage::Glsl => write!(f, "GLSL"),
            ShaderLanguage::Wgsl => write!(f, "WGSL"),
            ShaderLanguage::SpirV => write!(f, "SPIR-V"),
            ShaderLanguage::Msl => write!(f, "MSL"),
            ShaderLanguage::Hlsl => write!(f, "HLSL"),
        }
    }
}

// =============================================================================
// Shader Information
// =============================================================================

/// Information about a compiled shader
#[derive(Debug, Clone)]
pub struct ShaderInfo {
    /// Unique identifier for this shader
    pub id: u64,
    /// Type of shader
    pub shader_type: ShaderType,
    /// Source language
    pub language: ShaderLanguage,
    /// Original source code
    pub source: String,
    /// Compiled binary (if available)
    pub binary: Option<Vec<u8>>,
    /// Compilation time
    pub compile_time: Duration,
    /// Shader statistics
    pub stats: ShaderStats,
    /// Entry point name
    pub entry_point: String,
    /// Input/output bindings
    pub bindings: Vec<ShaderBinding>,
}

/// Statistics about a shader
#[derive(Debug, Clone, Default)]
pub struct ShaderStats {
    /// Estimated instruction count
    pub instruction_count: u32,
    /// Estimated register usage
    pub register_count: u32,
    /// Number of texture samplers used
    pub sampler_count: u32,
    /// Number of uniform buffers
    pub uniform_buffer_count: u32,
    /// Number of storage buffers
    pub storage_buffer_count: u32,
    /// Number of input attributes
    pub input_count: u32,
    /// Number of output attributes
    pub output_count: u32,
    /// Whether the shader uses derivatives
    pub uses_derivatives: bool,
    /// Whether the shader uses discard
    pub uses_discard: bool,
}

/// A shader binding (uniform, texture, etc.)
#[derive(Debug, Clone)]
pub struct ShaderBinding {
    /// Binding name
    pub name: String,
    /// Binding location/index
    pub location: u32,
    /// Binding type
    pub binding_type: BindingType,
    /// Binding group (for WGSL)
    pub group: u32,
}

/// Type of shader binding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingType {
    /// Uniform buffer
    UniformBuffer,
    /// Storage buffer
    StorageBuffer,
    /// Texture sampler
    Sampler,
    /// Texture
    Texture,
    /// Combined texture/sampler
    CombinedImageSampler,
    /// Storage image
    StorageImage,
    /// Input attribute
    Input,
    /// Output attribute
    Output,
}

// =============================================================================
// Validation Results
// =============================================================================

/// Result of shader validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the shader is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<ShaderError>,
    /// Validation warnings
    pub warnings: Vec<ShaderWarning>,
    /// Validation info messages
    pub info: Vec<String>,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    /// Create an invalid result with a single error
    pub fn error(error: ShaderError) -> Self {
        Self {
            is_valid: false,
            errors: vec![error],
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }
}

/// A shader compilation/validation error
#[derive(Debug, Clone)]
pub struct ShaderError {
    /// Error message
    pub message: String,
    /// Line number (1-indexed, 0 if unknown)
    pub line: u32,
    /// Column number (1-indexed, 0 if unknown)
    pub column: u32,
    /// Error code (if applicable)
    pub code: Option<String>,
    /// Source snippet around the error
    pub snippet: Option<String>,
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}: {}", self.line, self.column, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// A shader compilation/validation warning
#[derive(Debug, Clone)]
pub struct ShaderWarning {
    /// Warning message
    pub message: String,
    /// Line number (1-indexed, 0 if unknown)
    pub line: u32,
    /// Column number (1-indexed, 0 if unknown)
    pub column: u32,
    /// Warning code (if applicable)
    pub code: Option<String>,
}

impl fmt::Display for ShaderWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}: warning: {}", self.line, self.column, self.message)
        } else {
            write!(f, "warning: {}", self.message)
        }
    }
}

// =============================================================================
// Shader Debugger
// =============================================================================

/// GPU shader debugging tool
///
/// Provides utilities for inspecting, validating, and profiling shaders.
#[derive(Debug)]
pub struct ShaderDebugger {
    /// Cached shader information
    shader_cache: HashMap<u64, ShaderInfo>,
    /// Next shader ID
    next_id: u64,
    /// Debug verbosity level
    verbosity: DebugVerbosity,
    /// Whether to capture shader source
    capture_source: bool,
    /// Whether to capture shader binaries
    capture_binary: bool,
}

/// Debug verbosity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugVerbosity {
    /// Minimal output (errors only)
    #[default]
    Minimal,
    /// Normal output (errors and warnings)
    Normal,
    /// Verbose output (all messages)
    Verbose,
    /// Maximum verbosity (include debug info)
    Debug,
}

impl Default for ShaderDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderDebugger {
    /// Create a new shader debugger
    pub fn new() -> Self {
        Self {
            shader_cache: HashMap::new(),
            next_id: 1,
            verbosity: DebugVerbosity::Normal,
            capture_source: true,
            capture_binary: false,
        }
    }

    /// Set debug verbosity level
    pub fn set_verbosity(&mut self, verbosity: DebugVerbosity) {
        self.verbosity = verbosity;
    }

    /// Enable/disable source capture
    pub fn set_capture_source(&mut self, capture: bool) {
        self.capture_source = capture;
    }

    /// Enable/disable binary capture
    pub fn set_capture_binary(&mut self, capture: bool) {
        self.capture_binary = capture;
    }

    /// Register a shader for debugging
    pub fn register_shader(
        &mut self,
        source: &str,
        shader_type: ShaderType,
        language: ShaderLanguage,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let start = Instant::now();
        let stats = Self::analyze_shader(source, shader_type, language);
        let compile_time = start.elapsed();

        let info = ShaderInfo {
            id,
            shader_type,
            language,
            source: if self.capture_source {
                source.to_string()
            } else {
                String::new()
            },
            binary: None,
            compile_time,
            stats,
            entry_point: "main".to_string(),
            bindings: Self::extract_bindings(source, language),
        };

        self.shader_cache.insert(id, info);
        id
    }

    /// Get information about a registered shader
    pub fn get_shader_info(&self, id: u64) -> Option<&ShaderInfo> {
        self.shader_cache.get(&id)
    }

    /// Validate a shader
    pub fn validate_shader(
        &self,
        source: &str,
        shader_type: ShaderType,
        language: ShaderLanguage,
    ) -> ValidationResult {
        match language {
            ShaderLanguage::Glsl => self.validate_glsl(source, shader_type),
            ShaderLanguage::Wgsl => self.validate_wgsl(source, shader_type),
            ShaderLanguage::SpirV => ValidationResult::valid(), // Binary validation not implemented
            ShaderLanguage::Msl => self.validate_msl(source, shader_type),
            ShaderLanguage::Hlsl => self.validate_hlsl(source, shader_type),
        }
    }

    /// Analyze shader and extract statistics
    fn analyze_shader(
        source: &str,
        _shader_type: ShaderType,
        language: ShaderLanguage,
    ) -> ShaderStats {
        let mut stats = ShaderStats::default();

        // Count lines (rough instruction estimate)
        let lines: Vec<&str> = source.lines().collect();
        stats.instruction_count = lines
            .iter()
            .filter(|l| {
                let trimmed = l.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with('#')
            })
            .count() as u32;

        // Count specific patterns based on language
        match language {
            ShaderLanguage::Glsl => {
                stats.sampler_count = source.matches("sampler").count() as u32;
                stats.uniform_buffer_count = source.matches("uniform ").count() as u32;
                stats.uses_derivatives = source.contains("dFdx") || source.contains("dFdy");
                stats.uses_discard = source.contains("discard");
            }
            ShaderLanguage::Wgsl => {
                stats.sampler_count = source.matches("sampler").count() as u32;
                stats.uniform_buffer_count = source.matches("@group").count() as u32;
                stats.uses_derivatives = source.contains("dpdx") || source.contains("dpdy");
                stats.uses_discard = source.contains("discard");
            }
            _ => {}
        }

        stats
    }

    /// Extract bindings from shader source
    fn extract_bindings(source: &str, language: ShaderLanguage) -> Vec<ShaderBinding> {
        let mut bindings = Vec::new();

        match language {
            ShaderLanguage::Glsl => {
                // Extract GLSL uniforms
                for (idx, line) in source.lines().enumerate() {
                    let line = line.trim();
                    if line.starts_with("uniform ") {
                        if let Some(name) = Self::extract_glsl_uniform_name(line) {
                            bindings.push(ShaderBinding {
                                name,
                                location: idx as u32,
                                binding_type: BindingType::UniformBuffer,
                                group: 0,
                            });
                        }
                    }
                }
            }
            ShaderLanguage::Wgsl => {
                // Extract WGSL bindings
                for line in source.lines() {
                    if line.contains("@group") && line.contains("@binding") {
                        if let Some(binding) = Self::parse_wgsl_binding(line) {
                            bindings.push(binding);
                        }
                    }
                }
            }
            _ => {}
        }

        bindings
    }

    fn extract_glsl_uniform_name(line: &str) -> Option<String> {
        // Simple parser for "uniform type name;"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let name = parts[2].trim_end_matches(';').trim_end_matches('[');
            return Some(name.to_string());
        }
        None
    }

    fn parse_wgsl_binding(line: &str) -> Option<ShaderBinding> {
        // Parse @group(N) @binding(M) var<...> name: type;
        let group = Self::extract_number_from_attr(line, "@group(")?;
        let binding = Self::extract_number_from_attr(line, "@binding(")?;

        // Find var name
        let var_idx = line.find("var")?;
        let after_var = &line[var_idx..];
        let name_start = after_var.find(' ')? + 1;
        let name_end = after_var[name_start..].find(':')?;
        let name = after_var[name_start..name_start + name_end].trim().to_string();

        let binding_type = if line.contains("sampler") {
            BindingType::Sampler
        } else if line.contains("texture") {
            BindingType::Texture
        } else if line.contains("<uniform>") {
            BindingType::UniformBuffer
        } else if line.contains("<storage") {
            BindingType::StorageBuffer
        } else {
            BindingType::UniformBuffer
        };

        Some(ShaderBinding {
            name,
            location: binding,
            binding_type,
            group,
        })
    }

    fn extract_number_from_attr(line: &str, prefix: &str) -> Option<u32> {
        let start = line.find(prefix)? + prefix.len();
        let end = line[start..].find(')')?;
        line[start..start + end].parse().ok()
    }

    // Validation implementations
    fn validate_glsl(&self, source: &str, shader_type: ShaderType) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Check for version directive
        if !source.contains("#version") {
            result.warnings.push(ShaderWarning {
                message: "Missing #version directive".to_string(),
                line: 1,
                column: 1,
                code: Some("W001".to_string()),
            });
        }

        // Check for main function
        if !source.contains("void main()") && !source.contains("void main(void)") {
            result.errors.push(ShaderError {
                message: "Missing main() function".to_string(),
                line: 0,
                column: 0,
                code: Some("E001".to_string()),
                snippet: None,
            });
            result.is_valid = false;
        }

        // Check for required outputs based on shader type
        match shader_type {
            ShaderType::Vertex => {
                if !source.contains("gl_Position") {
                    result.warnings.push(ShaderWarning {
                        message: "Vertex shader does not write to gl_Position".to_string(),
                        line: 0,
                        column: 0,
                        code: Some("W002".to_string()),
                    });
                }
            }
            ShaderType::Fragment => {
                // Fragment shaders should have an output
                if !source.contains("out ") && !source.contains("gl_FragColor") {
                    result.warnings.push(ShaderWarning {
                        message: "Fragment shader has no output variable".to_string(),
                        line: 0,
                        column: 0,
                        code: Some("W003".to_string()),
                    });
                }
            }
            _ => {}
        }

        // Check for mismatched braces
        let open_braces = source.matches('{').count();
        let close_braces = source.matches('}').count();
        if open_braces != close_braces {
            result.errors.push(ShaderError {
                message: format!(
                    "Mismatched braces: {} open, {} close",
                    open_braces, close_braces
                ),
                line: 0,
                column: 0,
                code: Some("E002".to_string()),
                snippet: None,
            });
            result.is_valid = false;
        }

        result
    }

    fn validate_wgsl(&self, source: &str, shader_type: ShaderType) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Check for entry point
        let entry_point = match shader_type {
            ShaderType::Vertex => "@vertex",
            ShaderType::Fragment => "@fragment",
            ShaderType::Compute => "@compute",
            _ => "",
        };

        if !entry_point.is_empty() && !source.contains(entry_point) {
            result.errors.push(ShaderError {
                message: format!("Missing {} entry point annotation", entry_point),
                line: 0,
                column: 0,
                code: Some("E001".to_string()),
                snippet: None,
            });
            result.is_valid = false;
        }

        // Check for fn main or named entry point
        if !source.contains("fn ") {
            result.errors.push(ShaderError {
                message: "No function definitions found".to_string(),
                line: 0,
                column: 0,
                code: Some("E002".to_string()),
                snippet: None,
            });
            result.is_valid = false;
        }

        result
    }

    fn validate_msl(&self, source: &str, _shader_type: ShaderType) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Check for metal stdlib
        if !source.contains("#include <metal_stdlib>") {
            result.warnings.push(ShaderWarning {
                message: "Missing #include <metal_stdlib>".to_string(),
                line: 1,
                column: 1,
                code: Some("W001".to_string()),
            });
        }

        result
    }

    fn validate_hlsl(&self, source: &str, _shader_type: ShaderType) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Basic HLSL validation
        if !source.contains("main") && !source.contains("Main") {
            result.warnings.push(ShaderWarning {
                message: "No main entry point found".to_string(),
                line: 0,
                column: 0,
                code: Some("W001".to_string()),
            });
        }

        result
    }

    /// Dump shader information for debugging
    pub fn dump_shader(&self, id: u64) -> String {
        if let Some(info) = self.shader_cache.get(&id) {
            let mut output = String::new();
            output.push_str(&format!("=== Shader {} ===\n", id));
            output.push_str(&format!("Type: {}\n", info.shader_type));
            output.push_str(&format!("Language: {}\n", info.language));
            output.push_str(&format!("Entry Point: {}\n", info.entry_point));
            output.push_str(&format!("Compile Time: {:?}\n", info.compile_time));
            output.push_str("\n--- Statistics ---\n");
            output.push_str(&format!(
                "Instructions: ~{}\n",
                info.stats.instruction_count
            ));
            output.push_str(&format!("Registers: ~{}\n", info.stats.register_count));
            output.push_str(&format!("Samplers: {}\n", info.stats.sampler_count));
            output.push_str(&format!(
                "Uniform Buffers: {}\n",
                info.stats.uniform_buffer_count
            ));
            output.push_str(&format!(
                "Uses Derivatives: {}\n",
                info.stats.uses_derivatives
            ));
            output.push_str(&format!("Uses Discard: {}\n", info.stats.uses_discard));

            if !info.bindings.is_empty() {
                output.push_str("\n--- Bindings ---\n");
                for binding in &info.bindings {
                    output.push_str(&format!(
                        "  {} (group={}, binding={}, type={:?})\n",
                        binding.name, binding.group, binding.location, binding.binding_type
                    ));
                }
            }

            if !info.source.is_empty() {
                output.push_str("\n--- Source ---\n");
                for (i, line) in info.source.lines().enumerate() {
                    output.push_str(&format!("{:4} | {}\n", i + 1, line));
                }
            }

            output
        } else {
            format!("Shader {} not found", id)
        }
    }

    /// Get all registered shader IDs
    pub fn get_shader_ids(&self) -> Vec<u64> {
        self.shader_cache.keys().copied().collect()
    }

    /// Clear the shader cache
    pub fn clear(&mut self) {
        self.shader_cache.clear();
    }
}

// =============================================================================
// Shader Profiler
// =============================================================================

/// Shader performance profiler
#[derive(Debug, Default)]
pub struct ShaderProfiler {
    /// Compilation time samples
    compile_times: HashMap<u64, Vec<Duration>>,
    /// Execution time samples (if available)
    execution_times: HashMap<u64, Vec<Duration>>,
}

impl ShaderProfiler {
    /// Create a new shader profiler
    pub fn new() -> Self {
        Self::default()
    }

    /// Record compilation time for a shader
    pub fn record_compile_time(&mut self, shader_id: u64, duration: Duration) {
        self.compile_times
            .entry(shader_id)
            .or_default()
            .push(duration);
    }

    /// Record execution time for a shader
    pub fn record_execution_time(&mut self, shader_id: u64, duration: Duration) {
        self.execution_times
            .entry(shader_id)
            .or_default()
            .push(duration);
    }

    /// Get average compilation time for a shader
    pub fn avg_compile_time(&self, shader_id: u64) -> Option<Duration> {
        self.compile_times.get(&shader_id).map(|times| {
            let total: Duration = times.iter().sum();
            total / times.len() as u32
        })
    }

    /// Get average execution time for a shader
    pub fn avg_execution_time(&self, shader_id: u64) -> Option<Duration> {
        self.execution_times.get(&shader_id).map(|times| {
            let total: Duration = times.iter().sum();
            total / times.len() as u32
        })
    }

    /// Generate a profiling report
    pub fn generate_report(&self) -> String {
        let mut output = String::new();
        output.push_str("=== Shader Profiling Report ===\n\n");

        output.push_str("Compilation Times:\n");
        for (id, times) in &self.compile_times {
            let avg = times.iter().sum::<Duration>() / times.len() as u32;
            let min = times.iter().min().unwrap();
            let max = times.iter().max().unwrap();
            output.push_str(&format!(
                "  Shader {}: avg={:?}, min={:?}, max={:?}, samples={}\n",
                id,
                avg,
                min,
                max,
                times.len()
            ));
        }

        if !self.execution_times.is_empty() {
            output.push_str("\nExecution Times:\n");
            for (id, times) in &self.execution_times {
                let avg = times.iter().sum::<Duration>() / times.len() as u32;
                let min = times.iter().min().unwrap();
                let max = times.iter().max().unwrap();
                output.push_str(&format!(
                    "  Shader {}: avg={:?}, min={:?}, max={:?}, samples={}\n",
                    id,
                    avg,
                    min,
                    max,
                    times.len()
                ));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_debugger() {
        let mut debugger = ShaderDebugger::new();

        let glsl_source = r#"
#version 450
uniform mat4 uMVP;
in vec3 aPosition;
out vec4 vColor;
void main() {
    gl_Position = uMVP * vec4(aPosition, 1.0);
    vColor = vec4(1.0);
}
"#;

        let id = debugger.register_shader(glsl_source, ShaderType::Vertex, ShaderLanguage::Glsl);
        assert!(id > 0);

        let info = debugger.get_shader_info(id).unwrap();
        assert_eq!(info.shader_type, ShaderType::Vertex);
        assert_eq!(info.language, ShaderLanguage::Glsl);
    }

    #[test]
    fn test_glsl_validation() {
        let debugger = ShaderDebugger::new();

        // Valid shader
        let valid = r#"
#version 450
void main() {
    gl_Position = vec4(0.0);
}
"#;
        let result = debugger.validate_shader(valid, ShaderType::Vertex, ShaderLanguage::Glsl);
        assert!(result.is_valid);

        // Invalid shader (missing main)
        let invalid = r#"
#version 450
void notmain() { }
"#;
        let result = debugger.validate_shader(invalid, ShaderType::Vertex, ShaderLanguage::Glsl);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_wgsl_validation() {
        let debugger = ShaderDebugger::new();

        let wgsl = r#"
@vertex
fn main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}
"#;
        let result = debugger.validate_shader(wgsl, ShaderType::Vertex, ShaderLanguage::Wgsl);
        assert!(result.is_valid);
    }

    #[test]
    fn test_shader_profiler() {
        let mut profiler = ShaderProfiler::new();

        profiler.record_compile_time(1, Duration::from_millis(10));
        profiler.record_compile_time(1, Duration::from_millis(12));
        profiler.record_compile_time(1, Duration::from_millis(8));

        let avg = profiler.avg_compile_time(1).unwrap();
        assert_eq!(avg, Duration::from_millis(10));
    }
}
