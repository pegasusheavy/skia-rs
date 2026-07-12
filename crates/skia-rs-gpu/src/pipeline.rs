//! Pipeline state management for GPU rendering.
//!
//! This module provides abstractions for managing render and compute pipelines,
//! including pipeline caching and state tracking.

use crate::TextureFormat;

/// Vertex attribute format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    /// Single 32-bit float.
    Float32,
    /// 2x 32-bit floats.
    Float32x2,
    /// 3x 32-bit floats.
    Float32x3,
    /// 4x 32-bit floats.
    Float32x4,
    /// Single 32-bit signed integer.
    Sint32,
    /// 2x 32-bit signed integers.
    Sint32x2,
    /// 3x 32-bit signed integers.
    Sint32x3,
    /// 4x 32-bit signed integers.
    Sint32x4,
    /// Single 32-bit unsigned integer.
    Uint32,
    /// 2x 32-bit unsigned integers.
    Uint32x2,
    /// 3x 32-bit unsigned integers.
    Uint32x3,
    /// 4x 32-bit unsigned integers.
    Uint32x4,
    /// 4x 8-bit unsigned normalized.
    Unorm8x4,
}

impl VertexFormat {
    /// Get the size in bytes.
    pub fn size(&self) -> u32 {
        match self {
            Self::Float32 | Self::Sint32 | Self::Uint32 => 4,
            Self::Float32x2 | Self::Sint32x2 | Self::Uint32x2 => 8,
            Self::Float32x3 | Self::Sint32x3 | Self::Uint32x3 => 12,
            Self::Float32x4 | Self::Sint32x4 | Self::Uint32x4 | Self::Unorm8x4 => 16,
        }
    }
}

/// Vertex attribute descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexAttribute {
    /// Attribute location in shader.
    pub location: u32,
    /// Byte offset within vertex.
    pub offset: u32,
    /// Attribute format.
    pub format: VertexFormat,
}

/// Vertex buffer layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexBufferLayout {
    /// Stride in bytes between vertices.
    pub stride: u32,
    /// Step mode (vertex or instance).
    pub step_mode: VertexStepMode,
    /// Attributes.
    pub attributes: Vec<VertexAttribute>,
}

/// Vertex step mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VertexStepMode {
    /// Step per vertex.
    #[default]
    Vertex,
    /// Step per instance.
    Instance,
}

/// Primitive topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PrimitiveTopology {
    /// Point list.
    PointList,
    /// Line list.
    LineList,
    /// Line strip.
    LineStrip,
    /// Triangle list.
    #[default]
    TriangleList,
    /// Triangle strip.
    TriangleStrip,
}

/// Front face winding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FrontFace {
    /// Counter-clockwise is front.
    #[default]
    Ccw,
    /// Clockwise is front.
    Cw,
}

/// Cull mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CullMode {
    /// No culling.
    #[default]
    None,
    /// Cull front faces.
    Front,
    /// Cull back faces.
    Back,
}

/// Polygon mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PolygonMode {
    /// Filled polygons.
    #[default]
    Fill,
    /// Wireframe.
    Line,
    /// Points only.
    Point,
}

/// Primitive state configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PrimitiveState {
    /// Topology.
    pub topology: PrimitiveTopology,
    /// Front face.
    pub front_face: FrontFace,
    /// Cull mode.
    pub cull_mode: CullMode,
    /// Polygon mode.
    pub polygon_mode: PolygonMode,
    /// Strip index format for strip topologies.
    pub strip_index_format: Option<IndexFormat>,
}

/// Index format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexFormat {
    /// 16-bit indices.
    Uint16,
    /// 32-bit indices.
    Uint32,
}

/// Blend factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendFactor {
    /// 0.
    Zero,
    /// 1.
    #[default]
    One,
    /// Source color.
    Src,
    /// 1 - source color.
    OneMinusSrc,
    /// Source alpha.
    SrcAlpha,
    /// 1 - source alpha.
    OneMinusSrcAlpha,
    /// Destination color.
    Dst,
    /// 1 - destination color.
    OneMinusDst,
    /// Destination alpha.
    DstAlpha,
    /// 1 - destination alpha.
    OneMinusDstAlpha,
    /// Saturated source alpha.
    SrcAlphaSaturated,
    /// Constant color.
    Constant,
    /// 1 - constant color.
    OneMinusConstant,
}

/// Blend operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendOperation {
    /// Add source and destination.
    #[default]
    Add,
    /// Subtract destination from source.
    Subtract,
    /// Subtract source from destination.
    ReverseSubtract,
    /// Minimum of source and destination.
    Min,
    /// Maximum of source and destination.
    Max,
}

/// Blend component configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlendComponent {
    /// Source factor.
    pub src_factor: BlendFactor,
    /// Destination factor.
    pub dst_factor: BlendFactor,
    /// Operation.
    pub operation: BlendOperation,
}

impl Default for BlendComponent {
    fn default() -> Self {
        Self {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::Zero,
            operation: BlendOperation::Add,
        }
    }
}

impl BlendComponent {
    /// Standard alpha blending.
    pub const ALPHA_BLENDING: Self = Self {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    };

    /// Premultiplied alpha blending.
    pub const PREMULTIPLIED_ALPHA: Self = Self {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    };

    /// Additive blending.
    pub const ADDITIVE: Self = Self {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    };
}

/// Blend state configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlendState {
    /// Color blend component.
    pub color: BlendComponent,
    /// Alpha blend component.
    pub alpha: BlendComponent,
}

impl Default for BlendState {
    fn default() -> Self {
        Self {
            color: BlendComponent::default(),
            alpha: BlendComponent::default(),
        }
    }
}

impl BlendState {
    /// Standard alpha blending.
    pub const ALPHA_BLENDING: Self = Self {
        color: BlendComponent::ALPHA_BLENDING,
        alpha: BlendComponent::ALPHA_BLENDING,
    };

    /// Premultiplied alpha blending.
    pub const PREMULTIPLIED_ALPHA: Self = Self {
        color: BlendComponent::PREMULTIPLIED_ALPHA,
        alpha: BlendComponent::PREMULTIPLIED_ALPHA,
    };
}

/// Color write mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorWriteMask(u32);

impl ColorWriteMask {
    /// Write red.
    pub const RED: Self = Self(1);
    /// Write green.
    pub const GREEN: Self = Self(2);
    /// Write blue.
    pub const BLUE: Self = Self(4);
    /// Write alpha.
    pub const ALPHA: Self = Self(8);
    /// Write all components.
    pub const ALL: Self = Self(15);
    /// Write no components.
    pub const NONE: Self = Self(0);

    /// Check if a component is enabled.
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl Default for ColorWriteMask {
    fn default() -> Self {
        Self::ALL
    }
}

impl std::ops::BitOr for ColorWriteMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Color target state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColorTargetState {
    /// Format.
    pub format: TextureFormat,
    /// Blend state (None for no blending).
    pub blend: Option<BlendState>,
    /// Write mask.
    pub write_mask: ColorWriteMask,
}

impl ColorTargetState {
    /// Create with no blending.
    pub fn new(format: TextureFormat) -> Self {
        Self {
            format,
            blend: None,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Set blend state.
    pub fn with_blend(mut self, blend: BlendState) -> Self {
        self.blend = Some(blend);
        self
    }
}

/// Compare function for depth/stencil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompareFunction {
    /// Never pass.
    Never,
    /// Pass if less.
    Less,
    /// Pass if equal.
    Equal,
    /// Pass if less or equal.
    LessEqual,
    /// Pass if greater.
    Greater,
    /// Pass if not equal.
    NotEqual,
    /// Pass if greater or equal.
    GreaterEqual,
    /// Always pass.
    #[default]
    Always,
}

/// Stencil operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StencilOperation {
    /// Keep existing value.
    #[default]
    Keep,
    /// Set to zero.
    Zero,
    /// Replace with reference.
    Replace,
    /// Increment and clamp.
    IncrementClamp,
    /// Decrement and clamp.
    DecrementClamp,
    /// Bitwise invert.
    Invert,
    /// Increment and wrap.
    IncrementWrap,
    /// Decrement and wrap.
    DecrementWrap,
}

/// Stencil face state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StencilFaceState {
    /// Compare function.
    pub compare: CompareFunction,
    /// Operation on fail.
    pub fail_op: StencilOperation,
    /// Operation on depth fail.
    pub depth_fail_op: StencilOperation,
    /// Operation on pass.
    pub pass_op: StencilOperation,
}

/// Depth/stencil state.
#[derive(Debug, Clone, PartialEq)]
pub struct DepthStencilState {
    /// Depth/stencil format.
    pub format: TextureFormat,
    /// Enable depth writes.
    pub depth_write_enabled: bool,
    /// Depth compare function.
    pub depth_compare: CompareFunction,
    /// Stencil state for front faces.
    pub stencil_front: StencilFaceState,
    /// Stencil state for back faces.
    pub stencil_back: StencilFaceState,
    /// Stencil read mask.
    pub stencil_read_mask: u32,
    /// Stencil write mask.
    pub stencil_write_mask: u32,
    /// Depth bias.
    pub depth_bias: i32,
    /// Depth bias slope scale.
    pub depth_bias_slope_scale: f32,
    /// Depth bias clamp.
    pub depth_bias_clamp: f32,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            format: TextureFormat::Depth24Stencil8,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil_front: StencilFaceState::default(),
            stencil_back: StencilFaceState::default(),
            stencil_read_mask: 0xFF,
            stencil_write_mask: 0xFF,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }
    }
}

/// Multisample state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MultisampleState {
    /// Sample count.
    pub count: u32,
    /// Sample mask.
    pub mask: u64,
    /// Alpha to coverage enabled.
    pub alpha_to_coverage_enabled: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }
}

/// Render pipeline descriptor.
#[derive(Debug, Clone)]
pub struct RenderPipelineDescriptor {
    /// Label for debugging.
    pub label: Option<String>,
    /// Vertex shader source (WGSL).
    pub vertex_shader: String,
    /// Fragment shader source (WGSL).
    pub fragment_shader: String,
    /// Vertex entry point name.
    pub vertex_entry: String,
    /// Fragment entry point name.
    pub fragment_entry: String,
    /// Vertex buffer layouts.
    pub vertex_buffers: Vec<VertexBufferLayout>,
    /// Primitive state.
    pub primitive: PrimitiveState,
    /// Depth/stencil state.
    pub depth_stencil: Option<DepthStencilState>,
    /// Multisample state.
    pub multisample: MultisampleState,
    /// Color targets.
    pub color_targets: Vec<ColorTargetState>,
}

impl RenderPipelineDescriptor {
    /// Create a simple pipeline with vertex and fragment shaders.
    pub fn new(vertex_shader: &str, fragment_shader: &str) -> Self {
        Self {
            label: None,
            vertex_shader: vertex_shader.to_string(),
            fragment_shader: fragment_shader.to_string(),
            vertex_entry: "vs_main".to_string(),
            fragment_entry: "fs_main".to_string(),
            vertex_buffers: Vec::new(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            color_targets: Vec::new(),
        }
    }

    /// Set label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add a vertex buffer layout.
    pub fn with_vertex_buffer(mut self, layout: VertexBufferLayout) -> Self {
        self.vertex_buffers.push(layout);
        self
    }

    /// Add a color target.
    pub fn with_color_target(mut self, target: ColorTargetState) -> Self {
        self.color_targets.push(target);
        self
    }

    /// Set primitive state.
    pub fn with_primitive(mut self, primitive: PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    /// Set depth/stencil state.
    pub fn with_depth_stencil(mut self, state: DepthStencilState) -> Self {
        self.depth_stencil = Some(state);
        self
    }

    /// Set multisample state.
    pub fn with_multisample(mut self, state: MultisampleState) -> Self {
        self.multisample = state;
        self
    }
}

/// Compute pipeline descriptor.
#[derive(Debug, Clone)]
pub struct ComputePipelineDescriptor {
    /// Label for debugging.
    pub label: Option<String>,
    /// Compute shader source (WGSL).
    pub shader: String,
    /// Entry point name.
    pub entry_point: String,
}

impl ComputePipelineDescriptor {
    /// Create a new compute pipeline descriptor.
    pub fn new(shader: &str) -> Self {
        Self {
            label: None,
            shader: shader.to_string(),
            entry_point: "cs_main".to_string(),
        }
    }

    /// Set label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set entry point.
    pub fn with_entry_point(mut self, entry: impl Into<String>) -> Self {
        self.entry_point = entry.into();
        self
    }
}

/// Pipeline state cache key.
///
/// The key must distinguish any two descriptors that would compile into
/// *different* GPU pipelines. Two descriptors that share these fields are
/// guaranteed to produce interchangeable pipelines and may share a cache
/// slot. Missing a field here means two genuinely different pipelines
/// collide in the cache and the wrong one is used (e.g. a stencil-write
/// pass silently reusing a stencil-test pass's pipeline).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    /// Vertex shader hash.
    pub vertex_shader_hash: u64,
    /// Fragment shader hash.
    pub fragment_shader_hash: u64,
    /// Vertex/fragment entry-point name hash.
    pub entry_point_hash: u64,
    /// Vertex buffer layout hash (stride, step mode, attribute
    /// location/offset/format).
    pub vertex_layout_hash: u64,
    /// Primitive state hash (topology, front face, cull mode, polygon mode,
    /// strip index format).
    pub primitive_hash: u64,
    /// Color target formats.
    pub color_formats: Vec<TextureFormat>,
    /// Depth format.
    pub depth_format: Option<TextureFormat>,
    /// Depth/stencil state hash (depth compare/write, stencil front/back
    /// ops+compares, read/write masks, depth bias).
    pub depth_stencil_hash: u64,
    /// Sample count.
    pub sample_count: u32,
    /// Multisample-state hash (sample mask, alpha-to-coverage).
    pub multisample_hash: u64,
    /// Blend + write-mask hash (per-target color/alpha factors, operations,
    /// and color write mask).
    pub blend_hash: u64,
}

/// Hash a string for pipeline caching.
fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

impl PipelineKey {
    /// Create a key from a render pipeline descriptor.
    pub fn from_descriptor(desc: &RenderPipelineDescriptor) -> Self {
        use std::hash::{Hash, Hasher};

        let vertex_shader_hash = hash_str(&desc.vertex_shader);
        let fragment_shader_hash = hash_str(&desc.fragment_shader);

        let mut entry_hasher = std::collections::hash_map::DefaultHasher::new();
        desc.vertex_entry.hash(&mut entry_hasher);
        desc.fragment_entry.hash(&mut entry_hasher);
        let entry_point_hash = entry_hasher.finish();

        // Vertex layout: stride, step mode, and *every* attribute field —
        // location, offset, and format all change the compiled input state.
        let mut layout_hasher = std::collections::hash_map::DefaultHasher::new();
        for buf in &desc.vertex_buffers {
            buf.stride.hash(&mut layout_hasher);
            buf.step_mode.hash(&mut layout_hasher);
            for attr in &buf.attributes {
                attr.location.hash(&mut layout_hasher);
                attr.offset.hash(&mut layout_hasher);
                attr.format.hash(&mut layout_hasher);
            }
        }
        let vertex_layout_hash = layout_hasher.finish();

        // Primitive state.
        let mut prim_hasher = std::collections::hash_map::DefaultHasher::new();
        desc.primitive.topology.hash(&mut prim_hasher);
        desc.primitive.front_face.hash(&mut prim_hasher);
        desc.primitive.cull_mode.hash(&mut prim_hasher);
        desc.primitive.polygon_mode.hash(&mut prim_hasher);
        desc.primitive.strip_index_format.hash(&mut prim_hasher);
        let primitive_hash = prim_hasher.finish();

        // Depth/stencil: every field that reaches the pipeline. The f32 bias
        // fields are hashed via their bit patterns.
        let mut ds_hasher = std::collections::hash_map::DefaultHasher::new();
        if let Some(ds) = &desc.depth_stencil {
            ds.depth_write_enabled.hash(&mut ds_hasher);
            ds.depth_compare.hash(&mut ds_hasher);
            hash_stencil_face(&ds.stencil_front, &mut ds_hasher);
            hash_stencil_face(&ds.stencil_back, &mut ds_hasher);
            ds.stencil_read_mask.hash(&mut ds_hasher);
            ds.stencil_write_mask.hash(&mut ds_hasher);
            ds.depth_bias.hash(&mut ds_hasher);
            ds.depth_bias_slope_scale.to_bits().hash(&mut ds_hasher);
            ds.depth_bias_clamp.to_bits().hash(&mut ds_hasher);
        }
        let depth_stencil_hash = ds_hasher.finish();

        // Multisample fields beyond the count.
        let mut ms_hasher = std::collections::hash_map::DefaultHasher::new();
        desc.multisample.mask.hash(&mut ms_hasher);
        desc.multisample
            .alpha_to_coverage_enabled
            .hash(&mut ms_hasher);
        let multisample_hash = ms_hasher.finish();

        // Blend: both color *and* alpha components (factors + operation) plus
        // the color write mask, per target.
        let mut blend_hasher = std::collections::hash_map::DefaultHasher::new();
        for target in &desc.color_targets {
            match &target.blend {
                Some(blend) => {
                    1u8.hash(&mut blend_hasher); // discriminant: blending on
                    hash_blend_component(&blend.color, &mut blend_hasher);
                    hash_blend_component(&blend.alpha, &mut blend_hasher);
                }
                None => 0u8.hash(&mut blend_hasher), // discriminant: no blend
            }
            target.write_mask.hash(&mut blend_hasher);
        }
        let blend_hash = blend_hasher.finish();

        Self {
            vertex_shader_hash,
            fragment_shader_hash,
            entry_point_hash,
            vertex_layout_hash,
            primitive_hash,
            color_formats: desc.color_targets.iter().map(|t| t.format).collect(),
            depth_format: desc.depth_stencil.as_ref().map(|ds| ds.format),
            depth_stencil_hash,
            sample_count: desc.multisample.count,
            multisample_hash,
            blend_hash,
        }
    }
}

fn hash_stencil_face<H: std::hash::Hasher>(face: &StencilFaceState, hasher: &mut H) {
    use std::hash::Hash;
    face.compare.hash(hasher);
    face.fail_op.hash(hasher);
    face.depth_fail_op.hash(hasher);
    face.pass_op.hash(hasher);
}

fn hash_blend_component<H: std::hash::Hasher>(comp: &BlendComponent, hasher: &mut H) {
    use std::hash::Hash;
    comp.src_factor.hash(hasher);
    comp.dst_factor.hash(hasher);
    comp.operation.hash(hasher);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_format_size() {
        assert_eq!(VertexFormat::Float32.size(), 4);
        assert_eq!(VertexFormat::Float32x2.size(), 8);
        assert_eq!(VertexFormat::Float32x3.size(), 12);
        assert_eq!(VertexFormat::Float32x4.size(), 16);
    }

    #[test]
    fn test_blend_state() {
        let blend = BlendState::ALPHA_BLENDING;
        assert_eq!(blend.color.src_factor, BlendFactor::SrcAlpha);
        assert_eq!(blend.color.dst_factor, BlendFactor::OneMinusSrcAlpha);
    }

    #[test]
    fn test_color_write_mask() {
        let mask = ColorWriteMask::RED | ColorWriteMask::GREEN;
        assert!(mask.contains(ColorWriteMask::RED));
        assert!(mask.contains(ColorWriteMask::GREEN));
        assert!(!mask.contains(ColorWriteMask::BLUE));
    }

    #[test]
    fn test_pipeline_descriptor() {
        let desc = RenderPipelineDescriptor::new("vs", "fs")
            .with_label("test")
            .with_color_target(ColorTargetState::new(TextureFormat::Rgba8Unorm));

        assert_eq!(desc.label, Some("test".to_string()));
        assert_eq!(desc.color_targets.len(), 1);
    }

    #[test]
    fn test_pipeline_key() {
        let desc1 = RenderPipelineDescriptor::new("vs", "fs")
            .with_color_target(ColorTargetState::new(TextureFormat::Rgba8Unorm));
        let desc2 = RenderPipelineDescriptor::new("vs", "fs")
            .with_color_target(ColorTargetState::new(TextureFormat::Rgba8Unorm));
        let desc3 = RenderPipelineDescriptor::new("vs2", "fs")
            .with_color_target(ColorTargetState::new(TextureFormat::Rgba8Unorm));

        let key1 = PipelineKey::from_descriptor(&desc1);
        let key2 = PipelineKey::from_descriptor(&desc2);
        let key3 = PipelineKey::from_descriptor(&desc3);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    /// Regression for the PipelineKey coverage finding: the key must change
    /// whenever a field that affects the *compiled* pipeline changes.
    #[test]
    fn test_pipeline_key_covers_every_state_field() {
        // A rich base descriptor exercising all state.
        let base = || {
            let mut ds = DepthStencilState::default();
            ds.stencil_front = StencilFaceState {
                compare: CompareFunction::Equal,
                fail_op: StencilOperation::Keep,
                depth_fail_op: StencilOperation::Keep,
                pass_op: StencilOperation::Zero,
            };
            RenderPipelineDescriptor::new("vs", "fs")
                .with_vertex_buffer(VertexBufferLayout {
                    stride: 16,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![VertexAttribute {
                        location: 0,
                        offset: 0,
                        format: VertexFormat::Float32x2,
                    }],
                })
                .with_color_target(
                    ColorTargetState::new(TextureFormat::Rgba8Unorm)
                        .with_blend(BlendState::PREMULTIPLIED_ALPHA),
                )
                .with_depth_stencil(ds)
                .with_primitive(PrimitiveState::default())
        };

        let base_key = PipelineKey::from_descriptor(&base());
        assert_eq!(base_key, PipelineKey::from_descriptor(&base()));

        // Helper: mutate the descriptor and assert the key changes.
        let mut assert_changes = |mutate: &dyn Fn(&mut RenderPipelineDescriptor)| {
            let mut d = base();
            mutate(&mut d);
            assert_ne!(
                base_key,
                PipelineKey::from_descriptor(&d),
                "pipeline key failed to distinguish a state change"
            );
        };

        // Entry points.
        assert_changes(&|d| d.vertex_entry = "other".into());
        assert_changes(&|d| d.fragment_entry = "other".into());
        // Vertex attribute format (same location/offset).
        assert_changes(&|d| d.vertex_buffers[0].attributes[0].format = VertexFormat::Float32x4);
        assert_changes(&|d| d.vertex_buffers[0].step_mode = VertexStepMode::Instance);
        // Primitive state.
        assert_changes(&|d| d.primitive.topology = PrimitiveTopology::LineList);
        assert_changes(&|d| d.primitive.cull_mode = CullMode::Back);
        // Stencil ops / compares / masks.
        assert_changes(&|d| {
            d.depth_stencil.as_mut().unwrap().stencil_front.pass_op = StencilOperation::Replace
        });
        assert_changes(&|d| {
            d.depth_stencil.as_mut().unwrap().stencil_back.compare = CompareFunction::NotEqual
        });
        assert_changes(&|d| d.depth_stencil.as_mut().unwrap().stencil_write_mask = 0x0F);
        assert_changes(&|d| d.depth_stencil.as_mut().unwrap().depth_write_enabled = false);
        assert_changes(&|d| {
            d.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always
        });
        // Blend operation and alpha component (not just color src/dst).
        assert_changes(&|d| {
            d.color_targets[0].blend.as_mut().unwrap().color.operation =
                BlendOperation::ReverseSubtract
        });
        assert_changes(&|d| {
            d.color_targets[0].blend.as_mut().unwrap().alpha.dst_factor = BlendFactor::Zero
        });
        // Color write mask.
        assert_changes(&|d| d.color_targets[0].write_mask = ColorWriteMask::RED);
        // Multisample.
        assert_changes(&|d| d.multisample.alpha_to_coverage_enabled = true);
        assert_changes(&|d| d.multisample.count = 4);
    }
}
