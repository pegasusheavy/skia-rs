#![no_main]

use libfuzzer_sys::fuzz_target;
use skia_rs_codec::{GifDecoder, ImageDecoder};

fuzz_target!(|data: &[u8]| {
    // Limit input size to prevent OOM
    if data.len() > 1_000_000 {
        return;
    }

    // Try to decode arbitrary bytes as GIF - should never panic
    let decoder = GifDecoder::new();
    let _ = decoder.decode_bytes(data);
});
