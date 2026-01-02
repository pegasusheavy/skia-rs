//! PDF document structure.

use crate::canvas::PdfCanvas;
use skia_rs_core::{Rect, Scalar};
use std::io::Write;

/// PDF document metadata.
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Document keywords.
    pub keywords: Option<String>,
    /// Creator application.
    pub creator: Option<String>,
    /// Creation date (PDF format).
    pub creation_date: Option<String>,
    /// Modification date (PDF format).
    pub mod_date: Option<String>,
}

/// PDF document builder.
pub struct PdfDocument {
    /// Document metadata.
    metadata: PdfMetadata,
    /// Pages in the document.
    pages: Vec<PdfPage>,
    /// Next object ID.
    next_object_id: u32,
}

/// A page in the PDF document.
pub struct PdfPage {
    /// Page width in points.
    pub width: Scalar,
    /// Page height in points.
    pub height: Scalar,
    /// Page content stream.
    pub content: Vec<u8>,
    /// Object ID.
    pub object_id: u32,
}

impl Default for PdfDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfDocument {
    /// Create a new PDF document.
    pub fn new() -> Self {
        Self {
            metadata: PdfMetadata::default(),
            pages: Vec::new(),
            next_object_id: 1,
        }
    }

    /// Set the document metadata.
    pub fn set_metadata(&mut self, metadata: PdfMetadata) {
        self.metadata = metadata;
    }

    /// Get mutable reference to metadata.
    pub fn metadata_mut(&mut self) -> &mut PdfMetadata {
        &mut self.metadata
    }

    /// Allocate a new object ID.
    fn alloc_object_id(&mut self) -> u32 {
        let id = self.next_object_id;
        self.next_object_id += 1;
        id
    }

    /// Begin a new page.
    pub fn begin_page(&mut self, width: Scalar, height: Scalar) -> PdfCanvas {
        let object_id = self.alloc_object_id();
        PdfCanvas::new(width, height, object_id)
    }

    /// End the current page and add it to the document.
    pub fn end_page(&mut self, canvas: PdfCanvas) {
        let width = canvas.width();
        let height = canvas.height();
        let object_id = canvas.object_id();
        let content = canvas.into_content();

        let page = PdfPage {
            width,
            height,
            content,
            object_id,
        };
        self.pages.push(page);
    }

    /// Get the number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Write the PDF to a writer.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // PDF header
        writer.write_all(b"%PDF-1.4\n")?;
        writer.write_all(b"%\xE2\xE3\xCF\xD3\n")?; // Binary marker

        let mut object_offsets: Vec<(u32, u64)> = Vec::new();
        let mut offset = 15u64; // Header size

        // Write catalog
        let catalog_id = 1u32;
        object_offsets.push((catalog_id, offset));
        let catalog = format!(
            "{} 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            catalog_id
        );
        writer.write_all(catalog.as_bytes())?;
        offset += catalog.len() as u64;

        // Write pages object
        let pages_id = 2u32;
        object_offsets.push((pages_id, offset));

        let page_refs: Vec<String> = self
            .pages
            .iter()
            .enumerate()
            .map(|(i, _)| format!("{} 0 R", 3 + i * 2))
            .collect();

        let pages = format!(
            "{} 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            pages_id,
            page_refs.join(" "),
            self.pages.len()
        );
        writer.write_all(pages.as_bytes())?;
        offset += pages.len() as u64;

        // Write each page
        for (i, page) in self.pages.iter().enumerate() {
            let page_id = 3 + i as u32 * 2;
            let content_id = page_id + 1;

            // Page object
            object_offsets.push((page_id, offset));
            let page_obj = format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources << >> >>\nendobj\n",
                page_id, page.width, page.height, content_id
            );
            writer.write_all(page_obj.as_bytes())?;
            offset += page_obj.len() as u64;

            // Content stream
            object_offsets.push((content_id, offset));
            let content_header = format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n",
                content_id,
                page.content.len()
            );
            writer.write_all(content_header.as_bytes())?;
            writer.write_all(&page.content)?;
            writer.write_all(b"\nendstream\nendobj\n")?;
            offset += content_header.len() as u64 + page.content.len() as u64 + 18;
        }

        // Write info dictionary if metadata present
        let info_id = if self.has_metadata() {
            let id = self.next_object_id + self.pages.len() as u32 * 2;
            object_offsets.push((id, offset));
            let info = self.build_info_dict(id);
            writer.write_all(info.as_bytes())?;
            offset += info.len() as u64;
            Some(id)
        } else {
            None
        };

        // Write xref table
        let xref_offset = offset;
        writer.write_all(b"xref\n")?;
        writer.write_all(format!("0 {}\n", object_offsets.len() + 1).as_bytes())?;
        writer.write_all(b"0000000000 65535 f \n")?;

        // Sort offsets by object ID
        object_offsets.sort_by_key(|(id, _)| *id);
        for (_, offset) in &object_offsets {
            writer.write_all(format!("{:010} 00000 n \n", offset).as_bytes())?;
        }

        // Write trailer
        writer.write_all(b"trailer\n")?;
        let trailer = if let Some(info) = info_id {
            format!(
                "<< /Size {} /Root 1 0 R /Info {} 0 R >>\n",
                object_offsets.len() + 1,
                info
            )
        } else {
            format!("<< /Size {} /Root 1 0 R >>\n", object_offsets.len() + 1)
        };
        writer.write_all(trailer.as_bytes())?;

        // Write startxref
        writer.write_all(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes())?;

        Ok(())
    }

    /// Check if metadata is present.
    fn has_metadata(&self) -> bool {
        self.metadata.title.is_some()
            || self.metadata.author.is_some()
            || self.metadata.subject.is_some()
            || self.metadata.creator.is_some()
    }

    /// Build the info dictionary.
    fn build_info_dict(&self, id: u32) -> String {
        let mut entries = Vec::new();

        if let Some(title) = &self.metadata.title {
            entries.push(format!("/Title ({})", escape_pdf_string(title)));
        }
        if let Some(author) = &self.metadata.author {
            entries.push(format!("/Author ({})", escape_pdf_string(author)));
        }
        if let Some(subject) = &self.metadata.subject {
            entries.push(format!("/Subject ({})", escape_pdf_string(subject)));
        }
        if let Some(creator) = &self.metadata.creator {
            entries.push(format!("/Creator ({})", escape_pdf_string(creator)));
        }
        if let Some(keywords) = &self.metadata.keywords {
            entries.push(format!("/Keywords ({})", escape_pdf_string(keywords)));
        }

        entries.push("/Producer (skia-rs)".to_string());

        format!("{} 0 obj\n<< {} >>\nendobj\n", id, entries.join(" "))
    }

    /// Generate PDF bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        self.write_to(&mut buffer).unwrap();
        buffer
    }
}

/// Escape special characters in a PDF string.
fn escape_pdf_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\\' => result.push_str("\\\\"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_document_empty() {
        let doc = PdfDocument::new();
        let bytes = doc.to_bytes();
        assert!(bytes.starts_with(b"%PDF-1.4"));
    }

    #[test]
    fn test_pdf_document_with_page() {
        let mut doc = PdfDocument::new();
        let canvas = doc.begin_page(612.0, 792.0); // Letter size
        doc.end_page(canvas);

        let bytes = doc.to_bytes();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn test_pdf_metadata() {
        let mut doc = PdfDocument::new();
        doc.metadata_mut().title = Some("Test Document".to_string());
        doc.metadata_mut().author = Some("Test Author".to_string());

        let canvas = doc.begin_page(612.0, 792.0);
        doc.end_page(canvas);

        let bytes = doc.to_bytes();
        let content = String::from_utf8_lossy(&bytes);
        assert!(content.contains("/Title (Test Document)"));
    }
}
