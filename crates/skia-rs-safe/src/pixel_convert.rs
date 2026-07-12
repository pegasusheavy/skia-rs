//! Pixel-buffer conversion helpers shared by platform-specific bindings.
//!
//! Not gated to any particular target so the pure conversion logic can be
//! unit-tested on the host, even though today it is only consumed by the
//! `wasm32` bindings (see `wasm.rs`).

/// Convert premultiplied RGBA8888 bytes — as stored by
/// [`skia_rs_canvas::Surface`] (surfaces created via
/// `Surface::new_raster_n32_premul` hold **premultiplied** pixels in
/// **RGBA** byte order) — into the byte layout the web `ImageData`
/// constructor expects: **unpremultiplied**, straight-alpha RGBA.
///
/// No channel reordering is performed. `ImageData` and skia-rs surfaces
/// both use RGBA byte order; a prior revision of the WASM binding swapped
/// bytes 0 and 2 under the mistaken assumption the surface was BGRA, which
/// silently swapped the red and blue channels of every pixel handed to the
/// browser.
pub fn premul_rgba_to_image_data(pixels: &[u8]) -> Vec<u8> {
    let mut out = pixels.to_vec();
    skia_rs_canvas::simd::unpremultiply_span(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_channel_swap_opaque_pixel() {
        // Opaque red (alpha = 255): premultiplication is a no-op, so this
        // also exercises "no R/B swap" — if the old BGRA-swap logic were
        // still present, byte 0 (R) and byte 2 (B) would be exchanged.
        let px = [0xFF, 0x00, 0x00, 0xFF]; // R=255 G=0 B=0 A=255
        let out = premul_rgba_to_image_data(&px);
        assert_eq!(out, vec![0xFF, 0x00, 0x00, 0xFF]);
    }

    #[test]
    fn unpremultiplies_semi_transparent_pixel() {
        // Premultiplied 50%-alpha red: stored R is halved (128), full
        // alpha would be 255 unpremultiplied.
        let px = [128, 0, 0, 128];
        let out = premul_rgba_to_image_data(&px);
        // unpremultiply_span: r = (128 * 255) / 128 = 255
        assert_eq!(out[0], 255);
        assert_eq!(out[1], 0);
        assert_eq!(out[2], 0);
        assert_eq!(out[3], 128); // alpha unchanged
    }

    #[test]
    fn fully_transparent_pixel_passes_through_zeroed() {
        let px = [0, 0, 0, 0];
        let out = premul_rgba_to_image_data(&px);
        assert_eq!(out, vec![0, 0, 0, 0]);
    }

    #[test]
    fn preserves_pixel_count_for_multi_pixel_buffer() {
        // R, G, B, W (opaque) laid out RGBA — verifies ordering is
        // preserved across multiple pixels, not just swapped within one.
        let px = [
            0xFF, 0x00, 0x00, 0xFF, // red
            0x00, 0xFF, 0x00, 0xFF, // green
            0x00, 0x00, 0xFF, 0xFF, // blue
        ];
        let out = premul_rgba_to_image_data(&px);
        assert_eq!(out, px.to_vec());
    }
}
