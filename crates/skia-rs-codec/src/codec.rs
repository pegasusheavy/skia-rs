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
    /// WBMP format (Wireless Bitmap).
    Wbmp,
    /// AVIF format (AV1 Image File Format).
    Avif,
    /// Camera RAW format.
    Raw,
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

        // AVIF: ftyp box with 'avif' or 'avis' brand
        // HEIF/AVIF files start with ftyp box: size (4 bytes) + "ftyp" + brand
        if data.len() >= 12 && &data[4..8] == b"ftyp" {
            // Check for AVIF brands
            if &data[8..12] == b"avif" || &data[8..12] == b"avis" || &data[8..12] == b"mif1" {
                return Self::Avif;
            }
        }

        // WBMP: Type 0, FixHeaderField 0, followed by width/height
        // First two bytes are 0, third and fourth bytes encode width and height
        // Both dimensions must be non-zero for a valid WBMP
        if data.len() >= 4 && data[0] == 0 && data[1] == 0 {
            // Check if it looks like valid WBMP multibyte integers
            // WBMP uses variable-length integers where bit 7 indicates continuation
            // Both width and height must be non-zero
            if data[2] < 128 && data[2] > 0 && data[3] < 128 && data[3] > 0 {
                return Self::Wbmp;
            }
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
            Self::Wbmp => "wbmp",
            Self::Avif => "avif",
            Self::Raw => "raw",
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
            Self::Wbmp => "image/vnd.wap.wbmp",
            Self::Avif => "image/avif",
            Self::Raw => "image/x-raw",
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
        let mut png_reader = decoder
            .read_info()
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;

        let mut buf = vec![0; png_reader.output_buffer_size()];
        let info = png_reader
            .next_frame(&mut buf)
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;

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
        Err(CodecError::Unsupported(
            "PNG decoding requires the 'png' feature".into(),
        ))
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

        let mut png_writer = encoder
            .write_header()
            .map_err(|e| CodecError::EncodingError(e.to_string()))?;

        let pixels = image
            .peek_pixels()
            .ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;

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
            _ => {
                return Err(CodecError::Unsupported(
                    "Unsupported color type for PNG encoding".into(),
                ));
            }
        };

        png_writer
            .write_image_data(&rgba_data)
            .map_err(|e| CodecError::EncodingError(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(feature = "png"))]
    fn encode<W: Write>(&self, _image: &Image, _writer: W) -> CodecResult<()> {
        Err(CodecError::Unsupported(
            "PNG encoding requires the 'png' feature".into(),
        ))
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
        let pixels = decoder
            .decode()
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;
        let info = decoder
            .info()
            .ok_or_else(|| CodecError::DecodingError("No image info".into()))?;

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
            _ => {
                return Err(CodecError::Unsupported(
                    "Unsupported JPEG pixel format".into(),
                ));
            }
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
        Err(CodecError::Unsupported(
            "JPEG decoding requires the 'jpeg' feature".into(),
        ))
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
        let pixels = image
            .peek_pixels()
            .ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;

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
            _ => {
                return Err(CodecError::Unsupported(
                    "Unsupported color type for JPEG encoding".into(),
                ));
            }
        };

        let encoder = jpeg_encoder::Encoder::new(&mut writer, self.quality.value());
        encoder
            .encode(
                &rgb,
                image.width() as u16,
                image.height() as u16,
                jpeg_encoder::ColorType::Rgb,
            )
            .map_err(|e| CodecError::EncodingError(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(feature = "jpeg"))]
    fn encode<W: Write>(&self, _image: &Image, _writer: W) -> CodecResult<()> {
        Err(CodecError::Unsupported(
            "JPEG encoding requires the 'jpeg' feature".into(),
        ))
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
        let mut decoder = decoder
            .read_info(reader)
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;

        let first_frame = decoder
            .read_next_frame()
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
        Err(CodecError::Unsupported(
            "GIF decoding requires the 'gif' feature".into(),
        ))
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
        reader
            .read_to_end(&mut data)
            .map_err(|e| CodecError::Io(e))?;

        let decoder = webp::Decoder::new(&data);
        let webp_image = decoder
            .decode()
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
        Err(CodecError::Unsupported(
            "WebP decoding requires the 'webp' feature".into(),
        ))
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
        Self {
            quality,
            lossless: false,
        }
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
        let pixels = image
            .peek_pixels()
            .ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;
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
            _ => {
                return Err(CodecError::Unsupported(
                    "Unsupported color type for WebP encoding".into(),
                ));
            }
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
        Err(CodecError::Unsupported(
            "WebP encoding requires the 'webp' feature".into(),
        ))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::WebP
    }
}

// =============================================================================
// BMP Codec
// =============================================================================

/// BMP decoder.
#[derive(Debug, Default)]
pub struct BmpDecoder;

impl BmpDecoder {
    /// Create a new BMP decoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageDecoder for BmpDecoder {
    fn decode<R: Read>(&self, mut reader: R) -> CodecResult<Image> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(CodecError::Io)?;
        decode_bmp(&data)
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Bmp
    }
}

/// BMP encoder.
#[derive(Debug, Default)]
pub struct BmpEncoder;

impl BmpEncoder {
    /// Create a new BMP encoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageEncoder for BmpEncoder {
    fn encode<W: Write>(&self, image: &Image, mut writer: W) -> CodecResult<()> {
        let pixels = image
            .peek_pixels()
            .ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;

        let width = image.width() as u32;
        let height = image.height() as u32;

        // Calculate row padding (each row must be aligned to 4 bytes)
        let row_size = (width * 4 + 3) & !3; // 32-bit BGRA, aligned
        let pixel_data_size = row_size * height;
        let file_size = 14 + 40 + pixel_data_size; // header + DIB header + pixels

        // BITMAPFILEHEADER (14 bytes)
        writer.write_all(b"BM")?; // Signature
        writer.write_all(&(file_size as u32).to_le_bytes())?; // File size
        writer.write_all(&[0u8; 4])?; // Reserved
        writer.write_all(&(14u32 + 40).to_le_bytes())?; // Pixel data offset

        // BITMAPINFOHEADER (40 bytes)
        writer.write_all(&40u32.to_le_bytes())?; // Header size
        writer.write_all(&(width as i32).to_le_bytes())?; // Width
        writer.write_all(&(height as i32).to_le_bytes())?; // Height (positive = bottom-up)
        writer.write_all(&1u16.to_le_bytes())?; // Planes
        writer.write_all(&32u16.to_le_bytes())?; // Bits per pixel (32-bit BGRA)
        writer.write_all(&0u32.to_le_bytes())?; // Compression (BI_RGB = none)
        writer.write_all(&pixel_data_size.to_le_bytes())?; // Image size
        writer.write_all(&2835u32.to_le_bytes())?; // X pixels per meter (~72 DPI)
        writer.write_all(&2835u32.to_le_bytes())?; // Y pixels per meter
        writer.write_all(&0u32.to_le_bytes())?; // Colors used
        writer.write_all(&0u32.to_le_bytes())?; // Important colors

        // Pixel data (bottom-to-top, BGRA)
        let stride = width as usize * 4;
        let row_padding = row_size as usize - stride;
        let padding = vec![0u8; row_padding];

        for y in (0..height as usize).rev() {
            let row_start = y * stride;
            let row = &pixels[row_start..row_start + stride];

            // Convert RGBA to BGRA
            for chunk in row.chunks(4) {
                writer.write_all(&[chunk[2], chunk[1], chunk[0], chunk[3]])?; // BGRA
            }

            if row_padding > 0 {
                writer.write_all(&padding)?;
            }
        }

        Ok(())
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Bmp
    }
}

/// Decode a BMP image from bytes.
fn decode_bmp(data: &[u8]) -> CodecResult<Image> {
    if data.len() < 54 {
        return Err(CodecError::InvalidData("BMP too short".into()));
    }

    // Validate signature
    if &data[0..2] != b"BM" {
        return Err(CodecError::InvalidData("Invalid BMP signature".into()));
    }

    // Parse BITMAPFILEHEADER
    let pixel_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;

    // Parse BITMAPINFOHEADER
    let header_size = u32::from_le_bytes([data[14], data[15], data[16], data[17]]);
    if header_size < 40 {
        return Err(CodecError::Unsupported(
            "Only BITMAPINFOHEADER and later supported".into(),
        ));
    }

    let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bits_per_pixel = u16::from_le_bytes([data[28], data[29]]);
    let compression = u32::from_le_bytes([data[30], data[31], data[32], data[33]]);

    if width <= 0 {
        return Err(CodecError::InvalidData("Invalid BMP width".into()));
    }

    let width = width as usize;
    let (height, bottom_up) = if height < 0 {
        ((-height) as usize, false)
    } else {
        (height as usize, true)
    };

    // Only support uncompressed and basic RLE for now
    if compression != 0 && compression != 3 {
        return Err(CodecError::Unsupported(format!(
            "BMP compression {} not supported",
            compression
        )));
    }

    if pixel_offset >= data.len() {
        return Err(CodecError::InvalidData("Invalid pixel data offset".into()));
    }

    let pixel_data = &data[pixel_offset..];
    let mut rgba = vec![0u8; width * height * 4];

    match bits_per_pixel {
        24 => {
            // 24-bit BGR
            let row_size = (width * 3 + 3) & !3;
            for y in 0..height {
                let src_y = if bottom_up { height - 1 - y } else { y };
                let src_row = src_y * row_size;
                let dst_row = y * width * 4;

                for x in 0..width {
                    let src = src_row + x * 3;
                    let dst = dst_row + x * 4;
                    if src + 2 < pixel_data.len() {
                        rgba[dst] = pixel_data[src + 2]; // R
                        rgba[dst + 1] = pixel_data[src + 1]; // G
                        rgba[dst + 2] = pixel_data[src]; // B
                        rgba[dst + 3] = 255; // A
                    }
                }
            }
        }
        32 => {
            // 32-bit BGRA or BGRX
            let row_size = width * 4;
            for y in 0..height {
                let src_y = if bottom_up { height - 1 - y } else { y };
                let src_row = src_y * row_size;
                let dst_row = y * width * 4;

                for x in 0..width {
                    let src = src_row + x * 4;
                    let dst = dst_row + x * 4;
                    if src + 3 < pixel_data.len() {
                        rgba[dst] = pixel_data[src + 2]; // R
                        rgba[dst + 1] = pixel_data[src + 1]; // G
                        rgba[dst + 2] = pixel_data[src]; // B
                        rgba[dst + 3] = pixel_data[src + 3]; // A
                    }
                }
            }
        }
        8 => {
            // 8-bit indexed (need to read color table)
            let colors_used = u32::from_le_bytes([data[46], data[47], data[48], data[49]]) as usize;
            let palette_size = if colors_used == 0 { 256 } else { colors_used };
            let palette_offset = 14 + header_size as usize;

            if palette_offset + palette_size * 4 > pixel_offset {
                return Err(CodecError::InvalidData("Invalid palette".into()));
            }

            let palette = &data[palette_offset..palette_offset + palette_size * 4];
            let row_size = (width + 3) & !3;

            for y in 0..height {
                let src_y = if bottom_up { height - 1 - y } else { y };
                let src_row = src_y * row_size;
                let dst_row = y * width * 4;

                for x in 0..width {
                    let src = src_row + x;
                    let dst = dst_row + x * 4;
                    if src < pixel_data.len() {
                        let index = pixel_data[src] as usize;
                        if index < palette_size {
                            let p = index * 4;
                            rgba[dst] = palette[p + 2]; // R
                            rgba[dst + 1] = palette[p + 1]; // G
                            rgba[dst + 2] = palette[p]; // B
                            rgba[dst + 3] = 255; // A
                        }
                    }
                }
            }
        }
        _ => {
            return Err(CodecError::Unsupported(format!(
                "BMP {} bits per pixel not supported",
                bits_per_pixel
            )));
        }
    }

    let info = crate::ImageInfo::new(
        width as i32,
        height as i32,
        skia_rs_core::ColorType::Rgba8888,
        skia_rs_core::AlphaType::Unpremul,
    );

    Image::from_raster_data_owned(info, rgba, width * 4)
        .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
}

// =============================================================================
// ICO Codec
// =============================================================================

/// ICO decoder.
#[derive(Debug, Default)]
pub struct IcoDecoder;

impl IcoDecoder {
    /// Create a new ICO decoder.
    pub fn new() -> Self {
        Self
    }
}

impl ImageDecoder for IcoDecoder {
    fn decode<R: Read>(&self, mut reader: R) -> CodecResult<Image> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(CodecError::Io)?;
        decode_ico(&data)
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Ico
    }
}

/// Decode an ICO image from bytes.
/// Returns the largest image in the icon file.
fn decode_ico(data: &[u8]) -> CodecResult<Image> {
    if data.len() < 6 {
        return Err(CodecError::InvalidData("ICO too short".into()));
    }

    // Validate header
    let reserved = u16::from_le_bytes([data[0], data[1]]);
    let image_type = u16::from_le_bytes([data[2], data[3]]);
    let image_count = u16::from_le_bytes([data[4], data[5]]) as usize;

    if reserved != 0 || image_type != 1 {
        return Err(CodecError::InvalidData("Invalid ICO header".into()));
    }

    if image_count == 0 {
        return Err(CodecError::InvalidData("ICO has no images".into()));
    }

    // Find the largest image
    let mut best_index = 0;
    let mut best_size = 0u32;

    for i in 0..image_count {
        let entry_offset = 6 + i * 16;
        if entry_offset + 16 > data.len() {
            break;
        }

        let width = if data[entry_offset] == 0 {
            256
        } else {
            data[entry_offset] as u32
        };
        let height = if data[entry_offset + 1] == 0 {
            256
        } else {
            data[entry_offset + 1] as u32
        };

        let size = width * height;
        if size > best_size {
            best_size = size;
            best_index = i;
        }
    }

    // Get the best entry
    let entry_offset = 6 + best_index * 16;
    let image_offset = u32::from_le_bytes([
        data[entry_offset + 12],
        data[entry_offset + 13],
        data[entry_offset + 14],
        data[entry_offset + 15],
    ]) as usize;
    let image_size = u32::from_le_bytes([
        data[entry_offset + 8],
        data[entry_offset + 9],
        data[entry_offset + 10],
        data[entry_offset + 11],
    ]) as usize;

    if image_offset + image_size > data.len() {
        return Err(CodecError::InvalidData("Invalid ICO image data".into()));
    }

    let image_data = &data[image_offset..image_offset + image_size];

    // Check if it's PNG or BMP
    if image_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        // PNG encoded
        PngDecoder::new().decode_bytes(image_data)
    } else {
        // BMP encoded (without file header)
        decode_ico_bmp(image_data)
    }
}

/// Decode a BMP from ICO format (no BITMAPFILEHEADER).
fn decode_ico_bmp(data: &[u8]) -> CodecResult<Image> {
    if data.len() < 40 {
        return Err(CodecError::InvalidData("ICO BMP too short".into()));
    }

    let header_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if header_size < 40 {
        return Err(CodecError::Unsupported("Invalid ICO BMP header".into()));
    }

    let width = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let height = i32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize / 2; // Height includes mask
    let bits_per_pixel = u16::from_le_bytes([data[14], data[15]]);

    let pixel_offset = header_size as usize;
    let pixel_data = &data[pixel_offset..];
    let mut rgba = vec![0u8; width * height * 4];

    match bits_per_pixel {
        32 => {
            // 32-bit BGRA
            let row_size = width * 4;
            for y in 0..height {
                let src_y = height - 1 - y; // Bottom-up
                let src_row = src_y * row_size;
                let dst_row = y * width * 4;

                for x in 0..width {
                    let src = src_row + x * 4;
                    let dst = dst_row + x * 4;
                    if src + 3 < pixel_data.len() {
                        rgba[dst] = pixel_data[src + 2]; // R
                        rgba[dst + 1] = pixel_data[src + 1]; // G
                        rgba[dst + 2] = pixel_data[src]; // B
                        rgba[dst + 3] = pixel_data[src + 3]; // A
                    }
                }
            }
        }
        24 => {
            // 24-bit BGR with separate alpha mask
            let row_size = (width * 3 + 3) & !3;
            let mask_row_size = (width + 31) / 32 * 4;
            let mask_offset = height * row_size;

            for y in 0..height {
                let src_y = height - 1 - y;
                let src_row = src_y * row_size;
                let dst_row = y * width * 4;
                let mask_row = mask_offset + src_y * mask_row_size;

                for x in 0..width {
                    let src = src_row + x * 3;
                    let dst = dst_row + x * 4;
                    if src + 2 < pixel_data.len() {
                        rgba[dst] = pixel_data[src + 2]; // R
                        rgba[dst + 1] = pixel_data[src + 1]; // G
                        rgba[dst + 2] = pixel_data[src]; // B

                        // Check mask
                        let mask_byte = mask_row + x / 8;
                        let mask_bit = 7 - (x % 8);
                        let alpha = if mask_byte < pixel_data.len() {
                            if (pixel_data[mask_byte] >> mask_bit) & 1 == 0 {
                                255
                            } else {
                                0
                            }
                        } else {
                            255
                        };
                        rgba[dst + 3] = alpha;
                    }
                }
            }
        }
        _ => {
            return Err(CodecError::Unsupported(format!(
                "ICO {} bits per pixel not supported",
                bits_per_pixel
            )));
        }
    }

    let info = crate::ImageInfo::new(
        width as i32,
        height as i32,
        skia_rs_core::ColorType::Rgba8888,
        skia_rs_core::AlphaType::Unpremul,
    );

    Image::from_raster_data_owned(info, rgba, width * 4)
        .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
}

// =============================================================================
// WBMP Codec (Wireless Bitmap)
// =============================================================================

/// WBMP decoder.
///
/// WBMP is a simple monochrome image format used in WAP applications.
pub struct WbmpDecoder;

impl WbmpDecoder {
    /// Create a new WBMP decoder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WbmpDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageDecoder for WbmpDecoder {
    fn decode<R: Read>(&self, mut reader: R) -> CodecResult<Image> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(CodecError::Io)?;
        decode_wbmp(&data)
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Wbmp
    }
}

/// WBMP encoder.
pub struct WbmpEncoder;

impl WbmpEncoder {
    /// Create a new WBMP encoder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WbmpEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageEncoder for WbmpEncoder {
    fn encode<W: Write>(&self, image: &Image, mut writer: W) -> CodecResult<()> {
        encode_wbmp(image, &mut writer)
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Wbmp
    }
}

/// Read a WBMP multi-byte integer.
/// Each byte has 7 bits of data, MSB indicates if more bytes follow.
fn read_wbmp_int(data: &[u8], offset: &mut usize) -> Option<u32> {
    let mut value = 0u32;
    loop {
        if *offset >= data.len() {
            return None;
        }
        let byte = data[*offset];
        *offset += 1;
        value = (value << 7) | (byte & 0x7F) as u32;
        if byte & 0x80 == 0 {
            break;
        }
        if value > 0xFFFF {
            return None; // Sanity check
        }
    }
    Some(value)
}

/// Write a WBMP multi-byte integer.
fn write_wbmp_int<W: Write>(writer: &mut W, mut value: u32) -> CodecResult<()> {
    let mut bytes = Vec::new();
    bytes.push((value & 0x7F) as u8);
    value >>= 7;
    while value > 0 {
        bytes.push(0x80 | (value & 0x7F) as u8);
        value >>= 7;
    }
    bytes.reverse();
    writer.write_all(&bytes).map_err(CodecError::Io)
}

/// Decode a WBMP image from bytes.
fn decode_wbmp(data: &[u8]) -> CodecResult<Image> {
    if data.len() < 4 {
        return Err(CodecError::InvalidData("WBMP too short".into()));
    }

    let mut offset = 0;

    // Read type (should be 0 for WBMP type 0)
    let type_field = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP type".into()))?;
    if type_field != 0 {
        return Err(CodecError::Unsupported(format!(
            "WBMP type {} not supported",
            type_field
        )));
    }

    // Read fixed header field (should be 0)
    let fix_header = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP header".into()))?;
    if fix_header != 0 {
        return Err(CodecError::Unsupported(
            "Extended WBMP headers not supported".into(),
        ));
    }

    // Read width
    let width = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP width".into()))?;

    // Read height
    let height = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP height".into()))?;

    if width == 0 || height == 0 || width > 10000 || height > 10000 {
        return Err(CodecError::InvalidData("Invalid WBMP dimensions".into()));
    }

    // Calculate row size (bits rounded up to bytes)
    let row_bytes = (width as usize + 7) / 8;
    let expected_size = offset + row_bytes * height as usize;

    if data.len() < expected_size {
        return Err(CodecError::InvalidData("WBMP data too short".into()));
    }

    // Decode pixels (1 = white, 0 = black)
    let mut rgba = vec![0u8; width as usize * height as usize * 4];

    for y in 0..height as usize {
        let row_start = offset + y * row_bytes;
        for x in 0..width as usize {
            let byte_idx = x / 8;
            let bit_idx = 7 - (x % 8);
            let bit = (data[row_start + byte_idx] >> bit_idx) & 1;

            let pixel_idx = (y * width as usize + x) * 4;
            let value = if bit == 1 { 255 } else { 0 };
            rgba[pixel_idx] = value; // R
            rgba[pixel_idx + 1] = value; // G
            rgba[pixel_idx + 2] = value; // B
            rgba[pixel_idx + 3] = 255; // A
        }
    }

    let info = crate::ImageInfo::new(
        width as i32,
        height as i32,
        skia_rs_core::ColorType::Rgba8888,
        skia_rs_core::AlphaType::Opaque,
    );

    Image::from_raster_data_owned(info, rgba, width as usize * 4)
        .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
}

/// Encode an image as WBMP.
fn encode_wbmp<W: Write>(image: &Image, writer: &mut W) -> CodecResult<()> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let pixels = image
        .peek_pixels()
        .ok_or_else(|| CodecError::EncodingError("Cannot access pixels".into()))?;

    // Write type (0)
    write_wbmp_int(writer, 0)?;

    // Write fixed header field (0)
    write_wbmp_int(writer, 0)?;

    // Write width
    write_wbmp_int(writer, width)?;

    // Write height
    write_wbmp_int(writer, height)?;

    // Convert to 1-bit (use luminance threshold of 128)
    let row_bytes = (width as usize + 7) / 8;

    for y in 0..height as usize {
        let mut row_data = vec![0u8; row_bytes];

        for x in 0..width as usize {
            let pixel_idx = (y * width as usize + x) * 4;

            // Calculate luminance
            let r = pixels[pixel_idx] as u32;
            let g = pixels[pixel_idx + 1] as u32;
            let b = pixels[pixel_idx + 2] as u32;
            let luminance = (r * 299 + g * 587 + b * 114) / 1000;

            // Set bit if luminance > 128 (white)
            if luminance > 128 {
                let byte_idx = x / 8;
                let bit_idx = 7 - (x % 8);
                row_data[byte_idx] |= 1 << bit_idx;
            }
        }

        writer.write_all(&row_data).map_err(CodecError::Io)?;
    }

    Ok(())
}

/// Get WBMP dimensions without fully decoding.
fn get_wbmp_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    if data.len() < 4 {
        return Err(CodecError::InvalidData("WBMP too short".into()));
    }

    let mut offset = 0;

    // Skip type and header
    let _ = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP type".into()))?;
    let _ = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP header".into()))?;

    // Read dimensions
    let width = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP width".into()))?;
    let height = read_wbmp_int(data, &mut offset)
        .ok_or_else(|| CodecError::InvalidData("Invalid WBMP height".into()))?;

    Ok((width as i32, height as i32))
}

// =============================================================================
// AVIF Codec
// =============================================================================

/// AVIF decoder.
///
/// AVIF is a modern image format based on AV1 video codec, offering
/// excellent compression and quality.
pub struct AvifDecoder;

impl AvifDecoder {
    /// Create a new AVIF decoder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for AvifDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageDecoder for AvifDecoder {
    #[cfg(feature = "avif")]
    fn decode<R: Read>(&self, mut reader: R) -> CodecResult<Image> {
        use avif_decode::{Decoder, Image as AvifImage};

        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| CodecError::Io(e))?;

        let decoder = Decoder::from_avif(&data)
            .map_err(|e| CodecError::DecodingError(format!("AVIF decode error: {:?}", e)))?;

        let decoded = decoder
            .to_image()
            .map_err(|e| CodecError::DecodingError(format!("AVIF decode error: {:?}", e)))?;

        // Extract dimensions and pixels based on the image variant
        let (width, height, pixels) = match decoded {
            AvifImage::Rgb8(img) => {
                let w = img.width();
                let h = img.height();
                let rgba: Vec<u8> = img.pixels().flat_map(|p| [p.r, p.g, p.b, 255]).collect();
                (w as i32, h as i32, rgba)
            }
            AvifImage::Rgba8(img) => {
                let w = img.width();
                let h = img.height();
                let rgba: Vec<u8> = img.pixels().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
                (w as i32, h as i32, rgba)
            }
            AvifImage::Rgb16(img) => {
                let w = img.width();
                let h = img.height();
                // Convert 16-bit to 8-bit
                let rgba: Vec<u8> = img
                    .pixels()
                    .flat_map(|p| [(p.r >> 8) as u8, (p.g >> 8) as u8, (p.b >> 8) as u8, 255])
                    .collect();
                (w as i32, h as i32, rgba)
            }
            AvifImage::Rgba16(img) => {
                let w = img.width();
                let h = img.height();
                // Convert 16-bit to 8-bit
                let rgba: Vec<u8> = img
                    .pixels()
                    .flat_map(|p| {
                        [
                            (p.r >> 8) as u8,
                            (p.g >> 8) as u8,
                            (p.b >> 8) as u8,
                            (p.a >> 8) as u8,
                        ]
                    })
                    .collect();
                (w as i32, h as i32, rgba)
            }
            AvifImage::Gray8(img) => {
                let w = img.width();
                let h = img.height();
                let rgba: Vec<u8> = img
                    .pixels()
                    .flat_map(|g| {
                        let v = g.value();
                        [v, v, v, 255]
                    })
                    .collect();
                (w as i32, h as i32, rgba)
            }
            AvifImage::Gray16(img) => {
                let w = img.width();
                let h = img.height();
                let rgba: Vec<u8> = img
                    .pixels()
                    .flat_map(|g| {
                        let g8 = (g.value() >> 8) as u8;
                        [g8, g8, g8, 255]
                    })
                    .collect();
                (w as i32, h as i32, rgba)
            }
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

    #[cfg(not(feature = "avif"))]
    fn decode<R: Read>(&self, _reader: R) -> CodecResult<Image> {
        Err(CodecError::Unsupported(
            "AVIF decoding requires the 'avif' feature".into(),
        ))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Avif
    }
}

/// AVIF encoder.
///
/// Encodes images to AVIF format using the rav1e AV1 encoder.
pub struct AvifEncoder {
    quality: u8,
    speed: u8,
}

impl AvifEncoder {
    /// Create a new AVIF encoder with default settings.
    pub fn new() -> Self {
        Self {
            quality: 80,
            speed: 6, // Balance between speed and quality
        }
    }

    /// Set the quality (0-100, higher is better quality but larger file).
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality.min(100);
        self
    }

    /// Set the encoding speed (1-10, higher is faster but lower quality).
    pub fn with_speed(mut self, speed: u8) -> Self {
        self.speed = speed.clamp(1, 10);
        self
    }
}

impl Default for AvifEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageEncoder for AvifEncoder {
    #[cfg(feature = "avif")]
    fn encode<W: Write>(&self, image: &Image, mut writer: W) -> CodecResult<()> {
        use ravif::RGBA8;
        use ravif::{Encoder, Img};

        let width = image.width() as usize;
        let height = image.height() as usize;

        let pixels = image
            .peek_pixels()
            .ok_or_else(|| CodecError::EncodingError("Failed to access pixels".into()))?;

        // Convert to RGBA pixels for ravif
        let rgba_pixels: Vec<RGBA8> = pixels
            .chunks_exact(4)
            .map(|c| RGBA8::new(c[0], c[1], c[2], c[3]))
            .collect();

        let img = Img::new(rgba_pixels.as_slice(), width, height);

        let encoder = Encoder::new()
            .with_quality(self.quality as f32)
            .with_speed(self.speed);

        let result = encoder
            .encode_rgba(img)
            .map_err(|e| CodecError::EncodingError(format!("AVIF encode error: {:?}", e)))?;

        writer
            .write_all(&result.avif_file)
            .map_err(|e| CodecError::Io(e))?;

        Ok(())
    }

    #[cfg(not(feature = "avif"))]
    fn encode<W: Write>(&self, _image: &Image, _writer: W) -> CodecResult<()> {
        Err(CodecError::Unsupported(
            "AVIF encoding requires the 'avif' feature".into(),
        ))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Avif
    }
}

/// Get AVIF dimensions from data.
#[cfg(feature = "avif")]
fn get_avif_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    use avif_decode::{Decoder, Image as AvifImage};
    let decoder = Decoder::from_avif(data)
        .map_err(|e| CodecError::DecodingError(format!("AVIF decode error: {:?}", e)))?;
    let image = decoder
        .to_image()
        .map_err(|e| CodecError::DecodingError(format!("AVIF decode error: {:?}", e)))?;

    let (w, h) = match image {
        AvifImage::Rgb8(img) => (img.width(), img.height()),
        AvifImage::Rgba8(img) => (img.width(), img.height()),
        AvifImage::Rgb16(img) => (img.width(), img.height()),
        AvifImage::Rgba16(img) => (img.width(), img.height()),
        AvifImage::Gray8(img) => (img.width(), img.height()),
        AvifImage::Gray16(img) => (img.width(), img.height()),
    };
    Ok((w as i32, h as i32))
}

#[cfg(not(feature = "avif"))]
fn get_avif_dimensions(_data: &[u8]) -> CodecResult<(i32, i32)> {
    Err(CodecError::Unsupported(
        "AVIF support requires the 'avif' feature".into(),
    ))
}

// =============================================================================
// RAW Codec
// =============================================================================

/// Camera RAW decoder.
///
/// Decodes camera RAW formats from various manufacturers including
/// Canon, Nikon, Sony, Fuji, and many others.
pub struct RawDecoder;

impl RawDecoder {
    /// Create a new RAW decoder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RawDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageDecoder for RawDecoder {
    #[cfg(feature = "raw")]
    fn decode<R: Read>(&self, mut reader: R) -> CodecResult<Image> {
        use std::io::Cursor;

        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| CodecError::Io(e))?;

        let mut cursor = Cursor::new(&data);
        let raw_image = rawloader::decode(&mut cursor)
            .map_err(|e| CodecError::DecodingError(format!("RAW decode error: {:?}", e)))?;

        // Get dimensions
        let width = raw_image.width as i32;
        let height = raw_image.height as i32;

        // Convert RAW data to RGB
        // RAW images are typically in a Bayer pattern, we need to demosaic
        let pixels = demosaic_raw(&raw_image)?;

        let info = crate::ImageInfo::new(
            width,
            height,
            skia_rs_core::ColorType::Rgba8888,
            skia_rs_core::AlphaType::Opaque,
        );

        Image::from_raster_data_owned(info, pixels, width as usize * 4)
            .ok_or_else(|| CodecError::DecodingError("Failed to create image".into()))
    }

    #[cfg(not(feature = "raw"))]
    fn decode<R: Read>(&self, _reader: R) -> CodecResult<Image> {
        Err(CodecError::Unsupported(
            "RAW decoding requires the 'raw' feature".into(),
        ))
    }

    fn format(&self) -> ImageFormat {
        ImageFormat::Raw
    }
}

/// Demosaic a RAW image to RGBA.
#[cfg(feature = "raw")]
fn demosaic_raw(raw: &rawloader::RawImage) -> CodecResult<Vec<u8>> {
    let width = raw.width;
    let height = raw.height;
    let mut output = vec![0u8; width * height * 4];

    // Get the raw data
    let data = match &raw.data {
        rawloader::RawImageData::Integer(d) => d,
        rawloader::RawImageData::Float(_) => {
            return Err(CodecError::Unsupported(
                "Float RAW data not yet supported".into(),
            ));
        }
    };

    // Simple demosaicing - convert to grayscale for simplicity
    // A full implementation would do proper Bayer demosaicing
    let cfa = &raw.cfa;
    let black = raw.blacklevels[0] as f32;
    let white = raw.whitelevels[0] as f32;
    let range = if white > black {
        white - black
    } else {
        65535.0
    };

    // Color indices: 0=Red, 1=Green, 2=Blue
    const RED: usize = 0;
    const GREEN: usize = 1;
    const BLUE: usize = 2;

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let out_idx = idx * 4;

            // Get color index at this position (returns 0, 1, or 2)
            let color = cfa.color_at(y, x);

            // Get raw value normalized to 0-255
            let raw_val = data[idx] as f32;
            let normalized = ((raw_val - black) / range * 255.0).clamp(0.0, 255.0) as u8;

            // Simple nearest-neighbor for demo purposes
            match color {
                RED => {
                    output[out_idx] = normalized;
                    output[out_idx + 1] = get_neighbor_avg_by_color(
                        data, x, y, width, height, cfa, GREEN, black, range,
                    );
                    output[out_idx + 2] = get_neighbor_avg_by_color(
                        data, x, y, width, height, cfa, BLUE, black, range,
                    );
                }
                GREEN => {
                    output[out_idx] = get_neighbor_avg_by_color(
                        data, x, y, width, height, cfa, RED, black, range,
                    );
                    output[out_idx + 1] = normalized;
                    output[out_idx + 2] = get_neighbor_avg_by_color(
                        data, x, y, width, height, cfa, BLUE, black, range,
                    );
                }
                BLUE => {
                    output[out_idx] = get_neighbor_avg_by_color(
                        data, x, y, width, height, cfa, RED, black, range,
                    );
                    output[out_idx + 1] = get_neighbor_avg_by_color(
                        data, x, y, width, height, cfa, GREEN, black, range,
                    );
                    output[out_idx + 2] = normalized;
                }
                _ => {
                    // Unknown CFA pattern, just use grayscale
                    output[out_idx] = normalized;
                    output[out_idx + 1] = normalized;
                    output[out_idx + 2] = normalized;
                }
            }
            output[out_idx + 3] = 255; // Alpha
        }
    }

    Ok(output)
}

/// Get average of neighboring pixels of a specific color index.
#[cfg(feature = "raw")]
fn get_neighbor_avg_by_color(
    data: &[u16],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    cfa: &rawloader::CFA,
    target_color: usize,
    black: f32,
    range: f32,
) -> u8 {
    let mut sum = 0.0;
    let mut count = 0;

    // Check 3x3 neighborhood
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;

            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                let nx = nx as usize;
                let ny = ny as usize;

                if cfa.color_at(ny, nx) == target_color {
                    let idx = ny * width + nx;
                    let raw_val = data[idx] as f32;
                    sum += (raw_val - black) / range * 255.0;
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        (sum / count as f32).clamp(0.0, 255.0) as u8
    } else {
        128 // Fallback if no neighbors found
    }
}

/// Get RAW dimensions from data.
#[cfg(feature = "raw")]
fn get_raw_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    use std::io::Cursor;
    let mut cursor = Cursor::new(data);
    let raw = rawloader::decode(&mut cursor)
        .map_err(|e| CodecError::DecodingError(format!("RAW decode error: {:?}", e)))?;
    Ok((raw.width as i32, raw.height as i32))
}

#[cfg(not(feature = "raw"))]
fn get_raw_dimensions(_data: &[u8]) -> CodecResult<(i32, i32)> {
    Err(CodecError::Unsupported(
        "RAW support requires the 'raw' feature".into(),
    ))
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
        ImageFormat::Bmp => BmpDecoder::new().decode_bytes(data),
        ImageFormat::Ico => IcoDecoder::new().decode_bytes(data),
        ImageFormat::Wbmp => WbmpDecoder::new().decode_bytes(data),
        ImageFormat::Avif => AvifDecoder::new().decode_bytes(data),
        ImageFormat::Raw => RawDecoder::new().decode_bytes(data),
        _ => Err(CodecError::Unsupported(format!(
            "Format {:?} not supported",
            format
        ))),
    }
}

/// Get the image dimensions without fully decoding.
pub fn get_image_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    let format = ImageFormat::from_magic(data);

    match format {
        ImageFormat::Png => get_png_dimensions(data),
        ImageFormat::Jpeg => get_jpeg_dimensions(data),
        ImageFormat::Bmp => get_bmp_dimensions(data),
        ImageFormat::Ico => get_ico_dimensions(data),
        ImageFormat::Wbmp => get_wbmp_dimensions(data),
        ImageFormat::Avif => get_avif_dimensions(data),
        ImageFormat::Raw => get_raw_dimensions(data),
        _ => Err(CodecError::Unsupported(format!(
            "Format {:?} not supported",
            format
        ))),
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

    Err(CodecError::InvalidData(
        "Could not find JPEG dimensions".into(),
    ))
}

fn get_bmp_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    if data.len() < 26 {
        return Err(CodecError::InvalidData("BMP too short".into()));
    }

    let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]).abs();

    Ok((width, height))
}

fn get_ico_dimensions(data: &[u8]) -> CodecResult<(i32, i32)> {
    if data.len() < 22 {
        return Err(CodecError::InvalidData("ICO too short".into()));
    }

    let image_count = u16::from_le_bytes([data[4], data[5]]) as usize;
    if image_count == 0 {
        return Err(CodecError::InvalidData("ICO has no images".into()));
    }

    // Return the largest image dimensions
    let mut best_width = 0i32;
    let mut best_height = 0i32;

    for i in 0..image_count {
        let entry_offset = 6 + i * 16;
        if entry_offset + 8 > data.len() {
            break;
        }

        let width = if data[entry_offset] == 0 {
            256
        } else {
            data[entry_offset] as i32
        };
        let height = if data[entry_offset + 1] == 0 {
            256
        } else {
            data[entry_offset + 1] as i32
        };

        if width * height > best_width * best_height {
            best_width = width;
            best_height = height;
        }
    }

    Ok((best_width, best_height))
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

        // BMP magic bytes
        let bmp = b"BM\x00\x00\x00\x00\x00\x00";
        assert_eq!(ImageFormat::from_magic(bmp), ImageFormat::Bmp);

        // ICO magic bytes
        let ico = [0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x10, 0x10];
        assert_eq!(ImageFormat::from_magic(&ico), ImageFormat::Ico);

        // Unknown (need at least 8 bytes to test)
        let unknown = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(ImageFormat::from_magic(&unknown), ImageFormat::Unknown);
    }

    #[test]
    fn test_format_extensions() {
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Gif.extension(), "gif");
        assert_eq!(ImageFormat::Bmp.extension(), "bmp");
        assert_eq!(ImageFormat::Ico.extension(), "ico");
    }

    #[test]
    fn test_format_mime_types() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Bmp.mime_type(), "image/bmp");
        assert_eq!(ImageFormat::Ico.mime_type(), "image/x-icon");
    }

    #[test]
    fn test_encoder_quality() {
        let q = EncoderQuality::new(75);
        assert_eq!(q.value(), 75);

        let q_over = EncoderQuality::new(150);
        assert_eq!(q_over.value(), 100);
    }

    #[test]
    fn test_bmp_encode_decode_roundtrip() {
        // Create a simple 2x2 image
        let info = crate::ImageInfo::new(
            2,
            2,
            skia_rs_core::ColorType::Rgba8888,
            skia_rs_core::AlphaType::Unpremul,
        );
        let pixels = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 0, 255, // Yellow
        ];
        let image = Image::from_raster_data_owned(info, pixels, 8).unwrap();

        // Encode to BMP
        let encoder = BmpEncoder::new();
        let encoded = encoder.encode_bytes(&image).unwrap();

        // Verify format detection
        assert_eq!(ImageFormat::from_magic(&encoded), ImageFormat::Bmp);

        // Decode back
        let decoder = BmpDecoder::new();
        let decoded = decoder.decode_bytes(&encoded).unwrap();

        // Verify dimensions
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
    }

    #[test]
    fn test_bmp_dimensions() {
        // Create a simple BMP header for a 100x50 image
        let mut bmp = vec![0u8; 54];
        bmp[0] = b'B';
        bmp[1] = b'M';
        // Width at offset 18-21
        bmp[18..22].copy_from_slice(&100i32.to_le_bytes());
        // Height at offset 22-25
        bmp[22..26].copy_from_slice(&50i32.to_le_bytes());

        let dims = get_bmp_dimensions(&bmp).unwrap();
        assert_eq!(dims, (100, 50));
    }
}
