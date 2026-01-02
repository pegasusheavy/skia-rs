//! GPU rendering example for skia-rs
//!
//! This example demonstrates:
//! - Creating a wgpu GPU context
//! - Creating an offscreen GPU surface
//! - Clearing with a color
//! - Reading back pixels to CPU
//!
//! Note: This requires a GPU. If no GPU is available, it will fail gracefully.

use skia_rs_codec::{ImageEncoder, ImageInfo, PngEncoder};
use skia_rs_core::{AlphaType, ColorType};
use skia_rs_core::Color;
use std::fs::File;
use std::io::BufWriter;

fn main() {
    println!("skia-rs GPU Rendering Example");
    println!("==============================\n");

    // Try to create a GPU context
    #[cfg(feature = "wgpu-backend")]
    {
        use skia_rs_gpu::{GpuContext, GpuSurface, GpuSurfaceProps, TextureFormat, WgpuContext};

        match WgpuContext::new_blocking() {
            Ok(context) => {
                println!("GPU Context Created Successfully!");
                println!("  Adapter: {}", context.adapter_info().name);
                println!("  Backend: {:?}", context.backend_type());
                println!(
                    "  Device Type: {:?}",
                    context.adapter_info().device_type
                );

                let caps = context.capabilities();
                println!("\nCapabilities:");
                println!("  Max Texture Size: {}", caps.max_texture_size);
                println!("  MSAA Support: {}", caps.msaa_support);
                println!("  Compute Support: {}", caps.compute_support);

                // Create an offscreen surface
                let width = 800;
                let height = 600;

                let props = GpuSurfaceProps {
                    width,
                    height,
                    format: TextureFormat::Rgba8Unorm,
                    sample_count: 1,
                    srgb: false,
                };

                match context.create_surface(&props) {
                    Ok(mut surface) => {
                        println!("\nCreated {}x{} GPU surface", width, height);

                        // Clear with a gradient-like pattern (we'll do multiple clears)
                        // Since we don't have a full GPU drawing pipeline yet,
                        // we'll demonstrate the clear and readback functionality

                        // Clear to a nice blue color
                        let clear_color = Color::from_rgb(41, 128, 185);
                        surface.clear(clear_color);
                        println!("Cleared surface to blue");

                        // Read back pixels
                        let bytes_per_pixel = 4;
                        let row_bytes = width as usize * bytes_per_pixel;
                        let mut pixels = vec![0u8; row_bytes * height as usize];

                        if surface.read_pixels(&mut pixels, row_bytes) {
                            println!("Read back {} bytes from GPU", pixels.len());

                            // Verify the color (first pixel should be our clear color)
                            let r = pixels[0];
                            let g = pixels[1];
                            let b = pixels[2];
                            let a = pixels[3];
                            println!(
                                "First pixel: rgba({}, {}, {}, {})",
                                r, g, b, a
                            );

                            // Save to file
                            let output_path = "gpu_rendering_output.png";
                            let file = File::create(output_path).expect("Failed to create output file");
                            let ref mut writer = BufWriter::new(file);

                            let img_info = ImageInfo::new(
                                width as i32,
                                height as i32,
                                ColorType::Rgba8888,
                                AlphaType::Opaque,
                            );

                            if let Some(image) =
                                skia_rs_codec::Image::from_raster_data(&img_info, &pixels, row_bytes)
                            {
                                let encoder = PngEncoder::new();
                                encoder.encode(&image, writer).expect("Failed to encode PNG");
                                println!("\nSaved output to: {}", output_path);
                            }
                        } else {
                            eprintln!("Failed to read pixels from GPU surface");
                        }

                        // Flush and wait for GPU
                        surface.flush();
                        context.submit_and_wait();
                        println!("\nGPU operations complete");
                    }
                    Err(e) => {
                        eprintln!("Failed to create GPU surface: {:?}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to create GPU context: {:?}", e);
                eprintln!("This may be because no compatible GPU is available.");
                eprintln!("The example will exit, but this is expected on systems without GPU support.");
            }
        }
    }

    #[cfg(not(feature = "wgpu-backend"))]
    {
        eprintln!("GPU example requires 'wgpu-backend' feature.");
        eprintln!("Compile with: cargo run --example gpu_rendering --features wgpu-backend");
    }

    println!("\nExample complete!");
}
