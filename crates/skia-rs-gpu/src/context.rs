//! GPU context abstraction.

use thiserror::Error;

/// Errors from GPU operations.
#[derive(Debug, Error)]
pub enum GpuError {
    /// Device creation failed.
    #[error("failed to create device: {0}")]
    DeviceCreation(String),
    /// Surface creation failed.
    #[error("failed to create surface: {0}")]
    SurfaceCreation(String),
    /// Resource creation failed.
    #[error("failed to create resource: {0}")]
    ResourceCreation(String),
    /// Backend not available.
    #[error("backend not available: {0}")]
    BackendNotAvailable(String),
    /// Operation failed.
    #[error("operation failed: {0}")]
    OperationFailed(String),
}

/// Result type for GPU operations.
pub type GpuResult<T> = Result<T, GpuError>;

/// GPU backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuBackendType {
    /// Vulkan backend.
    Vulkan,
    /// OpenGL backend.
    OpenGL,
    /// Metal backend (macOS/iOS).
    Metal,
    /// Direct3D 12 backend (Windows).
    Direct3D12,
    /// WebGPU backend.
    WebGPU,
}

/// Information about a GPU adapter.
#[derive(Debug, Clone)]
pub struct GpuAdapterInfo {
    /// Adapter name.
    pub name: String,
    /// Vendor name.
    pub vendor: String,
    /// Backend type.
    pub backend: GpuBackendType,
    /// Device type.
    pub device_type: GpuDeviceType,
}

/// GPU device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuDeviceType {
    /// Integrated GPU.
    Integrated,
    /// Discrete GPU.
    Discrete,
    /// Virtual GPU.
    Virtual,
    /// CPU (software rendering).
    Cpu,
    /// Unknown device type.
    Unknown,
}

/// Trait for GPU contexts.
pub trait GpuContext: Send + Sync {
    /// Get backend type.
    fn backend_type(&self) -> GpuBackendType;

    /// Get adapter info.
    fn adapter_info(&self) -> &GpuAdapterInfo;

    /// Flush pending commands.
    fn flush(&self);

    /// Submit commands and wait for completion.
    fn submit_and_wait(&self);

    /// Check if the context is still valid.
    fn is_valid(&self) -> bool;
}

/// Capabilities of the GPU.
#[derive(Debug, Clone, Default)]
pub struct GpuCaps {
    /// Maximum texture dimension.
    pub max_texture_size: u32,
    /// Maximum render target size.
    pub max_render_target_size: u32,
    /// Supports MSAA.
    pub msaa_support: bool,
    /// Maximum MSAA sample count.
    pub max_msaa_samples: u32,
    /// Supports compute shaders.
    pub compute_support: bool,
    /// Supports instanced drawing.
    pub instancing_support: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_types() {
        let backend = GpuBackendType::WebGPU;
        assert_eq!(backend, GpuBackendType::WebGPU);
    }
}
