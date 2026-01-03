//! Glyph cache for GPU text rendering.
//!
//! This module provides a cache for rasterized glyphs, managing their
//! storage in texture atlases for efficient GPU rendering.

use crate::atlas::{AtlasAllocResult, AtlasConfig, AtlasEntryId, AtlasRegion, TextureAtlas};
use skia_rs_core::{Point, Rect, Scalar};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// A unique key for identifying a glyph in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Font ID.
    pub font_id: u32,
    /// Glyph ID within the font.
    pub glyph_id: u32,
    /// Font size in pixels (quantized).
    pub size_px: u32,
    /// Sub-pixel position (0-3 for 1/4 pixel precision).
    pub sub_pixel_x: u8,
    /// Sub-pixel position (0-3 for 1/4 pixel precision).
    pub sub_pixel_y: u8,
    /// Additional flags (bold, italic, etc.).
    pub flags: u8,
}

impl GlyphKey {
    /// Create a new glyph key.
    pub fn new(font_id: u32, glyph_id: u32, size: f32, sub_pixel: Point) -> Self {
        Self {
            font_id,
            glyph_id,
            size_px: (size * 4.0) as u32, // Quarter pixel precision
            sub_pixel_x: ((sub_pixel.x.fract() * 4.0) as u8).min(3),
            sub_pixel_y: ((sub_pixel.y.fract() * 4.0) as u8).min(3),
            flags: 0,
        }
    }

    /// Create with flags.
    pub fn with_flags(mut self, flags: u8) -> Self {
        self.flags = flags;
        self
    }
}

/// Cached glyph data.
#[derive(Debug, Clone)]
pub struct CachedGlyph {
    /// Atlas region containing the glyph.
    pub region: AtlasRegion,
    /// Glyph metrics: offset from baseline.
    pub offset: Point,
    /// Glyph advance width.
    pub advance: Scalar,
    /// Glyph bounding box (local).
    pub bounds: Rect,
}

/// Glyph cache statistics.
#[derive(Debug, Clone, Default)]
pub struct GlyphCacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of evictions.
    pub evictions: u64,
    /// Current number of cached glyphs.
    pub cached_count: usize,
}

impl GlyphCacheStats {
    /// Calculate hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Glyph cache configuration.
#[derive(Debug, Clone)]
pub struct GlyphCacheConfig {
    /// Maximum number of cached glyphs.
    pub max_glyphs: usize,
    /// Atlas configuration.
    pub atlas_config: AtlasConfig,
    /// Enable sub-pixel rendering.
    pub sub_pixel_rendering: bool,
}

impl Default for GlyphCacheConfig {
    fn default() -> Self {
        Self {
            max_glyphs: 4096,
            atlas_config: AtlasConfig {
                width: 1024,
                height: 1024,
                max_layers: 4,
                padding: 1,
                allow_resize: true,
            },
            sub_pixel_rendering: true,
        }
    }
}

/// A glyph cache for GPU rendering.
pub struct GlyphCache {
    /// Configuration.
    config: GlyphCacheConfig,
    /// Glyph atlas.
    atlas: TextureAtlas,
    /// Cached glyphs by key.
    cache: HashMap<GlyphKey, CachedGlyph>,
    /// LRU order (front = most recently used).
    lru_order: Vec<GlyphKey>,
    /// Statistics.
    stats: GlyphCacheStats,
}

impl GlyphCache {
    /// Create a new glyph cache.
    pub fn new(config: GlyphCacheConfig) -> Self {
        let atlas = TextureAtlas::new(config.atlas_config.clone());
        Self {
            config,
            atlas,
            cache: HashMap::new(),
            lru_order: Vec::new(),
            stats: GlyphCacheStats::default(),
        }
    }

    /// Get cache configuration.
    pub fn config(&self) -> &GlyphCacheConfig {
        &self.config
    }

    /// Get cache statistics.
    pub fn stats(&self) -> &GlyphCacheStats {
        &self.stats
    }

    /// Get the glyph atlas.
    pub fn atlas(&self) -> &TextureAtlas {
        &self.atlas
    }

    /// Look up a glyph in the cache.
    pub fn lookup(&mut self, key: &GlyphKey) -> Option<&CachedGlyph> {
        if let Some(glyph) = self.cache.get(key) {
            // Update LRU
            if let Some(pos) = self.lru_order.iter().position(|k| k == key) {
                let key = self.lru_order.remove(pos);
                self.lru_order.insert(0, key);
            }
            self.stats.hits += 1;
            Some(glyph)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Check if a glyph is cached without updating LRU.
    pub fn contains(&self, key: &GlyphKey) -> bool {
        self.cache.contains_key(key)
    }

    /// Insert a glyph into the cache.
    ///
    /// Returns the atlas region where the glyph data should be uploaded.
    pub fn insert(
        &mut self,
        key: GlyphKey,
        width: u32,
        height: u32,
        offset: Point,
        advance: Scalar,
    ) -> Option<AtlasRegion> {
        // Check if already cached
        if self.cache.contains_key(&key) {
            return self.cache.get(&key).map(|g| g.region);
        }

        // Evict if at capacity
        while self.cache.len() >= self.config.max_glyphs {
            self.evict_lru();
        }

        // Allocate in atlas
        let region = loop {
            match self.atlas.allocate(width, height) {
                AtlasAllocResult::Success(region) => break region,
                AtlasAllocResult::Full => {
                    // Evict some entries and try again
                    if !self.evict_lru() {
                        // Reset atlas if we can't evict anything
                        self.reset();
                    }
                }
                AtlasAllocResult::TooLarge => {
                    // Glyph too large for atlas
                    return None;
                }
            }
        };

        let bounds = Rect::from_xywh(0.0, 0.0, width as f32, height as f32);

        let glyph = CachedGlyph {
            region,
            offset,
            advance,
            bounds,
        };

        self.cache.insert(key, glyph);
        self.lru_order.insert(0, key);
        self.stats.cached_count = self.cache.len();

        Some(region)
    }

    /// Evict the least recently used glyph.
    fn evict_lru(&mut self) -> bool {
        if let Some(key) = self.lru_order.pop() {
            self.cache.remove(&key);
            self.stats.evictions += 1;
            self.stats.cached_count = self.cache.len();
            true
        } else {
            false
        }
    }

    /// Reset the cache, clearing all entries.
    pub fn reset(&mut self) {
        self.cache.clear();
        self.lru_order.clear();
        self.atlas.reset();
        self.stats.cached_count = 0;
    }

    /// Get number of cached glyphs.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new(GlyphCacheConfig::default())
    }
}

/// A batch of glyphs to render.
#[derive(Debug, Clone)]
pub struct GlyphBatch {
    /// Atlas generation this batch was created for.
    pub atlas_generation: u64,
    /// Glyph instances.
    pub instances: Vec<GlyphInstance>,
}

/// A single glyph instance for rendering.
#[derive(Debug, Clone, Copy)]
pub struct GlyphInstance {
    /// Position on screen.
    pub position: Point,
    /// Atlas region UV coordinates [u0, v0, u1, v1].
    pub uv: [f32; 4],
    /// Glyph size.
    pub size: [f32; 2],
    /// Color (RGBA).
    pub color: [f32; 4],
    /// Atlas layer.
    pub layer: u32,
}

impl GlyphBatch {
    /// Create a new empty batch.
    pub fn new(atlas_generation: u64) -> Self {
        Self {
            atlas_generation,
            instances: Vec::new(),
        }
    }

    /// Add a glyph instance.
    pub fn add_glyph(
        &mut self,
        glyph: &CachedGlyph,
        position: Point,
        color: [f32; 4],
        atlas_size: (u32, u32),
    ) {
        let uv = glyph.region.uv_rect(atlas_size.0, atlas_size.1);

        self.instances.push(GlyphInstance {
            position: Point::new(position.x + glyph.offset.x, position.y + glyph.offset.y),
            uv,
            size: [glyph.region.width as f32, glyph.region.height as f32],
            color,
            layer: glyph.region.layer,
        });
    }

    /// Check if batch is empty.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Get number of glyphs in batch.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Clear the batch.
    pub fn clear(&mut self) {
        self.instances.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_key() {
        let key = GlyphKey::new(1, 65, 16.0, Point::new(0.25, 0.5));
        assert_eq!(key.font_id, 1);
        assert_eq!(key.glyph_id, 65);
        assert_eq!(key.size_px, 64); // 16 * 4
        assert_eq!(key.sub_pixel_x, 1); // 0.25 * 4
        assert_eq!(key.sub_pixel_y, 2); // 0.5 * 4
    }

    #[test]
    fn test_glyph_cache_insert_lookup() {
        let mut cache = GlyphCache::default();

        let key = GlyphKey::new(1, 65, 16.0, Point::zero());
        let region = cache.insert(key, 16, 20, Point::new(0.0, -15.0), 10.0);

        assert!(region.is_some());
        assert_eq!(cache.len(), 1);

        let glyph = cache.lookup(&key);
        assert!(glyph.is_some());
        assert_eq!(glyph.unwrap().advance, 10.0);
    }

    #[test]
    fn test_glyph_cache_eviction() {
        let config = GlyphCacheConfig {
            max_glyphs: 3,
            ..Default::default()
        };
        let mut cache = GlyphCache::new(config);

        // Insert 4 glyphs, should evict first
        for i in 0..4 {
            let key = GlyphKey::new(1, i, 16.0, Point::zero());
            cache.insert(key, 16, 16, Point::zero(), 10.0);
        }

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn test_glyph_cache_stats() {
        let mut cache = GlyphCache::default();

        let key = GlyphKey::new(1, 65, 16.0, Point::zero());
        cache.insert(key, 16, 16, Point::zero(), 10.0);

        // Miss
        let miss_key = GlyphKey::new(1, 66, 16.0, Point::zero());
        cache.lookup(&miss_key);

        // Hit
        cache.lookup(&key);

        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 1);
        assert!((cache.stats().hit_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_glyph_batch() {
        let mut batch = GlyphBatch::new(0);
        assert!(batch.is_empty());

        let glyph = CachedGlyph {
            region: AtlasRegion {
                x: 0,
                y: 0,
                width: 16,
                height: 20,
                layer: 0,
            },
            offset: Point::new(0.0, -15.0),
            advance: 10.0,
            bounds: Rect::from_xywh(0.0, 0.0, 16.0, 20.0),
        };

        batch.add_glyph(
            &glyph,
            Point::new(100.0, 100.0),
            [1.0, 1.0, 1.0, 1.0],
            (1024, 1024),
        );

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.instances[0].position.x, 100.0);
        assert_eq!(batch.instances[0].position.y, 85.0); // 100 + (-15)
    }

    #[test]
    fn test_glyph_cache_reset() {
        let mut cache = GlyphCache::default();

        let key = GlyphKey::new(1, 65, 16.0, Point::zero());
        cache.insert(key, 16, 16, Point::zero(), 10.0);
        assert_eq!(cache.len(), 1);

        cache.reset();
        assert!(cache.is_empty());
    }
}
