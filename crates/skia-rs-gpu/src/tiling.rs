//! Image tiling for GPU rendering.
//!
//! This module provides utilities for tiling images across surfaces,
//! handling different tile modes and transformations.

use skia_rs_core::{Matrix, Point, Rect, Scalar};

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

    match (config.tile_x, config.tile_y) {
        (TileMode::Clamp, TileMode::Clamp) | (TileMode::Decal, TileMode::Decal) => {
            // Single tile
            tiles.push(TileInstance {
                position: Point::new(config.dest_rect.left, config.dest_rect.top),
                uv: [
                    config.source_rect.left,
                    config.source_rect.top,
                    config.source_rect.right,
                    config.source_rect.bottom,
                ],
                size: [config.dest_rect.width(), config.dest_rect.height()],
                tile_index: [0, 0],
            });
        }
        _ => {
            // Calculate number of tiles needed
            let dest_width = config.dest_rect.width();
            let dest_height = config.dest_rect.height();

            let tile_count_x = ((dest_width / src_width).ceil() as i32 + 2).max(1);
            let tile_count_y = ((dest_height / src_height).ceil() as i32 + 2).max(1);

            // Generate tiles
            for ty in -1..tile_count_y {
                for tx in -1..tile_count_x {
                    let (uv, flip_x, flip_y) = get_tile_uv(
                        tx,
                        ty,
                        config.tile_x,
                        config.tile_y,
                        &config.source_rect,
                    );

                    // Skip decal tiles outside bounds
                    if config.tile_x == TileMode::Decal && (tx < 0 || tx >= 1) {
                        continue;
                    }
                    if config.tile_y == TileMode::Decal && (ty < 0 || ty >= 1) {
                        continue;
                    }

                    let x = config.dest_rect.left + tx as f32 * src_width;
                    let y = config.dest_rect.top + ty as f32 * src_height;

                    // Apply UV flipping for mirror mode
                    let final_uv = if flip_x || flip_y {
                        [
                            if flip_x { uv[2] } else { uv[0] },
                            if flip_y { uv[3] } else { uv[1] },
                            if flip_x { uv[0] } else { uv[2] },
                            if flip_y { uv[1] } else { uv[3] },
                        ]
                    } else {
                        uv
                    };

                    tiles.push(TileInstance {
                        position: Point::new(x, y),
                        uv: final_uv,
                        size: [src_width, src_height],
                        tile_index: [tx, ty],
                    });
                }
            }
        }
    }

    tiles
}

/// Get UV coordinates for a tile at given index.
fn get_tile_uv(
    tx: i32,
    ty: i32,
    tile_x: TileMode,
    tile_y: TileMode,
    source_rect: &Rect,
) -> ([f32; 4], bool, bool) {
    let (u_base, flip_x) = get_tile_coord(tx, tile_x, source_rect.left, source_rect.right);
    let (v_base, flip_y) = get_tile_coord(ty, tile_y, source_rect.top, source_rect.bottom);

    let u_size = source_rect.width();
    let v_size = source_rect.height();

    (
        [u_base, v_base, u_base + u_size, v_base + v_size],
        flip_x,
        flip_y,
    )
}

/// Get coordinate for a single axis tile.
fn get_tile_coord(index: i32, mode: TileMode, min: f32, _max: f32) -> (f32, bool) {
    match mode {
        TileMode::Clamp | TileMode::Decal => (min, false),
        TileMode::Repeat => (min, false),
        TileMode::Mirror => {
            let flip = index.rem_euclid(2) != 0;
            (min, flip)
        }
    }
}

/// Calculate UV transform matrix for tiled rendering.
pub fn calculate_uv_transform(
    image_width: u32,
    image_height: u32,
    config: &TileConfig,
) -> Matrix {
    let scale_x = config.dest_rect.width() / (config.source_rect.width() * image_width as f32);
    let scale_y = config.dest_rect.height() / (config.source_rect.height() * image_height as f32);

    let offset_x = config.source_rect.left;
    let offset_y = config.source_rect.top;

    Matrix::scale(1.0 / scale_x, 1.0 / scale_y)
        .concat(&Matrix::translate(offset_x, offset_y))
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
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create a uniform nine-patch (same inset on all sides).
    pub fn uniform(inset: f32) -> Self {
        Self::new(inset, inset, inset, inset)
    }
}

/// Generate nine-patch tile instances.
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
            [0.0, to_uv_y(src_top), to_uv_x(src_left), to_uv_y(src_bottom)],
            [-1, 0],
        ),
        (
            Rect::new(dst_inner_left, dst_inner_top, dst_inner_right, dst_inner_bottom),
            [to_uv_x(src_left), to_uv_y(src_top), to_uv_x(src_right), to_uv_y(src_bottom)],
            [0, 0],
        ),
        (
            Rect::new(dst_inner_right, dst_inner_top, dst_right, dst_inner_bottom),
            [to_uv_x(src_right), to_uv_y(src_top), 1.0, to_uv_y(src_bottom)],
            [1, 0],
        ),
        // Bottom row
        (
            Rect::new(dst_left, dst_inner_bottom, dst_inner_left, dst_bottom),
            [0.0, to_uv_y(src_bottom), to_uv_x(src_left), 1.0],
            [-1, 1],
        ),
        (
            Rect::new(dst_inner_left, dst_inner_bottom, dst_inner_right, dst_bottom),
            [to_uv_x(src_left), to_uv_y(src_bottom), to_uv_x(src_right), 1.0],
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
