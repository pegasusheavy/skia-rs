//! PDF stream utilities.

use std::io::{self, Write};

/// A stream that can be written to PDF.
pub trait PdfStream {
    /// Write the stream content.
    fn write_content<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    /// Get the length of the stream.
    fn len(&self) -> usize;

    /// Check if the stream is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A simple byte stream.
pub struct ByteStream {
    data: Vec<u8>,
}

impl ByteStream {
    /// Create a new byte stream.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create from existing data.
    pub fn from_data(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Get the data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Take the data.
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// Append data.
    pub fn append(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }
}

impl Default for ByteStream {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfStream for ByteStream {
    fn write_content<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.data)
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

impl Write for ByteStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Standard PDF page sizes (in points, 1 point = 1/72 inch).
pub mod page_sizes {
    use skia_rs_core::Scalar;

    /// Letter size (8.5 x 11 inches).
    pub const LETTER: (Scalar, Scalar) = (612.0, 792.0);

    /// Legal size (8.5 x 14 inches).
    pub const LEGAL: (Scalar, Scalar) = (612.0, 1008.0);

    /// Tabloid size (11 x 17 inches).
    pub const TABLOID: (Scalar, Scalar) = (792.0, 1224.0);

    /// A4 size (210 x 297 mm).
    pub const A4: (Scalar, Scalar) = (595.28, 841.89);

    /// A3 size (297 x 420 mm).
    pub const A3: (Scalar, Scalar) = (841.89, 1190.55);

    /// A5 size (148 x 210 mm).
    pub const A5: (Scalar, Scalar) = (419.53, 595.28);

    /// B5 size (176 x 250 mm).
    pub const B5: (Scalar, Scalar) = (498.90, 708.66);

    /// Convert inches to points.
    #[inline]
    pub const fn inches_to_points(inches: Scalar) -> Scalar {
        inches * 72.0
    }

    /// Convert millimeters to points.
    #[inline]
    pub const fn mm_to_points(mm: Scalar) -> Scalar {
        mm * 2.834645669
    }

    /// Convert points to inches.
    #[inline]
    pub const fn points_to_inches(points: Scalar) -> Scalar {
        points / 72.0
    }

    /// Convert points to millimeters.
    #[inline]
    pub const fn points_to_mm(points: Scalar) -> Scalar {
        points / 2.834645669
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_stream() {
        let mut stream = ByteStream::new();
        stream.append(b"Hello, ");
        stream.append(b"World!");

        assert_eq!(stream.len(), 13);
        assert_eq!(stream.data(), b"Hello, World!");
    }

    #[test]
    fn test_byte_stream_write() {
        let mut stream = ByteStream::new();
        write!(stream, "Value: {}", 42).unwrap();

        assert_eq!(stream.data(), b"Value: 42");
    }

    #[test]
    fn test_page_sizes() {
        assert_eq!(page_sizes::LETTER, (612.0, 792.0));
        assert!((page_sizes::A4.0 - 595.28).abs() < 0.01);
    }
}
