#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{IRect, Region, RegionOp as CoreRegionOp};

#[derive(Debug, Arbitrary)]
enum RegionOp {
    Union,
    Intersect,
    Difference,
    Xor,
}

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    rects1: Vec<(i16, i16, i16, i16)>,
    rects2: Vec<(i16, i16, i16, i16)>,
    operation: RegionOp,
    translate: (i16, i16),
}

fuzz_target!(|input: FuzzInput| {
    // Limit number of rectangles
    if input.rects1.len() > 100 || input.rects2.len() > 100 {
        return;
    }

    // Build region 1
    let mut region1 = Region::new();
    for (left, top, right, bottom) in input.rects1.iter() {
        let rect = IRect::new(*left as i32, *top as i32, *right as i32, *bottom as i32);
        region1.op_rect(rect, CoreRegionOp::Union);
    }

    // Build region 2
    let mut region2 = Region::new();
    for (left, top, right, bottom) in input.rects2.iter() {
        let rect = IRect::new(*left as i32, *top as i32, *right as i32, *bottom as i32);
        region2.op_rect(rect, CoreRegionOp::Union);
    }

    // Test operations - create copies for mutation
    let mut result = region1.clone();
    let op = match input.operation {
        RegionOp::Union => CoreRegionOp::Union,
        RegionOp::Intersect => CoreRegionOp::Intersect,
        RegionOp::Difference => CoreRegionOp::Difference,
        RegionOp::Xor => CoreRegionOp::Xor,
    };
    result.op_region(&region2, op);

    // Test other methods
    let _is_empty = result.is_empty();
    let _is_rect = result.is_rect();
    let _bounds = result.bounds();

    // Test translation
    let translated = result.translated(input.translate.0 as i32, input.translate.1 as i32);
    let _translated_bounds = translated.bounds();

    // Test containment
    for irect in result.iter() {
        let center_x = (irect.left + irect.right) / 2;
        let center_y = (irect.top + irect.bottom) / 2;
        assert!(result.contains(center_x, center_y));
    }
});
