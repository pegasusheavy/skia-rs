//! Runtime effects for custom shaders.
//!
//! This module provides Skia's runtime effects system, allowing custom
//! shaders written in SkSL to be compiled and used at runtime.

use crate::sksl::{Expr, FnDecl, Parser, SkslProgram, SkslType, Stmt};
use crate::shader::{Shader, ShaderKind};
use skia_rs_core::{Color4f, Matrix, Scalar};
use std::sync::Arc;

/// Error type for runtime effect operations.
#[derive(Debug, Clone)]
pub enum RuntimeEffectError {
    /// SkSL parsing error.
    ParseError(String),
    /// Compilation error.
    CompileError(String),
    /// Missing uniform.
    MissingUniform(String),
    /// Type mismatch.
    TypeMismatch(String),
    /// Invalid child count.
    InvalidChildCount { expected: usize, got: usize },
}

impl std::fmt::Display for RuntimeEffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeEffectError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RuntimeEffectError::CompileError(msg) => write!(f, "Compile error: {}", msg),
            RuntimeEffectError::MissingUniform(name) => write!(f, "Missing uniform: {}", name),
            RuntimeEffectError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            RuntimeEffectError::InvalidChildCount { expected, got } => {
                write!(f, "Invalid child count: expected {}, got {}", expected, got)
            }
        }
    }
}

impl std::error::Error for RuntimeEffectError {}

/// Uniform metadata.
#[derive(Debug, Clone)]
pub struct Uniform {
    /// Uniform name.
    pub name: String,
    /// Uniform type.
    pub ty: UniformType,
    /// Byte offset in uniform data.
    pub offset: usize,
    /// Array count (1 for non-arrays).
    pub count: usize,
}

/// Uniform types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniformType {
    /// Float scalar.
    Float,
    /// 2-component float vector.
    Float2,
    /// 3-component float vector.
    Float3,
    /// 4-component float vector.
    Float4,
    /// 2x2 float matrix.
    Float2x2,
    /// 3x3 float matrix.
    Float3x3,
    /// 4x4 float matrix.
    Float4x4,
    /// Integer scalar.
    Int,
    /// 2-component integer vector.
    Int2,
    /// 3-component integer vector.
    Int3,
    /// 4-component integer vector.
    Int4,
}

impl UniformType {
    /// Get the size in bytes.
    pub fn size_bytes(&self) -> usize {
        match self {
            UniformType::Float => 4,
            UniformType::Float2 => 8,
            UniformType::Float3 => 12,
            UniformType::Float4 => 16,
            UniformType::Float2x2 => 16,
            UniformType::Float3x3 => 36,
            UniformType::Float4x4 => 64,
            UniformType::Int => 4,
            UniformType::Int2 => 8,
            UniformType::Int3 => 12,
            UniformType::Int4 => 16,
        }
    }

    /// Get the number of floats/ints.
    pub fn slot_count(&self) -> usize {
        match self {
            UniformType::Float | UniformType::Int => 1,
            UniformType::Float2 | UniformType::Int2 => 2,
            UniformType::Float3 | UniformType::Int3 => 3,
            UniformType::Float4 | UniformType::Int4 => 4,
            UniformType::Float2x2 => 4,
            UniformType::Float3x3 => 9,
            UniformType::Float4x4 => 16,
        }
    }

    /// Check if this is a float type.
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            UniformType::Float
                | UniformType::Float2
                | UniformType::Float3
                | UniformType::Float4
                | UniformType::Float2x2
                | UniformType::Float3x3
                | UniformType::Float4x4
        )
    }
}

impl From<&SkslType> for UniformType {
    fn from(ty: &SkslType) -> Self {
        match ty {
            SkslType::Float | SkslType::Half => UniformType::Float,
            SkslType::Vec2 | SkslType::Half2 => UniformType::Float2,
            SkslType::Vec3 | SkslType::Half3 => UniformType::Float3,
            SkslType::Vec4 | SkslType::Half4 => UniformType::Float4,
            SkslType::Mat2 => UniformType::Float2x2,
            SkslType::Mat3 => UniformType::Float3x3,
            SkslType::Mat4 => UniformType::Float4x4,
            SkslType::Int => UniformType::Int,
            _ => UniformType::Float,
        }
    }
}

/// Child shader/color filter metadata.
#[derive(Debug, Clone)]
pub struct Child {
    /// Child name.
    pub name: String,
    /// Child type.
    pub ty: ChildType,
    /// Index in children array.
    pub index: usize,
}

/// Child types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildType {
    /// Shader child.
    Shader,
    /// Color filter child.
    ColorFilter,
    /// Blender child.
    Blender,
}

/// Target language for shader compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderTarget {
    /// GLSL ES 3.0.
    GlslEs300,
    /// GLSL 4.5.
    Glsl450,
    /// SPIR-V.
    SpirV,
    /// Metal Shading Language.
    Msl,
    /// WebGPU Shading Language.
    Wgsl,
}

/// A compiled runtime effect.
#[derive(Debug, Clone)]
pub struct RuntimeEffect {
    /// Original SkSL source.
    source: String,
    /// Parsed program.
    program: SkslProgram,
    /// Uniforms.
    uniforms: Vec<Uniform>,
    /// Children (shader/colorFilter).
    children: Vec<Child>,
    /// Total uniform data size.
    uniform_size: usize,
    /// Compiled GLSL (cached).
    glsl_cache: Option<String>,
    /// Compiled WGSL (cached).
    wgsl_cache: Option<String>,
}

impl RuntimeEffect {
    /// Create a runtime effect from SkSL source for shaders.
    pub fn make_for_shader(source: &str) -> Result<Self, RuntimeEffectError> {
        Self::compile(source, EffectKind::Shader)
    }

    /// Create a runtime effect from SkSL source for color filters.
    pub fn make_for_color_filter(source: &str) -> Result<Self, RuntimeEffectError> {
        Self::compile(source, EffectKind::ColorFilter)
    }

    /// Create a runtime effect from SkSL source for blenders.
    pub fn make_for_blender(source: &str) -> Result<Self, RuntimeEffectError> {
        Self::compile(source, EffectKind::Blender)
    }

    fn compile(source: &str, _kind: EffectKind) -> Result<Self, RuntimeEffectError> {
        let mut parser = Parser::new(source);
        let program = parser
            .parse_program()
            .map_err(RuntimeEffectError::ParseError)?;

        // Extract uniforms
        let mut uniforms = Vec::new();
        let mut offset = 0;

        for uniform in &program.uniforms {
            let ty = UniformType::from(&uniform.ty);
            let count = uniform.array_size.unwrap_or(1);
            let size = ty.size_bytes() * count;

            // Align offset
            let alignment = ty.size_bytes().min(16);
            offset = (offset + alignment - 1) / alignment * alignment;

            uniforms.push(Uniform {
                name: uniform.name.clone(),
                ty,
                offset,
                count,
            });

            offset += size;
        }

        // Extract children
        let mut children = Vec::new();
        let mut child_index = 0;

        for uniform in &program.uniforms {
            let child_type = match &uniform.ty {
                SkslType::Shader => Some(ChildType::Shader),
                SkslType::ColorFilter => Some(ChildType::ColorFilter),
                SkslType::Blender => Some(ChildType::Blender),
                _ => None,
            };

            if let Some(ty) = child_type {
                children.push(Child {
                    name: uniform.name.clone(),
                    ty,
                    index: child_index,
                });
                child_index += 1;
            }
        }

        Ok(Self {
            source: source.to_string(),
            program,
            uniforms,
            children,
            uniform_size: offset,
            glsl_cache: None,
            wgsl_cache: None,
        })
    }

    /// Get the uniforms.
    pub fn uniforms(&self) -> &[Uniform] {
        &self.uniforms
    }

    /// Get the children.
    pub fn children(&self) -> &[Child] {
        &self.children
    }

    /// Get the uniform data size in bytes.
    pub fn uniform_size(&self) -> usize {
        self.uniform_size
    }

    /// Find a uniform by name.
    pub fn find_uniform(&self, name: &str) -> Option<&Uniform> {
        self.uniforms.iter().find(|u| u.name == name)
    }

    /// Find a child by name.
    pub fn find_child(&self, name: &str) -> Option<&Child> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Compile to target language.
    pub fn compile_to(&self, target: ShaderTarget) -> Result<String, RuntimeEffectError> {
        match target {
            ShaderTarget::GlslEs300 => Ok(self.to_glsl(true)),
            ShaderTarget::Glsl450 => Ok(self.to_glsl(false)),
            ShaderTarget::Wgsl => Ok(self.to_wgsl()),
            ShaderTarget::Msl => Ok(self.to_msl()),
            ShaderTarget::SpirV => Err(RuntimeEffectError::CompileError(
                "SPIR-V compilation not yet implemented".to_string(),
            )),
        }
    }

    /// Convert to GLSL.
    fn to_glsl(&self, es: bool) -> String {
        let mut output = String::new();

        // Version
        if es {
            output.push_str("#version 300 es\n");
            output.push_str("precision highp float;\n");
        } else {
            output.push_str("#version 450\n");
        }
        output.push('\n');

        // Uniforms
        for uniform in &self.uniforms {
            output.push_str(&format!(
                "uniform {} {};\n",
                self.type_to_glsl(&uniform.ty),
                uniform.name
            ));
        }
        if !self.uniforms.is_empty() {
            output.push('\n');
        }

        // Functions
        for func in &self.program.functions {
            output.push_str(&self.function_to_glsl(func));
            output.push('\n');
        }

        output
    }

    fn type_to_glsl(&self, ty: &UniformType) -> &'static str {
        match ty {
            UniformType::Float => "float",
            UniformType::Float2 => "vec2",
            UniformType::Float3 => "vec3",
            UniformType::Float4 => "vec4",
            UniformType::Float2x2 => "mat2",
            UniformType::Float3x3 => "mat3",
            UniformType::Float4x4 => "mat4",
            UniformType::Int => "int",
            UniformType::Int2 => "ivec2",
            UniformType::Int3 => "ivec3",
            UniformType::Int4 => "ivec4",
        }
    }

    fn function_to_glsl(&self, func: &FnDecl) -> String {
        let mut output = String::new();

        // Return type and name
        output.push_str(func.return_type.glsl_name());
        output.push(' ');
        output.push_str(&func.name);
        output.push('(');

        // Parameters
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            output.push_str(param.ty.glsl_name());
            output.push(' ');
            output.push_str(&param.name);
        }

        output.push_str(") ");
        output.push_str(&self.stmt_to_glsl(&func.body, 0));

        output
    }

    fn stmt_to_glsl(&self, stmt: &Stmt, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        match stmt {
            Stmt::Expr(expr) => format!("{}{};\n", ind, self.expr_to_glsl(expr)),
            Stmt::VarDecl { ty, name, init } => {
                if let Some(init) = init {
                    format!(
                        "{}{} {} = {};\n",
                        ind,
                        ty.glsl_name(),
                        name,
                        self.expr_to_glsl(init)
                    )
                } else {
                    format!("{}{} {};\n", ind, ty.glsl_name(), name)
                }
            }
            Stmt::Block(stmts) => {
                let mut output = format!("{}{{\n", ind);
                for s in stmts {
                    output.push_str(&self.stmt_to_glsl(s, indent + 1));
                }
                output.push_str(&format!("{}}}\n", ind));
                output
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let mut output = format!("{}if ({}) ", ind, self.expr_to_glsl(cond));
                output.push_str(&self.stmt_to_glsl(then_branch, indent));
                if let Some(else_b) = else_branch {
                    output.push_str(&format!("{}else ", ind));
                    output.push_str(&self.stmt_to_glsl(else_b, indent));
                }
                output
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
            } => {
                let mut output = format!("{}for (", ind);
                if let Some(init) = init {
                    let init_str = self.stmt_to_glsl(init, 0);
                    output.push_str(init_str.trim());
                } else {
                    output.push(';');
                }
                output.push(' ');
                if let Some(cond) = cond {
                    output.push_str(&self.expr_to_glsl(cond));
                }
                output.push_str("; ");
                if let Some(update) = update {
                    output.push_str(&self.expr_to_glsl(update));
                }
                output.push_str(") ");
                output.push_str(&self.stmt_to_glsl(body, indent));
                output
            }
            Stmt::While { cond, body } => {
                let mut output = format!("{}while ({}) ", ind, self.expr_to_glsl(cond));
                output.push_str(&self.stmt_to_glsl(body, indent));
                output
            }
            Stmt::DoWhile { body, cond } => {
                let mut output = format!("{}do ", ind);
                output.push_str(&self.stmt_to_glsl(body, indent));
                output.push_str(&format!(" while ({});\n", self.expr_to_glsl(cond)));
                output
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    format!("{}return {};\n", ind, self.expr_to_glsl(expr))
                } else {
                    format!("{}return;\n", ind)
                }
            }
            Stmt::Break => format!("{}break;\n", ind),
            Stmt::Continue => format!("{}continue;\n", ind),
            Stmt::Discard => format!("{}discard;\n", ind),
        }
    }

    fn expr_to_glsl(&self, expr: &Expr) -> String {
        match expr {
            Expr::IntLit(n) => n.to_string(),
            Expr::FloatLit(n) => {
                if n.fract() == 0.0 {
                    format!("{}.0", n)
                } else {
                    format!("{}", n)
                }
            }
            Expr::BoolLit(b) => b.to_string(),
            Expr::Var(name) => name.clone(),
            Expr::Binary { left, op, right } => {
                format!(
                    "({} {} {})",
                    self.expr_to_glsl(left),
                    op.glsl_str(),
                    self.expr_to_glsl(right)
                )
            }
            Expr::Unary { op, expr } => {
                format!("({}{})", op.glsl_str(), self.expr_to_glsl(expr))
            }
            Expr::Call { name, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.expr_to_glsl(a)).collect();
                format!("{}({})", name, args_str.join(", "))
            }
            Expr::Constructor { ty, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.expr_to_glsl(a)).collect();
                format!("{}({})", ty.glsl_name(), args_str.join(", "))
            }
            Expr::Field { expr, field } => {
                format!("{}.{}", self.expr_to_glsl(expr), field)
            }
            Expr::Index { expr, index } => {
                format!("{}[{}]", self.expr_to_glsl(expr), self.expr_to_glsl(index))
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                format!(
                    "({} ? {} : {})",
                    self.expr_to_glsl(cond),
                    self.expr_to_glsl(then_expr),
                    self.expr_to_glsl(else_expr)
                )
            }
            Expr::Assign { target, value } => {
                format!("({} = {})", self.expr_to_glsl(target), self.expr_to_glsl(value))
            }
            Expr::CompoundAssign { target, op, value } => {
                format!(
                    "({} {}= {})",
                    self.expr_to_glsl(target),
                    op.glsl_str(),
                    self.expr_to_glsl(value)
                )
            }
            Expr::PostIncDec { expr, inc } => {
                format!("{}{}",self.expr_to_glsl(expr), if *inc { "++" } else { "--" })
            }
            Expr::PreIncDec { expr, inc } => {
                format!("{}{}", if *inc { "++" } else { "--" }, self.expr_to_glsl(expr))
            }
        }
    }

    /// Convert to WGSL.
    fn to_wgsl(&self) -> String {
        let mut output = String::new();

        // Uniforms as struct
        if !self.uniforms.is_empty() {
            output.push_str("struct Uniforms {\n");
            for uniform in &self.uniforms {
                output.push_str(&format!(
                    "    {}: {},\n",
                    uniform.name,
                    self.type_to_wgsl(&uniform.ty)
                ));
            }
            output.push_str("};\n\n");
            output.push_str("@group(0) @binding(0) var<uniform> uniforms: Uniforms;\n\n");
        }

        // Functions
        for func in &self.program.functions {
            output.push_str(&self.function_to_wgsl(func));
            output.push('\n');
        }

        output
    }

    fn type_to_wgsl(&self, ty: &UniformType) -> &'static str {
        match ty {
            UniformType::Float => "f32",
            UniformType::Float2 => "vec2<f32>",
            UniformType::Float3 => "vec3<f32>",
            UniformType::Float4 => "vec4<f32>",
            UniformType::Float2x2 => "mat2x2<f32>",
            UniformType::Float3x3 => "mat3x3<f32>",
            UniformType::Float4x4 => "mat4x4<f32>",
            UniformType::Int => "i32",
            UniformType::Int2 => "vec2<i32>",
            UniformType::Int3 => "vec3<i32>",
            UniformType::Int4 => "vec4<i32>",
        }
    }

    fn function_to_wgsl(&self, func: &FnDecl) -> String {
        let mut output = String::new();

        output.push_str("fn ");
        output.push_str(&func.name);
        output.push('(');

        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            output.push_str(&param.name);
            output.push_str(": ");
            output.push_str(param.ty.wgsl_name());
        }

        output.push_str(") -> ");
        output.push_str(func.return_type.wgsl_name());
        output.push_str(" {\n");

        // Body (simplified)
        if let Stmt::Block(stmts) = &func.body {
            for stmt in stmts {
                output.push_str(&self.stmt_to_wgsl(stmt, 1));
            }
        }

        output.push_str("}\n");
        output
    }

    fn stmt_to_wgsl(&self, stmt: &Stmt, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        match stmt {
            Stmt::Return(Some(expr)) => {
                format!("{}return {};\n", ind, self.expr_to_wgsl(expr))
            }
            Stmt::Return(None) => format!("{}return;\n", ind),
            Stmt::VarDecl { ty, name, init } => {
                if let Some(init) = init {
                    format!(
                        "{}var {}: {} = {};\n",
                        ind,
                        name,
                        ty.wgsl_name(),
                        self.expr_to_wgsl(init)
                    )
                } else {
                    format!("{}var {}: {};\n", ind, name, ty.wgsl_name())
                }
            }
            Stmt::Expr(expr) => format!("{}{};\n", ind, self.expr_to_wgsl(expr)),
            _ => format!("{}// Unsupported statement\n", ind),
        }
    }

    fn expr_to_wgsl(&self, expr: &Expr) -> String {
        match expr {
            Expr::IntLit(n) => format!("{}i", n),
            Expr::FloatLit(n) => {
                if n.fract() == 0.0 {
                    format!("{}.0", n)
                } else {
                    format!("{}", n)
                }
            }
            Expr::BoolLit(b) => b.to_string(),
            Expr::Var(name) => name.clone(),
            Expr::Binary { left, op, right } => {
                format!(
                    "({} {} {})",
                    self.expr_to_wgsl(left),
                    op.glsl_str(),
                    self.expr_to_wgsl(right)
                )
            }
            Expr::Constructor { ty, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.expr_to_wgsl(a)).collect();
                format!("{}({})", ty.wgsl_name(), args_str.join(", "))
            }
            Expr::Call { name, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.expr_to_wgsl(a)).collect();
                format!("{}({})", name, args_str.join(", "))
            }
            Expr::Field { expr, field } => {
                format!("{}.{}", self.expr_to_wgsl(expr), field)
            }
            _ => "/* unsupported */".to_string(),
        }
    }

    /// Convert to MSL.
    fn to_msl(&self) -> String {
        let mut output = String::new();

        output.push_str("#include <metal_stdlib>\n");
        output.push_str("using namespace metal;\n\n");

        // Uniforms as struct
        if !self.uniforms.is_empty() {
            output.push_str("struct Uniforms {\n");
            for uniform in &self.uniforms {
                output.push_str(&format!(
                    "    {} {};\n",
                    self.type_to_msl(&uniform.ty),
                    uniform.name
                ));
            }
            output.push_str("};\n\n");
        }

        // Functions
        for func in &self.program.functions {
            output.push_str(&self.function_to_msl(func));
            output.push('\n');
        }

        output
    }

    fn type_to_msl(&self, ty: &UniformType) -> &'static str {
        match ty {
            UniformType::Float => "float",
            UniformType::Float2 => "float2",
            UniformType::Float3 => "float3",
            UniformType::Float4 => "float4",
            UniformType::Float2x2 => "float2x2",
            UniformType::Float3x3 => "float3x3",
            UniformType::Float4x4 => "float4x4",
            UniformType::Int => "int",
            UniformType::Int2 => "int2",
            UniformType::Int3 => "int3",
            UniformType::Int4 => "int4",
        }
    }

    fn function_to_msl(&self, func: &FnDecl) -> String {
        let mut output = String::new();

        // Use Metal types
        let ret_type = match &func.return_type {
            SkslType::Vec4 | SkslType::Half4 => "float4",
            SkslType::Vec3 | SkslType::Half3 => "float3",
            SkslType::Vec2 | SkslType::Half2 => "float2",
            SkslType::Float | SkslType::Half => "float",
            SkslType::Int => "int",
            SkslType::Bool => "bool",
            SkslType::Void => "void",
            _ => "float4",
        };

        output.push_str(ret_type);
        output.push(' ');
        output.push_str(&func.name);
        output.push('(');

        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            let param_type = match &param.ty {
                SkslType::Vec4 | SkslType::Half4 => "float4",
                SkslType::Vec3 | SkslType::Half3 => "float3",
                SkslType::Vec2 | SkslType::Half2 => "float2",
                SkslType::Float | SkslType::Half => "float",
                _ => "float",
            };
            output.push_str(param_type);
            output.push(' ');
            output.push_str(&param.name);
        }

        output.push_str(") ");
        output.push_str(&self.stmt_to_glsl(&func.body, 0)); // Reuse GLSL for simplicity

        output
    }

    /// Create a RuntimeShader from this effect.
    pub fn make_shader(
        self: &Arc<Self>,
        uniforms: &UniformData,
        children: &[Arc<dyn Shader>],
    ) -> Result<RuntimeShader, RuntimeEffectError> {
        if children.len() != self.children.len() {
            return Err(RuntimeEffectError::InvalidChildCount {
                expected: self.children.len(),
                got: children.len(),
            });
        }

        Ok(RuntimeShader {
            effect: Arc::clone(self),
            uniforms: uniforms.clone(),
            children: children.to_vec(),
        })
    }

    /// Create a RuntimeColorFilter from this effect.
    pub fn make_color_filter(
        self: &Arc<Self>,
        uniforms: &UniformData,
    ) -> Result<RuntimeColorFilter, RuntimeEffectError> {
        Ok(RuntimeColorFilter {
            effect: Arc::clone(self),
            uniforms: uniforms.clone(),
        })
    }
}

/// Effect kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectKind {
    Shader,
    ColorFilter,
    Blender,
}

/// Uniform data builder.
#[derive(Debug, Clone, Default)]
pub struct UniformData {
    data: Vec<u8>,
}

impl UniformData {
    /// Create new uniform data with specified size.
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
        }
    }

    /// Create from a runtime effect.
    pub fn from_effect(effect: &RuntimeEffect) -> Self {
        Self::new(effect.uniform_size())
    }

    /// Set a float uniform.
    pub fn set_float(&mut self, offset: usize, value: f32) {
        if offset + 4 <= self.data.len() {
            self.data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    /// Set a vec2 uniform.
    pub fn set_float2(&mut self, offset: usize, x: f32, y: f32) {
        self.set_float(offset, x);
        self.set_float(offset + 4, y);
    }

    /// Set a vec3 uniform.
    pub fn set_float3(&mut self, offset: usize, x: f32, y: f32, z: f32) {
        self.set_float(offset, x);
        self.set_float(offset + 4, y);
        self.set_float(offset + 8, z);
    }

    /// Set a vec4 uniform.
    pub fn set_float4(&mut self, offset: usize, x: f32, y: f32, z: f32, w: f32) {
        self.set_float(offset, x);
        self.set_float(offset + 4, y);
        self.set_float(offset + 8, z);
        self.set_float(offset + 12, w);
    }

    /// Set a color uniform.
    pub fn set_color(&mut self, offset: usize, color: Color4f) {
        self.set_float4(offset, color.r, color.g, color.b, color.a);
    }

    /// Set an int uniform.
    pub fn set_int(&mut self, offset: usize, value: i32) {
        if offset + 4 <= self.data.len() {
            self.data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    /// Get a float uniform.
    pub fn get_float(&self, offset: usize) -> f32 {
        if offset + 4 <= self.data.len() {
            f32::from_le_bytes(self.data[offset..offset + 4].try_into().unwrap())
        } else {
            0.0
        }
    }

    /// Get the raw data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// A runtime shader created from a RuntimeEffect.
#[derive(Debug, Clone)]
pub struct RuntimeShader {
    effect: Arc<RuntimeEffect>,
    uniforms: UniformData,
    children: Vec<Arc<dyn Shader>>,
}

impl RuntimeShader {
    /// Get the effect.
    pub fn effect(&self) -> &RuntimeEffect {
        &self.effect
    }

    /// Get the uniforms.
    pub fn uniforms(&self) -> &UniformData {
        &self.uniforms
    }

    /// Get the children.
    pub fn children(&self) -> &[Arc<dyn Shader>] {
        &self.children
    }
}

impl Shader for RuntimeShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        None
    }

    fn is_opaque(&self) -> bool {
        false
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::Color // Closest match for runtime shader
    }

    fn sample(&self, _x: Scalar, _y: Scalar) -> Color4f {
        // Software fallback - would need interpreter
        // For now, return magenta to indicate runtime shader
        Color4f::new(1.0, 0.0, 1.0, 1.0)
    }
}

/// A runtime color filter created from a RuntimeEffect.
#[derive(Debug, Clone)]
pub struct RuntimeColorFilter {
    effect: Arc<RuntimeEffect>,
    uniforms: UniformData,
}

impl RuntimeColorFilter {
    /// Get the effect.
    pub fn effect(&self) -> &RuntimeEffect {
        &self.effect
    }

    /// Get the uniforms.
    pub fn uniforms(&self) -> &UniformData {
        &self.uniforms
    }

    /// Filter a color.
    pub fn filter_color(&self, color: Color4f) -> Color4f {
        // Software fallback - would need interpreter
        color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_SHADER: &str = r#"
        uniform float time;
        uniform vec2 resolution;

        vec4 main(vec2 fragCoord) {
            vec2 uv = fragCoord / resolution;
            return vec4(uv.x, uv.y, sin(time), 1.0);
        }
    "#;

    #[test]
    fn test_make_effect() {
        let effect = RuntimeEffect::make_for_shader(SIMPLE_SHADER).unwrap();

        assert_eq!(effect.uniforms().len(), 2);
        assert!(effect.find_uniform("time").is_some());
        assert!(effect.find_uniform("resolution").is_some());
    }

    #[test]
    fn test_uniform_data() {
        let effect = RuntimeEffect::make_for_shader(SIMPLE_SHADER).unwrap();
        let mut data = UniformData::from_effect(&effect);

        let time_uniform = effect.find_uniform("time").unwrap();
        data.set_float(time_uniform.offset, 1.5);
        assert!((data.get_float(time_uniform.offset) - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_compile_glsl() {
        let effect = RuntimeEffect::make_for_shader(SIMPLE_SHADER).unwrap();
        let glsl = effect.compile_to(ShaderTarget::GlslEs300).unwrap();

        assert!(glsl.contains("#version 300 es"));
        assert!(glsl.contains("uniform float time"));
        assert!(glsl.contains("uniform vec2 resolution"));
    }

    #[test]
    fn test_compile_wgsl() {
        let effect = RuntimeEffect::make_for_shader(SIMPLE_SHADER).unwrap();
        let wgsl = effect.compile_to(ShaderTarget::Wgsl).unwrap();

        assert!(wgsl.contains("struct Uniforms"));
        assert!(wgsl.contains("time: f32"));
    }

    #[test]
    fn test_compile_msl() {
        let effect = RuntimeEffect::make_for_shader(SIMPLE_SHADER).unwrap();
        let msl = effect.compile_to(ShaderTarget::Msl).unwrap();

        assert!(msl.contains("#include <metal_stdlib>"));
        assert!(msl.contains("struct Uniforms"));
    }

    #[test]
    fn test_make_shader() {
        let effect = Arc::new(RuntimeEffect::make_for_shader(SIMPLE_SHADER).unwrap());
        let mut data = UniformData::from_effect(&effect);

        data.set_float(0, 1.0);
        data.set_float2(4, 800.0, 600.0);

        let shader = effect.make_shader(&data, &[]).unwrap();
        assert!(shader.effect().uniforms().len() == 2);
    }
}
