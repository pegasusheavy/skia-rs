//! Codec trait and format-specific implementations.
//!
//! Codecs handle encoding and decoding of images in various formats.

use crate::Image;
use std::io::{Read, Write};
use thiserror::Error;

/// Errors that can occur during codec operations.
#[derive(Debug, Error)]
pub enum CodecError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid or corrupt data.
    #[error("invalid data: {0}")]
    InvalidData(String),
    /// Unsupported format or feature.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// Encoding error.
    #[error("encoding error: {0}")]
    EncodingError(String),
    /// Decoding error.
    #[error("decoding error: {0}")]
    DecodingError(String),
}

/// Result type for codec operations.
pub type CodecResult<T> = Result<T, CodecError>;

/// Detected image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// PNG format.
    Png,
    /// JPEG format.
    Jpeg,
    /// GIF format.
    Gif,
    /// WebP format.
    WebP,
    /// BMP format.
    Bmp,
    /// ICO format.
    Ico,
    /// Unknown format.
    Unknown,
}

impl ImageFormat {
    /// Detect format from magic bytes.
    pub fn from_magic(data: &[u8]) -> Self {
        if data.len() < 8 {
            return Self::Unknown;
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Self::Png;
        }

        // JPEG: FF D8 FF
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Self::Jpeg;
        }

        // GIF: GIF87a or GIF89a
        if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            return Self::Gif;
        }

        // WebP: RIFF....WEBP
        if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
            return Self::WebP;
        }

        // BMP: BM
        if data.starts_with(b"BM") {
            return Self::Bmp;
        }

        // ICO: 00 00 01 00
        if data.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return Self::Ico;
        }

        Self::Unknown
    }

    /// Get the typical file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
            Self::Ico => "ico",
            Self::Unknown => "",
        }
    }

    /// Get the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
            Self::Bmp => "image/bmp",
            Self::Ico => "image/x-icon",
            Self::Unknown => "application/octet-stream",
        }
    }
}

/// A codec that can decode images.
pub trait ImageDecoder: Send + Sync {
    /// Decode an image from a reader.
    fn decode<R: Read>(&self, reader: R) -> CodecResult<Image>;

    /// Decode an image from bytes.
    fn decode_bytes(&self, data: &[u8]) -> CodecResult<Image> {
        self.decode(std::io::Cursor::new(data))
    }

    /// Get the format this decoder handles.
    fn format(&self) -> ImageFormat;

    /// Check if this decoder can handle the given data.
    fn can_decode(&self, data: &[u8]) -> bool {
        ImageFormat::from_magic(data) == self.format()
    }
}

/// A codec that can encode images.
pub trait ImageEncoder: Send + Sync {
    /// Encode an image to a writer.
    fn encode<W: Write>(&self, image: &Image, writer: W) -> CodecResult<()>;

    /// Encode an image to bytes.
    fn encode_bytes(&self, image: &Image) -> CodecResult<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(image, &mut buf)?;
        Ok(buf)
    }

    /// Get the format this encoder produces.
    fn format(&self) -> ImageFormat;
}

/// Quality setting for lossy encoders.
#[derive(Debug, Clone, Copy)]
pub struct EncoderQuality(u8);

impl EncoderQuality {
    /// Create a quality setting (0-100).
    pub fn new(quality: u8) -> Self {
        Self(quality.min(100))
    }

    /// Get the quality value.
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Default quality (80).
    pub const DEFAULT: Self = Self(80);

    /// High quality (95).
    pub const HIGH: Self = Self(95);

    /// Low quality (50).
    pub const LOW: Self = Self(50);
}

impl Default for EncoderQuality {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// =============================================================================
// Simple PNG Codec (stub - would use png crate for real implementation)
// =============================================================================

/// PNG decoder.
#[derive(Debug, Default)]
pub struct PngDecoder;

impl PngDecoder {
    /// Create a new PNG decoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageDecoder for PngDecoder {
    #[cfg(feature = "png")]
    fn decode<R: Read>(&self, reader: R) -> CodecResult<Image> {
        let decoder = png::Decoder::new(reader);
        let mut png_reader = decoder.read_info().map_err(|e| CodecError::DecodingError(e.to_string()))?;

        let mut buf = vec![0; png_reader.output_buffer_size()];
        let info = png_reader.next_frame(&mut buf).map_err(|e| CodecError::DecodingError(e.to_string()))?;

        let width = info.width as i32;
        let height = info.height as i32;

        // Convert to RGBA if necessary
        let pixels = match info.color_type {
            png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => {
                let rgb = &buf[..info.buffer_size()];
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                for chunk in rgb.chunks(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                rgba
            }
            png::ColorType::GrayscaleAlpha => {
                let ga = &buf[..info.buffer_size()];
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                for chunk in ga.chunks(2) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[0]);
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                }
                rgba
            }
            png::ColorType::Grayscale => {
                let gray = &buf[..info.buffer_size()];
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                for &g in gray {
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(255);
                }
                rgba
            }
            _ => return Err(CodecError::Unsupported("Unsupported PNG color type".into())),
        };

        let info = crate::ImageInfo::new(
            width,
            height,
            skia_rs_core::ColorType::Rgba8888,
            skia_rs_core::AlphaType::Unpremul,
        );

        Image::from_raster_data_owned(info, pixels, width as usize * 4)
            .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
    }

    #[cfg(not(feature = "png"))]
    fn decode<R: Read>(&self, _reader: R) -> CodecResult<Image> {
        Err(CodecError::Unsupported("PNG decoding requires the 'png' feature".into()))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Png
    }
}

/// PNG encoder.
#[derive(Debug, Default)]
pub struct PngEncoder;

impl PngEncoder {
    /// Create a new PNG encoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageEncoder for PngEncoder {
    #[cfg(feature = "png")]
    fn encode<W: Write>(&self, image: &Image, writer: W) -> CodecResult<()> {
        let mut encoder = png::Encoder::new(writer, image.width() as u32, image.height() as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut png_writer = encoder.write_header().map_err(|e| CodecError::EncodingError(e.to_string()))?;

        let pixels = image.peek_pixels().ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;

        // Convert to RGBA if necessary based on color type
        let rgba_data = match image.color_type() {
            skia_rs_core::ColorType::Rgba8888 => pixels.to_vec(),
            skia_rs_core::ColorType::Bgra8888 => {
                let mut rgba = Vec::with_capacity(pixels.len());
                for chunk in pixels.chunks(4) {
                    rgba.push(chunk[2]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[0]); // B
                    rgba.push(chunk[3]); // A
                }
                rgba
            }
            _ => return Err(CodecError::Unsupported("Unsupported color type for PNG encoding".into())),
        };

        png_writer.write_image_data(&rgba_data).map_err(|e| CodecError::EncodingError(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(feature = "png"))]
    fn encode<W: Write>(&self, _image: &Image, _writer: W) -> CodecResult<()> {
        Err(CodecError::Unsupported("PNG encoding requires the 'png' feature".into()))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Png
    }
}

// =============================================================================
// JPEG Codec (stub)
// =============================================================================

/// JPEG decoder.
#[derive(Debug, Default)]
pub struct JpegDecoder;

impl JpegDecoder {
    /// Create a new JPEG decoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageDecoder for JpegDecoder {
    #[cfg(feature = "jpeg")]
    fn decode<R: Read>(&self, reader: R) -> CodecResult<Image> {
        let mut decoder = jpeg_decoder::Decoder::new(reader);
        let pixels = decoder.decode().map_err(|e| CodecError::DecodingError(e.to_string()))?;
        let info = decoder.info().ok_or_else(|| CodecError::DecodingError("No image info".into()))?;

        let width = info.width as i32;
        let height = info.height as i32;

        // Convert to RGBA
        let rgba = match info.pixel_format {
            jpeg_decoder::PixelFormat::RGB24 => {
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                for chunk in pixels.chunks(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                rgba
            }
            jpeg_decoder::PixelFormat::L8 => {
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                for &g in &pixels {
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(255);
                }
                rgba
            }
            _ => return Err(CodecError::Unsupported("Unsupported JPEG pixel format".into())),
        };

        let img_info = crate::ImageInfo::new(
            width,
            height,
            skia_rs_core::ColorType::Rgba8888,
            skia_rs_core::AlphaType::Opaque,
        );

        Image::from_raster_data_owned(img_info, rgba, width as usize * 4)
            .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
    }

    #[cfg(not(feature = "jpeg"))]
    fn decode<R: Read>(&self, _reader: R) -> CodecResult<Image> {
        Err(CodecError::Unsupported("JPEG decoding requires the 'jpeg' feature".into()))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Jpeg
    }
}

/// JPEG encoder.
#[derive(Debug)]
pub struct JpegEncoder {
    quality: EncoderQuality,
}

impl JpegEncoder {
    /// Create a new JPEG encoder with default quality.
    pub fn new() -> Self {
        Self {
            quality: EncoderQuality::DEFAULT,
        }
    }

    /// Create a JPEG encoder with specified quality.
    pub fn with_quality(quality: EncoderQuality) -> Self {
        Self { quality }
    }

    /// Get the quality setting.
    pub fn quality(&self) -> EncoderQuality {
        self.quality
    }
}

impl Default for JpegEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageEncoder for JpegEncoder {
    #[cfg(feature = "jpeg")]
    fn encode<W: Write>(&self, image: &Image, mut writer: W) -> CodecResult<()> {
        let pixels = image.peek_pixels().ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;

        // Convert to RGB (JPEG doesn't support alpha)
        let rgb = match image.color_type() {
            skia_rs_core::ColorType::Rgba8888 => {
                let mut rgb = Vec::with_capacity((image.width() * image.height() * 3) as usize);
                for chunk in pixels.chunks(4) {
                    rgb.push(chunk[0]);
                    rgb.push(chunk[1]);
                    rgb.push(chunk[2]);
                }
                rgb
            }
            skia_rs_core::ColorType::Bgra8888 => {
                let mut rgb = Vec::with_capacity((image.width() * image.height() * 3) as usize);
                for chunk in pixels.chunks(4) {
                    rgb.push(chunk[2]); // R
                    rgb.push(chunk[1]); // G
                    rgb.push(chunk[0]); // B
                }
                rgb
            }
            _ => return Err(CodecError::Unsupported("Unsupported color type for JPEG encoding".into())),
        };

        let encoder = jpeg_encoder::Encoder::new(&mut writer, self.quality.value());
        encoder.encode(
            &rgb,
            image.width() as u16,
            image.height() as u16,
            jpeg_encoder::ColorType::Rgb,
        ).map_err(|e| CodecError::EncodingError(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(feature = "jpeg"))]
    fn encode<W: Write>(&self, _image: &Image, _writer: W) -> CodecResult<()> {
        Err(CodecError::Unsupported("JPEG encoding requires the 'jpeg' feature".into()))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Jpeg
    }
}

// =============================================================================
// GIF Codec
// =============================================================================

/// GIF decoder.
#[derive(Debug, Default)]
pub struct GifDecoder;

impl GifDecoder {
    /// Create a new GIF decoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageDecoder for GifDecoder {
    #[cfg(feature = "gif")]
    fn decode<R: Read>(&self, reader: R) -> CodecResult<Image> {
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(reader).map_err(|e| CodecError::DecodingError(e.to_string()))?;

        let first_frame = decoder.read_next_frame()
            .map_err(|e| CodecError::DecodingError(e.to_string()))?
            .ok_or_else(|| CodecError::DecodingError("No frames in GIF".into()))?;

        let width = first_frame.width as i32;
        let height = first_frame.height as i32;
        let pixels = first_frame.buffer.to_vec();

        let info = crate::ImageInfo::new(
            width,
            height,
            skia_rs_core::ColorType::Rgba8888,
            skia_rs_core::AlphaType::Unpremul,
        );

        Image::from_raster_data_owned(info, pixels, width as usize * 4)
            .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
    }

    #[cfg(not(feature = "gif"))]
    fn decode<R: Read>(&self, _reader: R) -> CodecResult<Image> {
        Err(CodecError::Unsupported("GIF decoding requires the 'gif' feature".into()))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Gif
    }
}

// =============================================================================
// WebP Codec
// =============================================================================

/// WebP decoder.
#[derive(Debug, Default)]
pub struct WebpDecoder;

impl WebpDecoder {
    /// Create a new WebP decoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageDecoder for WebpDecoder {
    #[cfg(feature = "webp")]
    fn decode<R: Read>(&self, mut reader: R) -> CodecResult<Image> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(|e| CodecError::Io(e))?;

        let decoder = webp::Decoder::new(&data);
        let webp_image = decoder.decode()
            .ok_or_else(|| CodecError::DecodingError("Failed to decode WebP".into()))?;

        let width = webp_image.width() as i32;
        let height = webp_image.height() as i32;

        // WebP returns RGBA
        let pixels = webp_image.to_vec();

        let info = crate::ImageInfo::new(
            width,
            height,
            skia_rs_core::ColorType::Rgba8888,
            skia_rs_core::AlphaType::Unpremul,
        );

        Image::from_raster_data_owned(info, pixels, width as usize * 4)
            .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
    }

    #[cfg(not(feature = "webp"))]
    fn decode<R: Read>(&self, _reader: R) -> CodecResult<Image> {
        Err(CodecError::Unsupported("WebP decoding requires the 'webp' feature".into()))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::WebP
    }
}

/// WebP encoder.
#[derive(Debug)]
pub struct WebpEncoder {
    quality: EncoderQuality,
    lossless: bool,
}

impl WebpEncoder {
    /// Create a new WebP encoder with default quality.
    pub fn new() -> Self {
        Self {
            quality: EncoderQuality::DEFAULT,
            lossless: false,
        }
    }

    /// Create a WebP encoder with specified quality.
    pub fn with_quality(quality: EncoderQuality) -> Self {
        Self { quality, lossless: false }
    }

    /// Create a lossless WebP encoder.
    pub fn lossless() -> Self {
        Self {
            quality: EncoderQuality::DEFAULT,
            lossless: true,
        }
    }
}

impl Default for WebpEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageEncoder for WebpEncoder {
    #[cfg(feature = "webp")]
    fn encode<W: Write>(&self, image: &Image, mut writer: W) -> CodecResult<()> {
        let pixels = image.peek_pixels().ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;
        let width = image.width() as u32;
        let height = image.height() as u32;

        // Convert to RGBA if needed
        let rgba = match image.color_type() {
            skia_rs_core::ColorType::Rgba8888 => pixels.to_vec(),
            skia_rs_core::ColorType::Bgra8888 => {
                let mut rgba = Vec::with_capacity(pixels.len());
                for chunk in pixels.chunks(4) {
                    rgba.push(chunk[2]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[0]);
                    rgba.push(chunk[3]);
                }
                rgba
            }
            _ => return Err(CodecError::Unsupported("Unsupported color type for WebP encoding".into())),
        };

        let encoder = webp::Encoder::from_rgba(&rgba, width, height);
        let encoded = if self.lossless {
            encoder.encode_lossless()
        } else {
            encoder.encode(self.quality.value() as f32)
        };

        writer.write_all(&encoded).map_err(CodecError::Io)?;
        Ok(())
    }

    #[cfg(not(feature = "webp"))]
    fn encode<W: Write>(&self, _image: &Image, _writer: W) -> CodecResult<()> {
        Err(CodecError::Unsupported("WebP encoding requires the 'webp' feature".into()))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::WebP
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Decode an image from bytes, auto-detecting the format.
pub fn decode_image(data: &[u8]) -> CodecResult<Image> {
    let format = ImageFormat::from_magic(data);

    match format {
        ImageFormat::Png => PngDecoder::new().decode_bytes(data),
        ImageFormat::Jpeg => JpegDecoder::new().decode_bytes(data),
        ImageFormat::Gif => GifDecoder::new().decode_bytes(data),
        ImageFormat::WebP => WebpDecoder::new().decode_bytes(data),
        _ => Err(CodecError::Unsupported(format!("Format {:?} not supported", format))),
    }
}

/// Get the image dimensions without fully decoding.
pub fn get_image_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    let format = ImageFormat::from_magic(data);

    match format {
        ImageFormat::Png => get_png_dimensions(data),
        ImageFormat::Jpeg => get_jpeg_dimensions(data),
        _ => Err(CodecError::Unsupported(format!("Format {:?} not supported", format))),
    }
}

fn get_png_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    // PNG IHDR chunk starts at byte 8, width at offset 16, height at offset 20
    if data.len() < 24 {
        return Err(CodecError::InvalidData("PNG too short".into()));
    }

    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as i32;
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]) as i32;

    Ok((width, height))
}

fn get_jpeg_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    // JPEG dimensions are in SOF marker
    let mut i = 2;
    while i < data.len() - 8 {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = data[i + 1];

        // SOF0, SOF1, SOF2 markers contain dimensions
        if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 {
            let height = u16::from_be_bytes([data[i + 5], data[i + 6]]) as i32;
            let width = u16::from_be_bytes([data[i + 7], data[i + 8]]) as i32;
            return Ok((width, height));
        }

        // Skip other markers
        if marker >= 0xC0 {
            let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            i += 2 + len;
        } else {
            i += 1;
        }
    }

    Err(CodecError::InvalidData("Could not find JPEG dimensions".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        // PNG magic bytes
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(ImageFormat::from_magic(&png), ImageFormat::Png);

        // JPEG magic bytes (need at least 8 bytes for magic detection)
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(ImageFormat::from_magic(&jpeg), ImageFormat::Jpeg);

        // GIF magic bytes (need at least 8 bytes)
        let gif = b"GIF89a\x00\x00";
        assert_eq!(ImageFormat::from_magic(gif), ImageFormat::Gif);

        // WebP magic bytes
        let webp = b"RIFF\x00\x00\x00\x00WEBP";
        assert_eq!(ImageFormat::from_magic(webp), ImageFormat::WebP);

        // Unknown (need at least 8 bytes to test)
        let unknown = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(ImageFormat::from_magic(&unknown), ImageFormat::Unknown);
    }

    #[test]
    fn test_format_extensions() {
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Gif.extension(), "gif");
    }

    #[test]
    fn test_format_mime_types() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
    }

    #[test]
    fn test_encoder_quality() {
        let q = EncoderQuality::new(75);
        assert_eq!(q.value(), 75);

        let q_over = EncoderQuality::new(150);
        assert_eq!(q_over.value(), 100);
    }
}
