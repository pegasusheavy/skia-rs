---
description: GPU backend guidelines for skia-rs-gpu
globs: ["crates/skia-rs-gpu/**"]
alwaysApply: false
---

# GPU Backend Guidelines

## Feature Flags

```toml
[features]
default = ["wgpu-backend"]
vulkan = ["dep:ash"]
opengl = ["dep:glow"]
wgpu-backend = ["dep:wgpu"]
```

## Backend Abstraction

```rust
pub trait GpuBackend: Send + Sync {
    fn create_surface(&self, info: &ImageInfo) -> Result<GpuSurface, GpuError>;
    fn flush(&self);
}
```

## Supported Backends

- **wgpu**: Cross-platform WebGPU abstraction (default)
- **Vulkan**: Low-level via `ash` crate
- **OpenGL**: Via `glow` crate
- **Metal**: macOS/iOS (planned)

## Resource Management

- Use GPU resource pools for frequently allocated objects
- Implement proper cleanup in `Drop` traits
- Handle device lost scenarios gracefully
