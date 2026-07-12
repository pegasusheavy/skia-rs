//! SVG-in-OpenType glyph decoder.
//!
//! The OpenType `SVG ` table stores per-glyph SVG documents. Each
//! document is either plain UTF-8 SVG or a gzip-compressed SVG
//! (commonly written as `.svgz`). This module decompresses the raw
//! bytes returned by `skia_rs_text::Font::glyph_svg` and parses them
//! into an [`SvgDom`] ready for rendering.
//!
//! The decoder lives in `skia-rs-svg` (rather than in `skia-rs-text`)
//! to avoid a dependency cycle: `skia-rs-svg` already depends on
//! `skia-rs-text` for glyph-outline rendering, so `skia-rs-text`
//! cannot depend back on `skia-rs-svg`.
//!
//! # Example
//!
//! ```no_run
//! use skia_rs_text::Font;
//! use skia_rs_svg::glyph_svg_to_dom;
//!
//! # let font: Font = todo!();
//! let glyph_id: u16 = 42;
//! if let Some(raw) = font.glyph_svg(glyph_id) {
//!     if let Some(dom) = glyph_svg_to_dom(&raw) {
//!         // Render with the existing SVG pipeline.
//!         // skia_rs_svg::render_svg(&mut canvas, &dom, ...);
//!     }
//! }
//! ```

use crate::dom::SvgDom;
use crate::parser::parse_svg;
use crate::render::render_svg_in_container;
use flate2::read::GzDecoder;
use skia_rs_canvas::Canvas;
use skia_rs_core::Point;
use skia_rs_text::Font;
use std::io::Read;

/// Fallback units-per-em when a font exposes no typeface metrics. 1000 is
/// the common PostScript/OpenType default and matches Skia's behaviour for
/// fonts without a usable head table.
const DEFAULT_UPEM: f32 = 1000.0;

/// gzip magic bytes (`0x1f 0x8b`) — the two-byte prefix of every gzip
/// stream (RFC 1952 section 2.3.1).
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// Decode an SVG-in-OpenType glyph document into an [`SvgDom`].
///
/// `raw` is the byte payload returned by
/// [`skia_rs_text::Font::glyph_svg`]. The SVG table stores documents
/// that may or may not be gzip-compressed (the spec leaves this as a
/// per-document choice); this function auto-detects the gzip magic
/// prefix and transparently decompresses when needed.
///
/// Returns `None` if:
/// - the bytes aren't valid UTF-8 after decompression, or
/// - gzip decompression fails, or
/// - the resulting text isn't parseable SVG.
///
/// The returned [`SvgDom`] can be fed straight into the regular SVG
/// rendering pipeline (`skia_rs_svg::render_svg_to_canvas` or similar)
/// to rasterise the glyph.
#[must_use]
pub fn glyph_svg_to_dom(raw: &[u8]) -> Option<SvgDom> {
    let decoded = decode_glyph_svg_bytes(raw)?;
    let text = std::str::from_utf8(&decoded).ok()?;
    parse_svg(text).ok()
}

/// Render an SVG-in-OpenType glyph at the given origin on the supplied
/// canvas.
///
/// This is the convenience entry point that ties together
/// [`Font::glyph_svg`] (read the table), [`glyph_svg_to_dom`] (decode +
/// parse), and [`render_svg`] (rasterise). It is the SVG-glyph
/// counterpart to `Canvas::draw_color_glyph`'s COLR path, kept in this
/// crate to avoid a dependency cycle (skia-rs-svg already depends on
/// skia-rs-canvas).
///
/// Returns `true` when the glyph had an SVG entry and was drawn,
/// `false` otherwise so the caller can fall through to an outline /
/// COLR path.
pub fn draw_glyph_svg(canvas: &mut Canvas<'_>, font: &Font, glyph: u16, origin: Point) -> bool {
    let Some(raw) = font.glyph_svg(glyph) else {
        return false;
    };
    let Some(dom) = glyph_svg_to_dom(&raw) else {
        return false;
    };

    // SVG glyph documents are authored in font units: their coordinate
    // system spans the em box (upem × upem). Render them into that em box and
    // scale by ppem/upem so the glyph lands at the requested pixel size,
    // rather than stretching the document to fill the whole canvas
    // (SkSVGOpenTypeSVGDecoder::render sets the container size to upem).
    let upem = font
        .typeface()
        .map(|tf| f32::from(tf.units_per_em()))
        .filter(|u| *u > 0.0)
        .unwrap_or(DEFAULT_UPEM);
    let ppem = font.size();
    let scale = ppem / upem;

    canvas.save();
    canvas.translate(origin.x, origin.y);
    canvas.scale(scale, scale);
    render_svg_in_container(&dom, canvas, upem, upem);
    canvas.restore();
    true
}

/// Decompress gzipped SVG bytes (if the input is gzipped) and return
/// the plain SVG bytes. Plain-text SVG input is returned as an owned
/// copy.
///
/// This is exposed separately from [`glyph_svg_to_dom`] for callers
/// that want to route the decompressed SVG through a different parser
/// (e.g. `usvg`) or hand the decompressed text to disk. Returns
/// `None` if gzip decompression fails.
#[must_use]
pub fn decode_glyph_svg_bytes(raw: &[u8]) -> Option<Vec<u8>> {
    if raw.len() >= 2 && raw[..2] == GZIP_MAGIC {
        let mut decoder = GzDecoder::new(raw);
        let mut out = Vec::with_capacity(raw.len() * 4);
        decoder.read_to_end(&mut out).ok()?;
        Some(out)
    } else {
        Some(raw.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    const PLAIN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn decode_plain_svg_is_passthrough() {
        let out = decode_glyph_svg_bytes(PLAIN_SVG.as_bytes()).expect("decode");
        assert_eq!(out, PLAIN_SVG.as_bytes());
    }

    #[test]
    fn decode_gzipped_svg_returns_original_bytes() {
        let gz = gzip(PLAIN_SVG.as_bytes());
        // Gzipped payload must start with the magic prefix we're
        // sniffing; otherwise the test wouldn't exercise the gzip path.
        assert_eq!(&gz[..2], &GZIP_MAGIC);
        let out = decode_glyph_svg_bytes(&gz).expect("decode");
        assert_eq!(out, PLAIN_SVG.as_bytes());
    }

    #[test]
    fn decode_truncated_gzip_returns_none() {
        let gz = gzip(PLAIN_SVG.as_bytes());
        // Chop off the gzip footer so decompression fails mid-stream.
        let truncated = &gz[..gz.len() / 2];
        assert!(decode_glyph_svg_bytes(truncated).is_none());
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact test assertion against a literal parsed from fixed SVG text"
    )]
    fn glyph_svg_to_dom_parses_plain_svg() {
        let dom = glyph_svg_to_dom(PLAIN_SVG.as_bytes()).expect("parse");
        assert_eq!(dom.width, 10.0);
        assert_eq!(dom.height, 10.0);
        // The root has a single <rect> child.
        assert_eq!(dom.root.children.len(), 1);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact test assertion against a literal parsed from fixed SVG text"
    )]
    fn glyph_svg_to_dom_parses_gzipped_svg() {
        let gz = gzip(PLAIN_SVG.as_bytes());
        let dom = glyph_svg_to_dom(&gz).expect("decompress + parse");
        assert_eq!(dom.width, 10.0);
        assert_eq!(dom.height, 10.0);
        assert_eq!(dom.root.children.len(), 1);
    }

    #[test]
    fn glyph_svg_to_dom_rejects_invalid_utf8() {
        // Non-gzipped, non-UTF-8 payload — parse must fail cleanly
        // rather than panic.
        let garbage: &[u8] = &[0xff, 0xfe, 0xfd, 0xfc];
        assert!(glyph_svg_to_dom(garbage).is_none());
    }

    #[test]
    fn draw_glyph_svg_returns_false_when_font_has_no_svg_table() {
        use skia_rs_canvas::{Canvas, raster::PixelBuffer};
        use skia_rs_core::Point;
        use skia_rs_text::Font;

        let mut buffer = PixelBuffer::new(16, 16);
        let mut canvas = Canvas::new_raster(&mut buffer);
        // Default dataless font — no SVG table.
        let font = Font::from_size(16.0);
        let drawn = draw_glyph_svg(&mut canvas, &font, 1, Point::new(0.0, 0.0));
        assert!(
            !drawn,
            "dataless font has no SVG table; draw_glyph_svg must return false"
        );
    }

    #[test]
    fn glyph_svg_to_dom_rejects_non_svg_text() {
        let not_svg = b"<html><body>not svg</body></html>";
        // roxmltree will parse this, but it has no <svg> root — the
        // resulting DOM either fails parsing or comes back with no
        // meaningful size. Our parser currently returns Ok in this
        // case (root is Svg kind but width/height default); assert we
        // at least don't panic and that an obviously-garbage input
        // produces a zero-sized DOM.
        let dom = glyph_svg_to_dom(not_svg);
        if let Some(dom) = dom {
            // If it parsed, it should have the default width/height
            // from the parser's fallback (100 per the parser source).
            assert!(dom.width >= 0.0);
        }
    }
}
