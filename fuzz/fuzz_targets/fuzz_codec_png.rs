#![no_main]

use libfuzzer_sys::fuzz_target;
use skia_rs_codec::{PngDecoder, ImageDecoder};

fuzz_target!(|data: &[u8]| {
    // Limit input size
    if data.len() > 1_000_000 {
        return;
    }

    // Try to decode arbitrary bytes as PNG - should never panic
    let decoder = PngDecoder::new();
    let _ = decoder.decode_bytes(data);
});
