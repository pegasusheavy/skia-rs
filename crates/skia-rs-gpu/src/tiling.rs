//! Image tiling for GPU rendering.
//!
//! This module provides utilities for tiling images across surfaces,
//! handling different tile modes and transformations.

use skia_rs_core::{Matrix, Point, Rect};

/// Tile mode for image edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileMode {
    /// Clamp to edge pixels.
    #[default]
    Clamp,
    /// Repeat the image.
    Repeat,
    /// Mirror the image.
    Mirror,
    /// Transparent outside bounds.
    Decal,
}

/// Image tiling configuration.
#[derive(Debug, Clone)]
pub struct TileConfig {
    /// Horizontal tile mode.
    pub tile_x: TileMode,
    /// Vertical tile mode.
    pub tile_y: TileMode,
    /// Source rect within the image (normalized 0-1).
    pub source_rect: Rect,
    /// Destination rect.
    pub dest_rect: Rect,
    /// Transform to apply.
    pub transform: Matrix,
}

impl Default for TileConfig {
    fn default() -> Self {
        Self {
            tile_x: TileMode::Clamp,
            tile_y: TileMode::Clamp,
            source_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            dest_rect: Rect::EMPTY,
            transform: Matrix::IDENTITY,
        }
    }
}

/// A single tile instance for batched rendering.
#[derive(Debug, Clone, Copy)]
pub struct TileInstance {
    /// Position of the tile.
    pub position: Point,
    /// UV coordinates [u0, v0, u1, v1].
    pub uv: [f32; 4],
    /// Size of the tile.
    pub size: [f32; 2],
    /// Tile index (for debugging).
    pub tile_index: [i32; 2],
}

/// Generate tile instances for a given area.
#[must_use] 
pub fn generate_tiles(
    image_width: u32,
    image_height: u32,
    config: &TileConfig,
) -> Vec<TileInstance> {
    let mut tiles = Vec::new();

    if config.dest_rect.is_empty() || image_width == 0 || image_height == 0 {
        return tiles;
    }

    let src_width = config.source_rect.width() * image_width as f32;
    let src_height = config.source_rect.height() * image_height as f32;

    if src_width <= 0.0 || src_height <= 0.0 {
        return tiles;
    }

    // Tile modes are handled *per axis*. A Clamp (or Decal) axis is a single
    // slot spanning the full destination extent, with the UV range covering
    // the whole source; only Repeat/Mirror axes are subdivided into repeated
    // native-sized slots. Previously any pair other than (Clamp,Clamp) tiled
    // *both* axes, so e.g. (Repeat, Clamp) wrongly repeated vertically too.
    let x_slots = axis_slots(
        config.tile_x,
        config.dest_rect.left,
        config.dest_rect.width(),
        src_width,
        config.source_rect.left,
        config.source_rect.width(),
    );
    let y_slots = axis_slots(
        config.tile_y,
        config.dest_rect.top,
        config.dest_rect.height(),
        src_height,
        config.source_rect.top,
        config.source_rect.height(),
    );

    for ys in &y_slots {
        for xs in &x_slots {
            // Mirror flips swap the UV endpoints on the affected axis.
            let (u0, u1) = if xs.flip {
                (xs.uv1, xs.uv0)
            } else {
                (xs.uv0, xs.uv1)
            };
            let (v0, v1) = if ys.flip {
                (ys.uv1, ys.uv0)
            } else {
                (ys.uv0, ys.uv1)
            };

            tiles.push(TileInstance {
                position: Point::new(xs.pos, ys.pos),
                uv: [u0, v0, u1, v1],
                size: [xs.size, ys.size],
                tile_index: [xs.index, ys.index],
            });
        }
    }

    tiles
}

/// One slot along a single axis: its destination position/size and the UV
/// span (plus a mirror flip flag) to draw there.
struct AxisSlot {
    index: i32,
    pos: f32,
    size: f32,
    uv0: f32,
    uv1: f32,
    flip: bool,
}

/// Compute the per-axis slots for a tile mode.
///
/// * `Clamp` / `Decal`: a single slot covering the whole destination extent,
///   with the full source UV range — the axis does not tile.
/// * `Repeat` / `Mirror`: native-sized slots stepped across the destination
///   (with a one-slot overscan on each side so partial edge tiles are drawn);
///   `Mirror` flips alternate slots.
fn axis_slots(
    mode: TileMode,
    dest_start: f32,
    dest_extent: f32,
    src_extent: f32,
    uv_min: f32,
    uv_size: f32,
) -> Vec<AxisSlot> {
    match mode {
        TileMode::Clamp | TileMode::Decal => vec![AxisSlot {
            index: 0,
            pos: dest_start,
            size: dest_extent,
            uv0: uv_min,
            uv1: uv_min + uv_size,
            flip: false,
        }],
        TileMode::Repeat | TileMode::Mirror => {
            let count = (dest_extent / src_extent).ceil() as i32 + 2;
            let mut slots = Vec::with_capacity((count + 1) as usize);
            for i in -1..count {
                let flip = mode == TileMode::Mirror && i.rem_euclid(2) != 0;
                slots.push(AxisSlot {
                    index: i,
                    pos: (i as f32).mul_add(src_extent, dest_start),
                    size: src_extent,
                    uv0: uv_min,
                    uv1: uv_min + uv_size,
                    flip,
                });
            }
            slots
        }
    }
}

/// Calculate UV transform matrix for tiled rendering.
#[must_use] 
pub fn calculate_uv_transform(image_width: u32, image_height: u32, config: &TileConfig) -> Matrix {
    let scale_x = config.dest_rect.width() / (config.source_rect.width() * image_width as f32);
    let scale_y = config.dest_rect.height() / (config.source_rect.height() * image_height as f32);

    let offset_x = config.source_rect.left;
    let offset_y = config.source_rect.top;

    Matrix::scale(1.0 / scale_x, 1.0 / scale_y).concat(&Matrix::translate(offset_x, offset_y))
}

/// Nine-patch image configuration.
#[derive(Debug, Clone)]
pub struct NinePatch {
    /// Left inset.
    pub left: f32,
    /// Top inset.
    pub top: f32,
    /// Right inset.
    pub right: f32,
    /// Bottom inset.
    pub bottom: f32,
}

impl NinePatch {
    /// Create a new nine-patch configuration.
    #[must_use] 
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create a uniform nine-patch (same inset on all sides).
    #[must_use] 
    pub const fn uniform(inset: f32) -> Self {
        Self::new(inset, inset, inset, inset)
    }
}

/// Generate nine-patch tile instances.
#[must_use] 
pub fn generate_nine_patch(
    image_width: u32,
    image_height: u32,
    patch: &NinePatch,
    dest_rect: &Rect,
) -> Vec<TileInstance> {
    let mut tiles = Vec::with_capacity(9);

    let img_w = image_width as f32;
    let img_h = image_height as f32;

    // Source regions (in pixels)
    let src_left = patch.left;
    let src_top = patch.top;
    let src_right = img_w - patch.right;
    let src_bottom = img_h - patch.bottom;

    // Destination regions
    let dst_left = dest_rect.left;
    let dst_top = dest_rect.top;
    let dst_right = dest_rect.right;
    let dst_bottom = dest_rect.bottom;

    let dst_inner_left = dst_left + patch.left;
    let dst_inner_top = dst_top + patch.top;
    let dst_inner_right = dst_right - patch.right;
    let dst_inner_bottom = dst_bottom - patch.bottom;

    // UV conversion
    let to_uv_x = |x: f32| x / img_w;
    let to_uv_y = |y: f32| y / img_h;

    // Generate 9 patches
    let patches = [
        // Top row
        (
            Rect::new(dst_left, dst_top, dst_inner_left, dst_inner_top),
            [0.0, 0.0, to_uv_x(src_left), to_uv_y(src_top)],
            [-1, -1],
        ),
        (
            Rect::new(dst_inner_left, dst_top, dst_inner_right, dst_inner_top),
            [to_uv_x(src_left), 0.0, to_uv_x(src_right), to_uv_y(src_top)],
            [0, -1],
        ),
        (
            Rect::new(dst_inner_right, dst_top, dst_right, dst_inner_top),
            [to_uv_x(src_right), 0.0, 1.0, to_uv_y(src_top)],
            [1, -1],
        ),
        // Middle row
        (
            Rect::new(dst_left, dst_inner_top, dst_inner_left, dst_inner_bottom),
            [
                0.0,
                to_uv_y(src_top),
                to_uv_x(src_left),
                to_uv_y(src_bottom),
            ],
            [-1, 0],
        ),
        (
            Rect::new(
                dst_inner_left,
                dst_inner_top,
                dst_inner_right,
                dst_inner_bottom,
            ),
            [
                to_uv_x(src_left),
                to_uv_y(src_top),
                to_uv_x(src_right),
                to_uv_y(src_bottom),
            ],
            [0, 0],
        ),
        (
            Rect::new(dst_inner_right, dst_inner_top, dst_right, dst_inner_bottom),
            [
                to_uv_x(src_right),
                to_uv_y(src_top),
                1.0,
                to_uv_y(src_bottom),
            ],
            [1, 0],
        ),
        // Bottom row
        (
            Rect::new(dst_left, dst_inner_bottom, dst_inner_left, dst_bottom),
            [0.0, to_uv_y(src_bottom), to_uv_x(src_left), 1.0],
            [-1, 1],
        ),
        (
            Rect::new(
                dst_inner_left,
                dst_inner_bottom,
                dst_inner_right,
                dst_bottom,
            ),
            [
                to_uv_x(src_left),
                to_uv_y(src_bottom),
                to_uv_x(src_right),
                1.0,
            ],
            [0, 1],
        ),
        (
            Rect::new(dst_inner_right, dst_inner_bottom, dst_right, dst_bottom),
            [to_uv_x(src_right), to_uv_y(src_bottom), 1.0, 1.0],
            [1, 1],
        ),
    ];

    for (rect, uv, idx) in patches {
        if rect.width() > 0.0 && rect.height() > 0.0 {
            tiles.push(TileInstance {
                position: Point::new(rect.left, rect.top),
                uv,
                size: [rect.width(), rect.height()],
                tile_index: idx,
            });
        }
    }

    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_mode() {
        assert_eq!(TileMode::default(), TileMode::Clamp);
    }

    #[test]
    fn test_generate_tiles_clamp() {
        let config = TileConfig {
            tile_x: TileMode::Clamp,
            tile_y: TileMode::Clamp,
            dest_rect: Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            ..Default::default()
        };

        let tiles = generate_tiles(64, 64, &config);
        assert_eq!(tiles.len(), 1);
    }

    #[test]
    fn test_generate_tiles_repeat() {
        let config = TileConfig {
            tile_x: TileMode::Repeat,
            tile_y: TileMode::Repeat,
            dest_rect: Rect::from_xywh(0.0, 0.0, 200.0, 200.0),
            source_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            ..Default::default()
        };

        let tiles = generate_tiles(64, 64, &config);
        assert!(tiles.len() > 1);
    }

    #[test]
    fn test_mixed_tile_modes_repeat_x_clamp_y() {
        // Regression: a Clamp axis must NOT tile. With Repeat in X and Clamp
        // in Y, all tiles share one row: same y position and full dest height.
        let config = TileConfig {
            tile_x: TileMode::Repeat,
            tile_y: TileMode::Clamp,
            dest_rect: Rect::from_xywh(0.0, 0.0, 200.0, 100.0),
            source_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            ..Default::default()
        };
        let tiles = generate_tiles(64, 64, &config);
        assert!(tiles.len() > 1, "X should tile");
        // Exactly one distinct y position, spanning the full dest height.
        let ys: std::collections::BTreeSet<i64> = tiles
            .iter()
            .map(|t| t.position.y.to_bits() as i64)
            .collect();
        assert_eq!(ys.len(), 1, "Clamp Y must not tile (single row)");
        for t in &tiles {
            assert_eq!(t.position.y, 0.0);
            assert!(
                (t.size[1] - 100.0).abs() < 1e-3,
                "clamp Y spans full dest height"
            );
        }
    }

    #[test]
    fn test_mixed_tile_modes_clamp_x_repeat_y() {
        let config = TileConfig {
            tile_x: TileMode::Clamp,
            tile_y: TileMode::Repeat,
            dest_rect: Rect::from_xywh(0.0, 0.0, 100.0, 200.0),
            source_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            ..Default::default()
        };
        let tiles = generate_tiles(64, 64, &config);
        // One column: single distinct x position, full dest width.
        let xs: std::collections::BTreeSet<i64> = tiles
            .iter()
            .map(|t| t.position.x.to_bits() as i64)
            .collect();
        assert_eq!(xs.len(), 1, "Clamp X must not tile (single column)");
        for t in &tiles {
            assert!(
                (t.size[0] - 100.0).abs() < 1e-3,
                "clamp X spans full dest width"
            );
        }
    }

    #[test]
    fn test_nine_patch() {
        let patch = NinePatch::uniform(10.0);
        let dest = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);

        let tiles = generate_nine_patch(64, 64, &patch, &dest);
        assert_eq!(tiles.len(), 9);
    }

    #[test]
    fn test_tile_instance() {
        let tile = TileInstance {
            position: Point::new(10.0, 20.0),
            uv: [0.0, 0.0, 1.0, 1.0],
            size: [64.0, 64.0],
            tile_index: [0, 0],
        };

        assert_eq!(tile.position.x, 10.0);
        assert_eq!(tile.size[0], 64.0);
    }
}
