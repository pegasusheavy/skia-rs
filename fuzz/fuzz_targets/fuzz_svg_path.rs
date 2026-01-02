#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_path::parse_svg_path;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as SVG path data
    if let Ok(s) = std::str::from_utf8(data) {
        // Limit input size to avoid excessive memory usage
        if s.len() > 10000 {
            return;
        }

        // Try to parse the path - should never panic
        let _ = parse_svg_path(s);
    }
});
