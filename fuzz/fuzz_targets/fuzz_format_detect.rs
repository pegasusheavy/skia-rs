#![no_main]

use libfuzzer_sys::fuzz_target;
use skia_rs_codec::ImageFormat;

fuzz_target!(|data: &[u8]| {
    // Format detection should never panic on any input
    let _format = ImageFormat::from_magic(data);
});
