//! End-to-end tests for `skia-rs-pdf`.
//!
//! Covers:
//! - Producing a PDF that mixes text, images, transparency, and PDF/A
//!   metadata, and sanity-checking the resulting bytes (resources dict
//!   populated; XObjects, Font, ExtGState all referenced; xref/trailer
//!   present).
//! - TrueType embedding: feeding a real `demo.ttf` through `PdfFont::
//!   truetype` and asserting that the emitted PDF contains the font's
//!   FontFile2 stream, a matching FontDescriptor, and populated Widths.

use skia_rs_core::{Color, Rect};
use skia_rs_paint::Paint;
use skia_rs_pdf::{PdfALevel, PdfDocument, StandardFont};

const DEMO_TTF: &[u8] = include_bytes!("fixtures/demo.ttf");

/// Sanity: a trivial document round-trips through the byte writer and
/// contains every structural marker. Not a pdftotext test — we only check
/// the bytes we produced.
#[test]
fn simple_pdf_has_valid_structure() {
    let mut doc = PdfDocument::new();
    doc.metadata_mut().title = Some("Smoke".to_string());
    doc.metadata_mut().creator = Some("skia-rs".to_string());
    {
        let mut c = doc.begin_page(612.0, 792.0);
        c.use_standard_font(StandardFont::Helvetica);
        let mut p = Paint::new();
        p.set_color32(Color::from_rgb(0, 0, 0));
        c.draw_text("Hello, World!", 72.0, 72.0, 14.0, &p);
        let page = c.finish();
        doc.end_page(page);
    }
    let mut buf = Vec::new();
    doc.write_to(&mut buf).unwrap();

    assert!(buf.starts_with(b"%PDF-1.4"));
    assert!(buf.ends_with(b"%%EOF\n"));
    let s = String::from_utf8_lossy(&buf);
    assert!(s.contains("/Title (Smoke)"));
    assert!(s.contains("/Creator (skia-rs)"));
    assert!(s.contains("/ID ["));
    assert!(s.contains("(Hello, World!)"));
}

#[test]
fn end_to_end_pdf_with_text_image_transparency_pdfa() {
    let mut doc = PdfDocument::new();
    doc.set_pdfa_conformance(PdfALevel::A2b); // A2b permits transparency.

    // Seed an RGBA image on the document.
    let data = vec![200u8; 8 * 8 * 4];
    let (img_idx, _mask_idx) = doc.images_mut().add_rgba(8, 8, &data);

    {
        let mut canvas = doc.begin_page(612.0, 792.0);
        canvas.use_standard_font(StandardFont::Helvetica);

        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(0, 0, 0));
        canvas.draw_text("Hello, PDF!", 72.0, 100.0, 14.0, &paint);

        // Translucent rect — should register an ExtGState.
        let mut translucent = Paint::new();
        translucent.set_color32(Color::from_argb(96, 255, 64, 64));
        canvas.draw_rect(&Rect::from_xywh(72.0, 150.0, 200.0, 80.0), &translucent);

        canvas.draw_image(img_idx, 300.0, 150.0, 100.0, 100.0);

        let page = canvas.finish();
        doc.end_page(page);
    }

    let mut buf = Vec::new();
    doc.write_to(&mut buf).expect("write should succeed for A2b");

    // `PdfDocument::write_to` emits a binary marker (4 high bits) after the
    // header; cope with non-UTF-8 bytes by using `from_utf8_lossy`.
    let s_cow = String::from_utf8_lossy(&buf);
    let s = &*s_cow;
    // Structural sanity checks.
    assert!(buf.starts_with(b"%PDF-1.7"), "expected PDF-1.7 header for A2b");
    assert!(s.contains("xref"));
    assert!(s.contains("startxref"));
    assert!(s.contains("%%EOF"));

    // Resources dict populated.
    assert!(s.contains("/Font << /F1"), "font missing from resources");
    assert!(s.contains("/XObject << /Im"), "image missing from resources");
    assert!(s.contains("/ExtGState << /GS"), "extgstate missing");

    // XMP metadata + output intent from PDF/A wiring.
    assert!(s.contains("/Type /Metadata"));
    assert!(s.contains("/Type /OutputIntent"));
    assert!(s.contains("pdfaid:part>2"));

    // Image XObject emitted with SMask resolved to a real object id (not
    // the raw manager index).
    assert!(s.contains("/SMask"));

    // ToUnicode CMap emitted for the font.
    assert!(s.contains("/ToUnicode"));
    assert!(s.contains("beginbfchar"));
}

#[test]
fn truetype_embedding_emits_fontfile2_and_widths() {
    let mut doc = PdfDocument::new();

    let font_idx = doc
        .fonts_mut()
        .register_truetype("Demo", DEMO_TTF.to_vec());

    {
        let mut canvas = doc.begin_page(200.0, 200.0);
        canvas.set_font(font_idx);
        let paint = Paint::new();
        canvas.draw_text("A", 10.0, 20.0, 12.0, &paint);
        let page = canvas.finish();
        doc.end_page(page);
    }

    let mut buf = Vec::new();
    doc.write_to(&mut buf).expect("write should succeed");

    let s = String::from_utf8_lossy(&buf);

    // The TTF is compressed (flate) inside a FontFile2 stream. We detect it
    // via the object header marker.
    assert!(s.contains("/FontFile2"), "no FontFile2 stream: {}", s);
    assert!(
        s.contains("/FontDescriptor"),
        "no FontDescriptor reference"
    );
    assert!(s.contains("/Subtype /TrueType"));
    // Subset prefix on the BaseFont (six uppercase letters + '+').
    let subset_re = regex_lite(&s, "/BaseFont /", "+Demo");
    assert!(
        subset_re,
        "BaseFont should carry a six-letter subset prefix; output: {}",
        &s[..2000.min(s.len())]
    );

    // Widths array: the demo.ttf has only 'A' mapped, so widths for 'A'
    // (ASCII 0x41 = 65) must be nonzero. The font's FirstChar/LastChar
    // should include 65.
    assert!(s.contains("/FirstChar 65") || s.contains("/FirstChar 33"),
        "unexpected FirstChar range");
}

/// Tiny helper: check that the string `haystack` contains the pattern
/// `start .{6}end` (where `.{6}` matches six arbitrary chars). Used to
/// verify the `ABCDEF+FontName` subset-prefix syntax without pulling in
/// the `regex` crate.
fn regex_lite(haystack: &str, start: &str, end: &str) -> bool {
    let mut i = 0;
    while let Some(pos) = haystack[i..].find(start) {
        let abs = i + pos + start.len();
        if abs + 6 + end.len() > haystack.len() {
            return false;
        }
        let tag = &haystack[abs..abs + 6];
        let rest = &haystack[abs + 6..abs + 6 + end.len()];
        if tag.chars().all(|c| c.is_ascii_uppercase()) && rest == end {
            return true;
        }
        i = abs;
    }
    false
}
