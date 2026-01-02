//! WebGPU backend implementation using wgpu.

use crate::{
    GpuAdapterInfo, GpuBackendType, GpuCaps, GpuContext, GpuDeviceType, GpuError, GpuResult,
    GpuSurface, GpuSurfaceProps, RenderPassDescriptor, TextureFormat,
};
use parking_lot::Mutex;
use skia_rs_core::Color;
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

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
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

#[cfg(test)]
mod tests {
    // Note: GPU tests require a GPU and are typically run manually
    // or in CI with appropriate hardware

    #[test]
    fn test_format_conversion() {
        use crate::TextureFormat;
        assert_eq!(TextureFormat::Rgba8Unorm.bytes_per_pixel(), 4);
    }
}
