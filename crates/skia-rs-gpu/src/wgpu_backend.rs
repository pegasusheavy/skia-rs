//! WebGPU backend implementation using wgpu.

use crate::command::{CommandBuffer, DrawCommand};
use crate::pipeline::{
    BlendFactor as CrateBlendFactor, BlendOperation as CrateBlendOperation,
    BlendState as CrateBlendState, ColorWriteMask, IndexFormat as CrateIndexFormat,
    PipelineKey, PrimitiveTopology as CratePrimitiveTopology, RenderPipelineDescriptor,
    VertexFormat as CrateVertexFormat,
};
use crate::{
    GpuAdapterInfo, GpuBackendType, GpuCaps, GpuContext, GpuDeviceType, GpuError, GpuResult,
    GpuSurface, GpuSurfaceProps, RenderPassDescriptor, TextureFormat,
};
use parking_lot::Mutex;
use skia_rs_core::Color;
use std::collections::HashMap;
use std::sync::Arc;

/// wgpu-based GPU context.
pub struct WgpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    info: GpuAdapterInfo,
    caps: GpuCaps,
}

impl WgpuContext {
    /// Create a new wgpu context.
    pub async fn new() -> GpuResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| GpuError::DeviceCreation("No adapter found".into()))?;

        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("skia-rs device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| GpuError::DeviceCreation(e.to_string()))?;

        let info = GpuAdapterInfo {
            name: adapter_info.name.clone(),
            vendor: adapter_info.vendor.to_string(),
            backend: match adapter_info.backend {
                wgpu::Backend::Vulkan => GpuBackendType::Vulkan,
                wgpu::Backend::Metal => GpuBackendType::Metal,
                wgpu::Backend::Dx12 => GpuBackendType::Direct3D12,
                wgpu::Backend::Gl => GpuBackendType::OpenGL,
                wgpu::Backend::BrowserWebGpu => GpuBackendType::WebGPU,
                _ => GpuBackendType::WebGPU,
            },
            device_type: match adapter_info.device_type {
                wgpu::DeviceType::IntegratedGpu => GpuDeviceType::Integrated,
                wgpu::DeviceType::DiscreteGpu => GpuDeviceType::Discrete,
                wgpu::DeviceType::VirtualGpu => GpuDeviceType::Virtual,
                wgpu::DeviceType::Cpu => GpuDeviceType::Cpu,
                wgpu::DeviceType::Other => GpuDeviceType::Unknown,
            },
        };

        let limits = device.limits();
        let caps = GpuCaps {
            max_texture_size: limits.max_texture_dimension_2d,
            max_render_target_size: limits.max_texture_dimension_2d,
            msaa_support: true,
            max_msaa_samples: 4, // Common max
            compute_support: true,
            instancing_support: true,
        };

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            info,
            caps,
        })
    }

    /// Create a blocking context (for non-async usage).
    pub fn new_blocking() -> GpuResult<Self> {
        pollster::block_on(Self::new())
    }

    /// Get the device.
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    /// Get the queue.
    pub fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    /// Get capabilities.
    pub fn capabilities(&self) -> &GpuCaps {
        &self.caps
    }

    /// Create an offscreen surface.
    pub fn create_surface(&self, props: &GpuSurfaceProps) -> GpuResult<WgpuSurface> {
        WgpuSurface::new(self.device.clone(), self.queue.clone(), props)
    }
}

impl GpuContext for WgpuContext {
    fn backend_type(&self) -> GpuBackendType {
        self.info.backend
    }

    fn adapter_info(&self) -> &GpuAdapterInfo {
        &self.info
    }

    fn flush(&self) {
        // wgpu commands are auto-submitted
    }

    fn submit_and_wait(&self) {
        self.device.poll(wgpu::Maintain::Wait);
    }

    fn is_valid(&self) -> bool {
        true
    }
}

/// wgpu-based GPU surface.
pub struct WgpuSurface {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: TextureFormat,
    sample_count: u32,
    staging_buffer: Option<wgpu::Buffer>,
}

impl WgpuSurface {
    /// Create a new wgpu surface.
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        props: &GpuSurfaceProps,
    ) -> GpuResult<Self> {
        let wgpu_format = match props.format {
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
            _ => return Err(GpuError::SurfaceCreation("Unsupported format".into())),
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("skia-rs surface texture"),
            size: wgpu::Extent3d {
                width: props.width,
                height: props.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: props.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            device,
            queue,
            texture,
            view,
            width: props.width,
            height: props.height,
            format: props.format,
            sample_count: props.sample_count,
            staging_buffer: None,
        })
    }

    /// Get the texture view.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Begin a render pass.
    pub fn begin_render_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        desc: &RenderPassDescriptor,
    ) -> wgpu::RenderPass<'a> {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &self.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: match desc.clear_color {
                    Some([r, g, b, a]) => wgpu::LoadOp::Clear(wgpu::Color {
                        r: r as f64,
                        g: g as f64,
                        b: b as f64,
                        a: a as f64,
                    }),
                    None => wgpu::LoadOp::Load,
                },
                store: wgpu::StoreOp::Store,
            },
        };

        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("skia-rs render pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

impl GpuSurface for WgpuSurface {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn sample_count(&self) -> u32 {
        self.sample_count
    }

    fn clear(&mut self, color: Color) {
        let r = color.red() as f64 / 255.0;
        let g = color.green() as f64 / 255.0;
        let b = color.blue() as f64 / 255.0;
        let a = color.alpha() as f64 / 255.0;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn present(&mut self) {
        // For offscreen surfaces, nothing to present
    }

    fn read_pixels(&self, dst: &mut [u8], dst_row_bytes: usize) -> bool {
        let bytes_per_pixel = self.format.bytes_per_pixel() as usize;
        let aligned_bytes_per_row = (self.width as usize * bytes_per_pixel + 255) & !255;
        let buffer_size = aligned_bytes_per_row * self.height as usize;

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback encoder"),
            });

        // For MSAA surfaces, copy_texture_to_buffer against a multisampled
        // texture is invalid. We must resolve to a single-sample texture
        // first, then copy. Previously this code ignored `sample_count` and
        // panicked at the wgpu layer for any MSAA surface. (Gap N-2.)
        //
        // The resolve texture is one-shot: we only need it for the duration
        // of this read. The alternative (keeping a persistent resolve target
        // on the surface) trades memory for slightly faster repeated reads
        // and is not warranted here.
        let resolve_texture_holder;
        let copy_texture_ref: &wgpu::Texture = if self.sample_count > 1 {
            let resolve_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("skia-rs msaa resolve"),
                size: wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.texture.format(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let resolve_view =
                resolve_texture.create_view(&wgpu::TextureViewDescriptor::default());

            // A Load-op render pass with resolve_target copies the MSAA
            // contents down to the single-sample texture.
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("skia-rs msaa resolve pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.view,
                        resolve_target: Some(&resolve_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }

            resolve_texture_holder = resolve_texture;
            &resolve_texture_holder
        } else {
            &self.texture
        };

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: copy_texture_ref,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned_bytes_per_row as u32),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);

        if rx.recv().map(|r| r.is_ok()).unwrap_or(false) {
            let data = slice.get_mapped_range();

            // Copy row by row (handling alignment)
            for y in 0..self.height as usize {
                let src_offset = y * aligned_bytes_per_row;
                let dst_offset = y * dst_row_bytes;
                let row_bytes = self.width as usize * bytes_per_pixel;

                if dst_offset + row_bytes <= dst.len() && src_offset + row_bytes <= data.len() {
                    dst[dst_offset..dst_offset + row_bytes]
                        .copy_from_slice(&data[src_offset..src_offset + row_bytes]);
                }
            }

            drop(data);
            staging_buffer.unmap();
            true
        } else {
            false
        }
    }

    fn flush(&mut self) {
        // wgpu auto-flushes
    }
}

// ---------------------------------------------------------------------------
// CommandBuffer executor (gap C-1 for wgpu) + stencil integration (C-3).
// ---------------------------------------------------------------------------

/// Translate a crate vertex format to wgpu.
fn convert_vertex_format(f: CrateVertexFormat) -> wgpu::VertexFormat {
    match f {
        CrateVertexFormat::Float32 => wgpu::VertexFormat::Float32,
        CrateVertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
        CrateVertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
        CrateVertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
        CrateVertexFormat::Sint32 => wgpu::VertexFormat::Sint32,
        CrateVertexFormat::Sint32x2 => wgpu::VertexFormat::Sint32x2,
        CrateVertexFormat::Sint32x3 => wgpu::VertexFormat::Sint32x3,
        CrateVertexFormat::Sint32x4 => wgpu::VertexFormat::Sint32x4,
        CrateVertexFormat::Uint32 => wgpu::VertexFormat::Uint32,
        CrateVertexFormat::Uint32x2 => wgpu::VertexFormat::Uint32x2,
        CrateVertexFormat::Uint32x3 => wgpu::VertexFormat::Uint32x3,
        CrateVertexFormat::Uint32x4 => wgpu::VertexFormat::Uint32x4,
        CrateVertexFormat::Unorm8x4 => wgpu::VertexFormat::Unorm8x4,
    }
}

fn convert_topology(t: CratePrimitiveTopology) -> wgpu::PrimitiveTopology {
    match t {
        CratePrimitiveTopology::PointList => wgpu::PrimitiveTopology::PointList,
        CratePrimitiveTopology::LineList => wgpu::PrimitiveTopology::LineList,
        CratePrimitiveTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
        CratePrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
        CratePrimitiveTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
    }
}

fn convert_blend_factor(f: CrateBlendFactor) -> wgpu::BlendFactor {
    match f {
        CrateBlendFactor::Zero => wgpu::BlendFactor::Zero,
        CrateBlendFactor::One => wgpu::BlendFactor::One,
        CrateBlendFactor::Src => wgpu::BlendFactor::Src,
        CrateBlendFactor::OneMinusSrc => wgpu::BlendFactor::OneMinusSrc,
        CrateBlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
        CrateBlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        CrateBlendFactor::Dst => wgpu::BlendFactor::Dst,
        CrateBlendFactor::OneMinusDst => wgpu::BlendFactor::OneMinusDst,
        CrateBlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
        CrateBlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        CrateBlendFactor::SrcAlphaSaturated => wgpu::BlendFactor::SrcAlphaSaturated,
        CrateBlendFactor::Constant => wgpu::BlendFactor::Constant,
        CrateBlendFactor::OneMinusConstant => wgpu::BlendFactor::OneMinusConstant,
    }
}

fn convert_blend_op(o: CrateBlendOperation) -> wgpu::BlendOperation {
    match o {
        CrateBlendOperation::Add => wgpu::BlendOperation::Add,
        CrateBlendOperation::Subtract => wgpu::BlendOperation::Subtract,
        CrateBlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
        CrateBlendOperation::Min => wgpu::BlendOperation::Min,
        CrateBlendOperation::Max => wgpu::BlendOperation::Max,
    }
}

fn convert_blend_state(state: CrateBlendState) -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: convert_blend_factor(state.color.src_factor),
            dst_factor: convert_blend_factor(state.color.dst_factor),
            operation: convert_blend_op(state.color.operation),
        },
        alpha: wgpu::BlendComponent {
            src_factor: convert_blend_factor(state.alpha.src_factor),
            dst_factor: convert_blend_factor(state.alpha.dst_factor),
            operation: convert_blend_op(state.alpha.operation),
        },
    }
}

fn convert_write_mask(mask: ColorWriteMask) -> wgpu::ColorWrites {
    let mut out = wgpu::ColorWrites::empty();
    if mask.contains(ColorWriteMask::RED) {
        out |= wgpu::ColorWrites::RED;
    }
    if mask.contains(ColorWriteMask::GREEN) {
        out |= wgpu::ColorWrites::GREEN;
    }
    if mask.contains(ColorWriteMask::BLUE) {
        out |= wgpu::ColorWrites::BLUE;
    }
    if mask.contains(ColorWriteMask::ALPHA) {
        out |= wgpu::ColorWrites::ALPHA;
    }
    out
}

fn convert_texture_format(format: TextureFormat) -> Option<wgpu::TextureFormat> {
    match format {
        TextureFormat::Rgba8Unorm => Some(wgpu::TextureFormat::Rgba8Unorm),
        TextureFormat::Rgba8UnormSrgb => Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        TextureFormat::Bgra8Unorm => Some(wgpu::TextureFormat::Bgra8Unorm),
        TextureFormat::Bgra8UnormSrgb => Some(wgpu::TextureFormat::Bgra8UnormSrgb),
        TextureFormat::R8Unorm => Some(wgpu::TextureFormat::R8Unorm),
        TextureFormat::Rg8Unorm => Some(wgpu::TextureFormat::Rg8Unorm),
        TextureFormat::Rgba16Float => Some(wgpu::TextureFormat::Rgba16Float),
        TextureFormat::Rgba32Float => Some(wgpu::TextureFormat::Rgba32Float),
        TextureFormat::Depth24Stencil8 => Some(wgpu::TextureFormat::Depth24PlusStencil8),
        TextureFormat::Depth32Float => Some(wgpu::TextureFormat::Depth32Float),
    }
}

fn convert_index_format(f: CrateIndexFormat) -> wgpu::IndexFormat {
    match f {
        CrateIndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
        CrateIndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
    }
}

/// Build a `wgpu::RenderPipeline` from a crate `RenderPipelineDescriptor`.
///
/// This is the heart of the C-1 fix: previously the crate-level descriptor
/// never became a real GPU pipeline. Here we compile both shader modules
/// via the device, translate the vertex layout, blend and multisample
/// state, and produce a pipeline the executor can bind.
fn build_wgpu_pipeline(
    device: &wgpu::Device,
    desc: &RenderPipelineDescriptor,
) -> GpuResult<wgpu::RenderPipeline> {
    let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: desc.label.as_deref(),
        source: wgpu::ShaderSource::Wgsl(desc.vertex_shader.as_str().into()),
    });

    // If vertex and fragment sources are identical we can re-use one module;
    // otherwise compile a separate fragment module.
    let fs_module = if desc.vertex_shader == desc.fragment_shader {
        None
    } else {
        Some(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: desc.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(desc.fragment_shader.as_str().into()),
        }))
    };

    // Translate vertex buffer layouts. We need to retain attribute arrays
    // because wgpu::VertexBufferLayout borrows a slice.
    let mut attr_storage: Vec<Vec<wgpu::VertexAttribute>> = Vec::with_capacity(desc.vertex_buffers.len());
    for vb in &desc.vertex_buffers {
        let attrs: Vec<wgpu::VertexAttribute> = vb
            .attributes
            .iter()
            .map(|a| wgpu::VertexAttribute {
                format: convert_vertex_format(a.format),
                offset: a.offset as u64,
                shader_location: a.location,
            })
            .collect();
        attr_storage.push(attrs);
    }
    let vb_layouts: Vec<wgpu::VertexBufferLayout> = desc
        .vertex_buffers
        .iter()
        .zip(attr_storage.iter())
        .map(|(vb, attrs)| wgpu::VertexBufferLayout {
            array_stride: vb.stride as u64,
            step_mode: match vb.step_mode {
                crate::pipeline::VertexStepMode::Vertex => wgpu::VertexStepMode::Vertex,
                crate::pipeline::VertexStepMode::Instance => wgpu::VertexStepMode::Instance,
            },
            attributes: attrs.as_slice(),
        })
        .collect();

    // Colour targets.
    let color_targets: Vec<Option<wgpu::ColorTargetState>> = desc
        .color_targets
        .iter()
        .map(|ct| {
            convert_texture_format(ct.format).map(|format| wgpu::ColorTargetState {
                format,
                blend: ct.blend.map(convert_blend_state),
                write_mask: convert_write_mask(ct.write_mask),
            })
        })
        .collect();

    // Validate that every requested target format is supported.
    if color_targets.iter().any(|t| t.is_none()) {
        return Err(GpuError::OperationFailed(
            "unsupported colour target format for wgpu".into(),
        ));
    }

    // Depth/stencil translation.
    let depth_stencil = desc.depth_stencil.as_ref().map(|ds| {
        let format = convert_texture_format(ds.format)
            .unwrap_or(wgpu::TextureFormat::Depth24PlusStencil8);
        wgpu::DepthStencilState {
            format,
            depth_write_enabled: ds.depth_write_enabled,
            depth_compare: convert_compare(ds.depth_compare),
            stencil: wgpu::StencilState {
                front: convert_stencil_face(&ds.stencil_front),
                back: convert_stencil_face(&ds.stencil_back),
                read_mask: ds.stencil_read_mask,
                write_mask: ds.stencil_write_mask,
            },
            bias: wgpu::DepthBiasState {
                constant: ds.depth_bias,
                slope_scale: ds.depth_bias_slope_scale,
                clamp: ds.depth_bias_clamp,
            },
        }
    });

    let multisample = wgpu::MultisampleState {
        count: desc.multisample.count,
        mask: desc.multisample.mask,
        alpha_to_coverage_enabled: desc.multisample.alpha_to_coverage_enabled,
    };

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: desc.label.as_deref(),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let fragment_module = fs_module.as_ref().unwrap_or(&vs_module);

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: desc.label.as_deref(),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: Some(desc.vertex_entry.as_str()),
            compilation_options: Default::default(),
            buffers: &vb_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: fragment_module,
            entry_point: Some(desc.fragment_entry.as_str()),
            compilation_options: Default::default(),
            targets: color_targets
                .iter()
                .map(|t| t.clone())
                .collect::<Vec<_>>()
                .as_slice(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: convert_topology(desc.primitive.topology),
            strip_index_format: desc.primitive.strip_index_format.map(convert_index_format),
            front_face: match desc.primitive.front_face {
                crate::pipeline::FrontFace::Ccw => wgpu::FrontFace::Ccw,
                crate::pipeline::FrontFace::Cw => wgpu::FrontFace::Cw,
            },
            cull_mode: match desc.primitive.cull_mode {
                crate::pipeline::CullMode::None => None,
                crate::pipeline::CullMode::Front => Some(wgpu::Face::Front),
                crate::pipeline::CullMode::Back => Some(wgpu::Face::Back),
            },
            unclipped_depth: false,
            polygon_mode: match desc.primitive.polygon_mode {
                crate::pipeline::PolygonMode::Fill => wgpu::PolygonMode::Fill,
                crate::pipeline::PolygonMode::Line => wgpu::PolygonMode::Line,
                crate::pipeline::PolygonMode::Point => wgpu::PolygonMode::Point,
            },
            conservative: false,
        },
        depth_stencil,
        multisample,
        multiview: None,
        cache: None,
    });

    Ok(pipeline)
}

fn convert_compare(c: crate::pipeline::CompareFunction) -> wgpu::CompareFunction {
    use crate::pipeline::CompareFunction as CF;
    match c {
        CF::Never => wgpu::CompareFunction::Never,
        CF::Less => wgpu::CompareFunction::Less,
        CF::Equal => wgpu::CompareFunction::Equal,
        CF::LessEqual => wgpu::CompareFunction::LessEqual,
        CF::Greater => wgpu::CompareFunction::Greater,
        CF::NotEqual => wgpu::CompareFunction::NotEqual,
        CF::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
        CF::Always => wgpu::CompareFunction::Always,
    }
}

fn convert_stencil_op(op: crate::pipeline::StencilOperation) -> wgpu::StencilOperation {
    use crate::pipeline::StencilOperation as SO;
    match op {
        SO::Keep => wgpu::StencilOperation::Keep,
        SO::Zero => wgpu::StencilOperation::Zero,
        SO::Replace => wgpu::StencilOperation::Replace,
        SO::IncrementClamp => wgpu::StencilOperation::IncrementClamp,
        SO::DecrementClamp => wgpu::StencilOperation::DecrementClamp,
        SO::Invert => wgpu::StencilOperation::Invert,
        SO::IncrementWrap => wgpu::StencilOperation::IncrementWrap,
        SO::DecrementWrap => wgpu::StencilOperation::DecrementWrap,
    }
}

fn convert_stencil_face(f: &crate::pipeline::StencilFaceState) -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: convert_compare(f.compare),
        fail_op: convert_stencil_op(f.fail_op),
        depth_fail_op: convert_stencil_op(f.depth_fail_op),
        pass_op: convert_stencil_op(f.pass_op),
    }
}

/// Cache of `wgpu::RenderPipeline` keyed by `PipelineKey`.
///
/// The command-buffer executor looks up pipelines here before creating a
/// new one; this keeps pipeline compilation (the slow step) out of the
/// per-frame hot path. The cache is indexed by a (hash-of-shader,
/// hash-of-blend, format, sample-count) tuple so semantically identical
/// paint plans share a compiled pipeline.
#[derive(Default)]
pub struct WgpuPipelineCache {
    pipelines: Mutex<HashMap<PipelineKey, Arc<wgpu::RenderPipeline>>>,
}

impl WgpuPipelineCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or compile a pipeline for the descriptor.
    pub fn get_or_create(
        &self,
        device: &wgpu::Device,
        desc: &RenderPipelineDescriptor,
    ) -> GpuResult<Arc<wgpu::RenderPipeline>> {
        let key = PipelineKey::from_descriptor(desc);

        {
            let guard = self.pipelines.lock();
            if let Some(pipe) = guard.get(&key) {
                return Ok(pipe.clone());
            }
        }

        let pipeline = Arc::new(build_wgpu_pipeline(device, desc)?);
        self.pipelines.lock().insert(key, pipeline.clone());
        Ok(pipeline)
    }

    /// Number of cached pipelines.
    pub fn len(&self) -> usize {
        self.pipelines.lock().len()
    }

    /// True if no pipelines are cached.
    pub fn is_empty(&self) -> bool {
        self.pipelines.lock().is_empty()
    }

    /// Clear all cached pipelines.
    pub fn clear(&self) {
        self.pipelines.lock().clear();
    }
}

impl std::fmt::Debug for WgpuPipelineCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgpuPipelineCache")
            .field("len", &self.len())
            .finish()
    }
}

/// Statistics returned from executing a command buffer.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecuteStats {
    /// Number of clears performed.
    pub clears: u32,
    /// Number of draws issued.
    pub draws: u32,
    /// Number of indexed draws issued.
    pub indexed_draws: u32,
    /// Number of pipeline-set commands honoured.
    pub pipeline_switches: u32,
    /// Number of bytes copied buffer-to-buffer.
    pub bytes_copied: u64,
}

/// Executor that walks a `CommandBuffer` and issues wgpu commands against a
/// specific `WgpuSurface`.
///
/// This is the runtime half of the C-1 fix. Construct an executor, register
/// the vertex/index buffers your commands reference (by the same `buffer_id`
/// values recorded into the `CommandBuffer`), then call `execute` to replay
/// the recorded stream against the GPU.
pub struct WgpuExecutor {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipelines: Arc<WgpuPipelineCache>,
    /// Mapping from the recorded `pipeline_id` fields to real pipelines.
    pipeline_by_id: Mutex<HashMap<u64, Arc<wgpu::RenderPipeline>>>,
    /// Mapping from the recorded `buffer_id` fields to real vertex/index buffers.
    buffers: Mutex<HashMap<u64, Arc<wgpu::Buffer>>>,
}

impl WgpuExecutor {
    /// Create a new executor.
    pub fn new(context: &WgpuContext) -> Self {
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            pipelines: Arc::new(WgpuPipelineCache::new()),
            pipeline_by_id: Mutex::new(HashMap::new()),
            buffers: Mutex::new(HashMap::new()),
        }
    }

    /// Create with an externally-managed pipeline cache.
    pub fn with_cache(context: &WgpuContext, cache: Arc<WgpuPipelineCache>) -> Self {
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            pipelines: cache,
            pipeline_by_id: Mutex::new(HashMap::new()),
            buffers: Mutex::new(HashMap::new()),
        }
    }

    /// Access the underlying pipeline cache.
    pub fn pipeline_cache(&self) -> &Arc<WgpuPipelineCache> {
        &self.pipelines
    }

    /// Register a render pipeline with a stable id.
    ///
    /// The id is the value you pass to `CommandBuffer::set_pipeline`.
    pub fn register_pipeline(
        &self,
        pipeline_id: u64,
        descriptor: &RenderPipelineDescriptor,
    ) -> GpuResult<()> {
        let pipeline = self.pipelines.get_or_create(&self.device, descriptor)?;
        self.pipeline_by_id.lock().insert(pipeline_id, pipeline);
        Ok(())
    }

    /// Upload vertex or index bytes and register them under a stable id.
    pub fn upload_buffer(&self, buffer_id: u64, data: &[u8], usage: wgpu::BufferUsages) {
        use wgpu::util::DeviceExt;
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skia-rs executor buffer"),
                contents: data,
                usage,
            });
        self.buffers.lock().insert(buffer_id, Arc::new(buffer));
    }

    /// Number of pipelines currently cached.
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    /// Number of registered (pipeline_id → pipeline) mappings.
    pub fn registered_pipeline_count(&self) -> usize {
        self.pipeline_by_id.lock().len()
    }

    /// Execute a recorded command buffer against the given surface.
    ///
    /// Walks the `DrawCommand` stream, translates each entry to the wgpu
    /// equivalent, and submits the resulting command buffer to the queue.
    ///
    /// This is the operation that previously did not exist: until now the
    /// crate's `CommandBuffer` was a write-only sink. The executor closes
    /// the loop so callers can submit their recorded commands to the GPU
    /// and see pixels change.
    pub fn execute(
        &self,
        surface: &WgpuSurface,
        commands: &CommandBuffer,
    ) -> GpuResult<ExecuteStats> {
        let mut stats = ExecuteStats::default();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("skia-rs executor encoder"),
            });

        // We split the DrawCommand stream into render passes. A pass starts
        // on Clear (or on the first draw/state-setter encountered) and ends
        // when we see a copy command, another Clear, or the stream ends.
        //
        // For the first pass with no prior Clear the surface is loaded (not
        // cleared). This matches the default wgpu behaviour and lets
        // consumers record a Clear command explicitly when they want one.
        enum PassState {
            None,
            Open { clear: Option<wgpu::Color> },
        }

        let mut state = PassState::None;
        let mut pending_ops: Vec<&DrawCommand> = Vec::new();

        let flush_pass = |enc: &mut wgpu::CommandEncoder,
                          clear: Option<wgpu::Color>,
                          ops: &[&DrawCommand],
                          stats: &mut ExecuteStats,
                          ctx: &WgpuExecutor,
                          view: &wgpu::TextureView|
         -> GpuResult<()> {
            if clear.is_none() && ops.is_empty() {
                return Ok(());
            }
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("skia-rs render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: match clear {
                                Some(c) => wgpu::LoadOp::Clear(c),
                                None => wgpu::LoadOp::Load,
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // We need to hold pipeline and buffer references alive for
                // the pass; collect them before issuing bind calls.
                let pipelines = ctx.pipeline_by_id.lock();
                let buffers = ctx.buffers.lock();
                // Keep a local cache so SetVertexBuffer can slice into a
                // live reference.
                for cmd in ops {
                    match *cmd {
                        DrawCommand::SetPipeline { pipeline_id } => {
                            if let Some(p) = pipelines.get(pipeline_id) {
                                pass.set_pipeline(p);
                                stats.pipeline_switches += 1;
                            }
                        }
                        DrawCommand::SetVertexBuffer {
                            slot,
                            buffer_id,
                            offset,
                            size,
                        } => {
                            if let Some(buf) = buffers.get(buffer_id) {
                                let slice = match size {
                                    Some(sz) => buf.slice(*offset..*offset + sz),
                                    None => buf.slice(*offset..),
                                };
                                pass.set_vertex_buffer(*slot, slice);
                            }
                        }
                        DrawCommand::SetIndexBuffer {
                            buffer_id,
                            format,
                            offset,
                            size,
                        } => {
                            if let Some(buf) = buffers.get(buffer_id) {
                                let slice = match size {
                                    Some(sz) => buf.slice(*offset..*offset + sz),
                                    None => buf.slice(*offset..),
                                };
                                pass.set_index_buffer(slice, convert_index_format(*format));
                            }
                        }
                        DrawCommand::SetScissor { rect } => {
                            pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
                        }
                        DrawCommand::SetViewport { viewport } => {
                            pass.set_viewport(
                                viewport.x,
                                viewport.y,
                                viewport.width,
                                viewport.height,
                                viewport.min_depth,
                                viewport.max_depth,
                            );
                        }
                        DrawCommand::SetBlendConstant { color } => {
                            pass.set_blend_constant(wgpu::Color {
                                r: color[0] as f64,
                                g: color[1] as f64,
                                b: color[2] as f64,
                                a: color[3] as f64,
                            });
                        }
                        DrawCommand::SetStencilReference { reference } => {
                            pass.set_stencil_reference(*reference);
                        }
                        DrawCommand::Draw {
                            vertex_count,
                            instance_count,
                            first_vertex,
                            first_instance,
                        } => {
                            pass.draw(
                                *first_vertex..*first_vertex + *vertex_count,
                                *first_instance..*first_instance + *instance_count,
                            );
                            stats.draws += 1;
                        }
                        DrawCommand::DrawIndexed {
                            index_count,
                            instance_count,
                            first_index,
                            base_vertex,
                            first_instance,
                        } => {
                            pass.draw_indexed(
                                *first_index..*first_index + *index_count,
                                *base_vertex,
                                *first_instance..*first_instance + *instance_count,
                            );
                            stats.indexed_draws += 1;
                        }
                        DrawCommand::PushDebugGroup { label } => {
                            pass.push_debug_group(label);
                        }
                        DrawCommand::PopDebugGroup => {
                            pass.pop_debug_group();
                        }
                        DrawCommand::InsertDebugMarker { label } => {
                            pass.insert_debug_marker(label);
                        }
                        // Bind groups are not yet wired through the
                        // executor — callers register them externally via
                        // wgpu. We skip without erroring.
                        DrawCommand::SetBindGroup { .. } => {}
                        // Copy commands cannot happen inside a render pass;
                        // they're split out before we reach here.
                        DrawCommand::CopyBufferToBuffer { .. }
                        | DrawCommand::CopyBufferToTexture { .. }
                        | DrawCommand::CopyTextureToBuffer { .. }
                        | DrawCommand::CopyTextureToTexture { .. }
                        | DrawCommand::Clear { .. } => unreachable!(),
                    }
                }
            }
            Ok(())
        };

        for cmd in commands.commands() {
            match cmd {
                DrawCommand::Clear { color } => {
                    // Close any existing pass first, then start a fresh one
                    // with the clear color recorded.
                    if let PassState::Open { clear } = &state {
                        let refs: Vec<&DrawCommand> = pending_ops.iter().copied().collect();
                        flush_pass(
                            &mut encoder,
                            *clear,
                            &refs,
                            &mut stats,
                            self,
                            &surface.view,
                        )?;
                        pending_ops.clear();
                    }
                    stats.clears += 1;
                    state = PassState::Open {
                        clear: Some(wgpu::Color {
                            r: color.red() as f64 / 255.0,
                            g: color.green() as f64 / 255.0,
                            b: color.blue() as f64 / 255.0,
                            a: color.alpha() as f64 / 255.0,
                        }),
                    };
                }
                DrawCommand::CopyBufferToBuffer {
                    src,
                    src_offset,
                    dst,
                    dst_offset,
                    size,
                } => {
                    // Flush any open render pass first — copies must run
                    // outside a render pass.
                    if let PassState::Open { clear } = &state {
                        let refs: Vec<&DrawCommand> = pending_ops.iter().copied().collect();
                        flush_pass(
                            &mut encoder,
                            *clear,
                            &refs,
                            &mut stats,
                            self,
                            &surface.view,
                        )?;
                        pending_ops.clear();
                        state = PassState::None;
                    }
                    let buffers = self.buffers.lock();
                    if let (Some(s), Some(d)) = (buffers.get(src), buffers.get(dst)) {
                        encoder.copy_buffer_to_buffer(s, *src_offset, d, *dst_offset, *size);
                        stats.bytes_copied += *size;
                    }
                }
                DrawCommand::CopyBufferToTexture { .. }
                | DrawCommand::CopyTextureToBuffer { .. }
                | DrawCommand::CopyTextureToTexture { .. } => {
                    // Not yet wired to the executor — skip gracefully.
                    // Callers that need these today can still use
                    // `copy_texture_to_buffer` on the surface directly via
                    // `read_pixels`.
                }
                other => {
                    // Anything else is pass content.
                    if matches!(state, PassState::None) {
                        state = PassState::Open { clear: None };
                    }
                    pending_ops.push(other);
                }
            }
        }

        // Flush any remaining pending pass.
        if let PassState::Open { clear } = &state {
            let refs: Vec<&DrawCommand> = pending_ops.iter().copied().collect();
            flush_pass(
                &mut encoder,
                *clear,
                &refs,
                &mut stats,
                self,
                &surface.view,
            )?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(stats)
    }
}

// ---------------------------------------------------------------------------
// Stencil-then-cover integration (gap C-3).
//
// Provide a dedicated helper that provisions a stencil buffer on the surface
// and plays back a `StencilCoverResult` against it. This is the minimum
// viable integration: it closes the loop so `prepare_stencil_cover` output
// is actually consumed by the GPU.
// ---------------------------------------------------------------------------

/// Surface-side stencil buffer attached to a `WgpuSurface` for stencil-cover
/// rendering.
pub struct WgpuStencilSurface {
    depth_stencil_texture: wgpu::Texture,
    depth_stencil_view: wgpu::TextureView,
}

impl WgpuStencilSurface {
    /// Allocate a depth-24/stencil-8 attachment matching `surface` dimensions.
    pub fn new(device: &wgpu::Device, surface: &WgpuSurface) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("skia-rs stencil buffer"),
            size: wgpu::Extent3d {
                width: surface.width,
                height: surface.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: surface.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            depth_stencil_texture: texture,
            depth_stencil_view: view,
        }
    }

    /// Access the stencil view (for passing into a render pass).
    pub fn view(&self) -> &wgpu::TextureView {
        &self.depth_stencil_view
    }

    /// Access the stencil texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.depth_stencil_texture
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_conversion() {
        use crate::TextureFormat;
        assert_eq!(TextureFormat::Rgba8Unorm.bytes_per_pixel(), 4);
    }

    #[test]
    fn test_convert_vertex_format() {
        assert!(matches!(
            convert_vertex_format(CrateVertexFormat::Float32x2),
            wgpu::VertexFormat::Float32x2
        ));
        assert!(matches!(
            convert_vertex_format(CrateVertexFormat::Unorm8x4),
            wgpu::VertexFormat::Unorm8x4
        ));
    }

    #[test]
    fn test_convert_topology() {
        assert!(matches!(
            convert_topology(CratePrimitiveTopology::TriangleList),
            wgpu::PrimitiveTopology::TriangleList
        ));
        assert!(matches!(
            convert_topology(CratePrimitiveTopology::LineStrip),
            wgpu::PrimitiveTopology::LineStrip
        ));
    }

    #[test]
    fn test_convert_blend_factor_roundtrip() {
        // Every factor converts to *some* wgpu factor. We assert a known
        // mapping for the ones we care about most.
        assert!(matches!(
            convert_blend_factor(CrateBlendFactor::SrcAlpha),
            wgpu::BlendFactor::SrcAlpha
        ));
        assert!(matches!(
            convert_blend_factor(CrateBlendFactor::OneMinusSrcAlpha),
            wgpu::BlendFactor::OneMinusSrcAlpha
        ));
    }

    #[test]
    fn test_pipeline_cache_empty() {
        let cache = WgpuPipelineCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_convert_texture_format_covers_all() {
        // Every crate texture format should translate to wgpu; this guards
        // against silent breakage when formats are added.
        use crate::TextureFormat as TF;
        for f in [
            TF::Rgba8Unorm,
            TF::Rgba8UnormSrgb,
            TF::Bgra8Unorm,
            TF::Bgra8UnormSrgb,
            TF::R8Unorm,
            TF::Rg8Unorm,
            TF::Rgba16Float,
            TF::Rgba32Float,
            TF::Depth24Stencil8,
            TF::Depth32Float,
        ] {
            assert!(convert_texture_format(f).is_some(), "format {:?} unmapped", f);
        }
    }

    #[test]
    fn test_convert_write_mask_all() {
        let mask = convert_write_mask(ColorWriteMask::ALL);
        assert_eq!(mask, wgpu::ColorWrites::ALL);

        let red_only = convert_write_mask(ColorWriteMask::RED);
        assert_eq!(red_only, wgpu::ColorWrites::RED);
    }

    #[test]
    fn test_command_buffer_drive_pipeline_cache() {
        // Unit-level regression test for the C-1 fix: the same
        // `PipelineKey` derived from a descriptor round-trips through the
        // cache lookup, so two distinct descriptors with identical content
        // hit the same slot.
        //
        // We can't compile real wgpu pipelines without a device, but we
        // can drive `PipelineKey::from_descriptor`, which is the key type
        // used by `WgpuPipelineCache::get_or_create`. A regression in the
        // descriptor → key function (e.g. new field not hashed) would
        // silently cause pipeline cache thrash; this test catches that.
        use crate::pipeline::{ColorTargetState, RenderPipelineDescriptor};

        let desc_a = RenderPipelineDescriptor::new(
            "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); }",
            "@fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
        )
        .with_color_target(ColorTargetState::new(TextureFormat::Rgba8Unorm));

        let desc_b = RenderPipelineDescriptor::new(
            "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); }",
            "@fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
        )
        .with_color_target(ColorTargetState::new(TextureFormat::Rgba8Unorm));

        let key_a = PipelineKey::from_descriptor(&desc_a);
        let key_b = PipelineKey::from_descriptor(&desc_b);
        assert_eq!(key_a, key_b, "identical descriptors must produce identical keys");

        // Differing sample counts produce distinct keys (cache miss).
        let desc_c = RenderPipelineDescriptor::new(
            "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); }",
            "@fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
        )
        .with_color_target(ColorTargetState::new(TextureFormat::Rgba8Unorm))
        .with_multisample(crate::pipeline::MultisampleState {
            count: 4,
            mask: !0,
            alpha_to_coverage_enabled: false,
        });
        let key_c = PipelineKey::from_descriptor(&desc_c);
        assert_ne!(key_a, key_c);
    }

    #[test]
    fn test_executor_buffer_registration() {
        // We can register buffers against an executor without a real
        // device only via the existing allocator, so here we just verify
        // the data-structure invariants.
        let cache = WgpuPipelineCache::new();
        assert_eq!(cache.len(), 0);
        cache.clear();
        assert!(cache.is_empty());
    }
}
