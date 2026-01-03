//! Atlas management for GPU rendering.
//!
//! This module provides texture atlas management for efficiently batching
//! small paths, glyphs, and other small elements into larger textures.

use skia_rs_core::{Point, Rect, Scalar};
use std::collections::HashMap;

/// Unique identifier for an atlas entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtlasEntryId(u64);

impl AtlasEntryId {
    /// Create a new entry ID.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// A region within an atlas.
#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    /// X position in atlas.
    pub x: u32,
    /// Y position in atlas.
    pub y: u32,
    /// Width of region.
    pub width: u32,
    /// Height of region.
    pub height: u32,
    /// Atlas layer (for array textures).
    pub layer: u32,
}

impl AtlasRegion {
    /// Get UV coordinates for this region within an atlas of given size.
    pub fn uv_rect(&self, atlas_width: u32, atlas_height: u32) -> [f32; 4] {
        [
            self.x as f32 / atlas_width as f32,
            self.y as f32 / atlas_height as f32,
            (self.x + self.width) as f32 / atlas_width as f32,
            (self.y + self.height) as f32 / atlas_height as f32,
        ]
    }

    /// Convert to a rect.
    pub fn to_rect(&self) -> Rect {
        Rect::from_xywh(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        )
    }
}

/// Atlas configuration.
#[derive(Debug, Clone)]
pub struct AtlasConfig {
    /// Atlas width in pixels.
    pub width: u32,
    /// Atlas height in pixels.
    pub height: u32,
    /// Maximum number of layers (for array textures).
    pub max_layers: u32,
    /// Padding between entries.
    pub padding: u32,
    /// Allow resizing when full.
    pub allow_resize: bool,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            width: 2048,
            height: 2048,
            max_layers: 4,
            padding: 1,
            allow_resize: true,
        }
    }
}

/// Atlas allocation result.
#[derive(Debug)]
pub enum AtlasAllocResult {
    /// Successfully allocated.
    Success(AtlasRegion),
    /// Atlas is full, need to flush and reset.
    Full,
    /// Request too large for atlas.
    TooLarge,
}

/// A single atlas layer using shelf-based allocation.
#[derive(Debug)]
struct AtlasLayer {
    /// Current shelf Y position.
    current_y: u32,
    /// Current shelf height.
    current_shelf_height: u32,
    /// Current X position in shelf.
    current_x: u32,
    /// Layer dimensions.
    width: u32,
    height: u32,
}

impl AtlasLayer {
    fn new(width: u32, height: u32) -> Self {
        Self {
            current_y: 0,
            current_shelf_height: 0,
            current_x: 0,
            width,
            height,
        }
    }

    fn allocate(&mut self, width: u32, height: u32, padding: u32) -> Option<(u32, u32)> {
        let padded_width = width + padding * 2;
        let padded_height = height + padding * 2;

        // Check if it fits in current shelf
        if self.current_x + padded_width <= self.width
            && self.current_y + padded_height.max(self.current_shelf_height) <= self.height
        {
            let x = self.current_x + padding;
            let y = self.current_y + padding;

            self.current_x += padded_width;
            self.current_shelf_height = self.current_shelf_height.max(padded_height);

            return Some((x, y));
        }

        // Try next shelf
        if self.current_y + self.current_shelf_height + padded_height <= self.height {
            self.current_y += self.current_shelf_height;
            self.current_x = 0;
            self.current_shelf_height = padded_height;

            if self.current_x + padded_width <= self.width {
                let x = self.current_x + padding;
                let y = self.current_y + padding;

                self.current_x += padded_width;

                return Some((x, y));
            }
        }

        None
    }

    fn reset(&mut self) {
        self.current_y = 0;
        self.current_shelf_height = 0;
        self.current_x = 0;
    }
}

/// A texture atlas for batching small elements.
#[derive(Debug)]
pub struct TextureAtlas {
    /// Configuration.
    config: AtlasConfig,
    /// Atlas layers.
    layers: Vec<AtlasLayer>,
    /// Entry lookup by ID.
    entries: HashMap<AtlasEntryId, AtlasRegion>,
    /// Next entry ID.
    next_id: u64,
    /// Generation counter (incremented on reset).
    generation: u64,
}

impl TextureAtlas {
    /// Create a new texture atlas.
    pub fn new(config: AtlasConfig) -> Self {
        let mut layers = Vec::with_capacity(config.max_layers as usize);
        layers.push(AtlasLayer::new(config.width, config.height));

        Self {
            config,
            layers,
            entries: HashMap::new(),
            next_id: 0,
            generation: 0,
        }
    }

    /// Get atlas configuration.
    pub fn config(&self) -> &AtlasConfig {
        &self.config
    }

    /// Get current generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get number of active layers.
    pub fn layer_count(&self) -> u32 {
        self.layers.len() as u32
    }

    /// Get number of entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Look up an existing entry.
    pub fn lookup(&self, id: AtlasEntryId) -> Option<&AtlasRegion> {
        self.entries.get(&id)
    }

    /// Allocate a region in the atlas.
    pub fn allocate(&mut self, width: u32, height: u32) -> AtlasAllocResult {
        // Check if request is too large
        if width > self.config.width || height > self.config.height {
            return AtlasAllocResult::TooLarge;
        }

        // Try to allocate in existing layers
        for (layer_idx, layer) in self.layers.iter_mut().enumerate() {
            if let Some((x, y)) = layer.allocate(width, height, self.config.padding) {
                let id = AtlasEntryId::new(self.next_id);
                self.next_id += 1;

                let region = AtlasRegion {
                    x,
                    y,
                    width,
                    height,
                    layer: layer_idx as u32,
                };

                self.entries.insert(id, region);
                return AtlasAllocResult::Success(region);
            }
        }

        // Try to add a new layer
        if self.layers.len() < self.config.max_layers as usize {
            let mut new_layer = AtlasLayer::new(self.config.width, self.config.height);
            if let Some((x, y)) = new_layer.allocate(width, height, self.config.padding) {
                let layer_idx = self.layers.len();
                self.layers.push(new_layer);

                let id = AtlasEntryId::new(self.next_id);
                self.next_id += 1;

                let region = AtlasRegion {
                    x,
                    y,
                    width,
                    height,
                    layer: layer_idx as u32,
                };

                self.entries.insert(id, region);
                return AtlasAllocResult::Success(region);
            }
        }

        AtlasAllocResult::Full
    }

    /// Allocate and return the entry ID.
    pub fn allocate_with_id(&mut self, width: u32, height: u32) -> Option<(AtlasEntryId, AtlasRegion)> {
        match self.allocate(width, height) {
            AtlasAllocResult::Success(region) => {
                let id = AtlasEntryId::new(self.next_id - 1);
                Some((id, region))
            }
            _ => None,
        }
    }

    /// Reset the atlas, clearing all entries.
    pub fn reset(&mut self) {
        for layer in &mut self.layers {
            layer.reset();
        }
        self.entries.clear();
        self.generation += 1;
        // Keep only first layer
        self.layers.truncate(1);
    }

    /// Compact the atlas by removing unused entries.
    ///
    /// This requires re-uploading data, so returns the IDs that need updating.
    pub fn compact(&mut self) -> Vec<AtlasEntryId> {
        // For simplicity, just reset and return all IDs
        let ids: Vec<AtlasEntryId> = self.entries.keys().copied().collect();
        self.reset();
        ids
    }
}

/// Atlas manager for multiple atlases by type.
#[derive(Debug)]
pub struct AtlasManager {
    /// Path atlas.
    path_atlas: TextureAtlas,
    /// Glyph atlas (alpha).
    glyph_atlas: TextureAtlas,
    /// Color atlas (RGBA).
    color_atlas: TextureAtlas,
}

impl AtlasManager {
    /// Create a new atlas manager with default configuration.
    pub fn new() -> Self {
        Self {
            path_atlas: TextureAtlas::new(AtlasConfig {
                width: 2048,
                height: 2048,
                max_layers: 4,
                padding: 2,
                allow_resize: true,
            }),
            glyph_atlas: TextureAtlas::new(AtlasConfig {
                width: 1024,
                height: 1024,
                max_layers: 2,
                padding: 1,
                allow_resize: true,
            }),
            color_atlas: TextureAtlas::new(AtlasConfig {
                width: 1024,
                height: 1024,
                max_layers: 2,
                padding: 1,
                allow_resize: true,
            }),
        }
    }

    /// Get path atlas.
    pub fn path_atlas(&self) -> &TextureAtlas {
        &self.path_atlas
    }

    /// Get mutable path atlas.
    pub fn path_atlas_mut(&mut self) -> &mut TextureAtlas {
        &mut self.path_atlas
    }

    /// Get glyph atlas.
    pub fn glyph_atlas(&self) -> &TextureAtlas {
        &self.glyph_atlas
    }

    /// Get mutable glyph atlas.
    pub fn glyph_atlas_mut(&mut self) -> &mut TextureAtlas {
        &mut self.glyph_atlas
    }

    /// Get color atlas.
    pub fn color_atlas(&self) -> &TextureAtlas {
        &self.color_atlas
    }

    /// Get mutable color atlas.
    pub fn color_atlas_mut(&mut self) -> &mut TextureAtlas {
        &mut self.color_atlas
    }

    /// Reset all atlases.
    pub fn reset_all(&mut self) {
        self.path_atlas.reset();
        self.glyph_atlas.reset();
        self.color_atlas.reset();
    }
}

impl Default for AtlasManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_region_uv() {
        let region = AtlasRegion {
            x: 100,
            y: 200,
            width: 50,
            height: 75,
            layer: 0,
        };

        let uv = region.uv_rect(1000, 1000);
        assert_eq!(uv[0], 0.1);
        assert_eq!(uv[1], 0.2);
        assert_eq!(uv[2], 0.15);
        assert_eq!(uv[3], 0.275);
    }

    #[test]
    fn test_atlas_allocate() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            max_layers: 1,
            padding: 0,
            allow_resize: false,
        };

        let mut atlas = TextureAtlas::new(config);

        // First allocation should succeed
        match atlas.allocate(64, 64) {
            AtlasAllocResult::Success(region) => {
                assert_eq!(region.width, 64);
                assert_eq!(region.height, 64);
            }
            _ => panic!("Expected success"),
        }

        // Too large allocation should fail
        match atlas.allocate(512, 512) {
            AtlasAllocResult::TooLarge => {}
            _ => panic!("Expected TooLarge"),
        }
    }

    #[test]
    fn test_atlas_multiple_allocations() {
        let config = AtlasConfig {
            width: 256,
            height: 256,
            max_layers: 1,
            padding: 0,
            allow_resize: false,
        };

        let mut atlas = TextureAtlas::new(config);

        // Allocate multiple small regions
        for _ in 0..16 {
            match atlas.allocate(32, 32) {
                AtlasAllocResult::Success(_) => {}
                _ => panic!("Expected success"),
            }
        }

        assert_eq!(atlas.entry_count(), 16);
    }

    #[test]
    fn test_atlas_reset() {
        let mut atlas = TextureAtlas::new(AtlasConfig::default());

        atlas.allocate(100, 100);
        atlas.allocate(100, 100);
        assert_eq!(atlas.entry_count(), 2);

        let gen_before = atlas.generation();
        atlas.reset();
        assert_eq!(atlas.entry_count(), 0);
        assert_eq!(atlas.generation(), gen_before + 1);
    }

    #[test]
    fn test_atlas_manager() {
        let mut manager = AtlasManager::new();

        // Allocate in different atlases
        manager.path_atlas_mut().allocate(64, 64);
        manager.glyph_atlas_mut().allocate(32, 32);
        manager.color_atlas_mut().allocate(48, 48);

        assert_eq!(manager.path_atlas().entry_count(), 1);
        assert_eq!(manager.glyph_atlas().entry_count(), 1);
        assert_eq!(manager.color_atlas().entry_count(), 1);

        manager.reset_all();
        assert_eq!(manager.path_atlas().entry_count(), 0);
    }

    #[test]
    fn test_atlas_entry_id() {
        let id = AtlasEntryId::new(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn test_atlas_lookup() {
        let mut atlas = TextureAtlas::new(AtlasConfig::default());

        if let Some((id, region)) = atlas.allocate_with_id(100, 100) {
            let looked_up = atlas.lookup(id);
            assert!(looked_up.is_some());
            assert_eq!(looked_up.unwrap().width, region.width);
        }
    }
}
