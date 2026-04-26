//! Advanced clipping operations for the rasterizer.
//!
//! This module provides:
//! - **Anti-aliased clipping** using coverage masks
//! - **Region-based clipping** for complex clip shapes
//! - **Clip stack** for save/restore semantics
//!
//! ## Anti-Aliased Clipping
//!
//! Anti-aliased clips use a coverage mask to store per-pixel alpha values,
//! allowing smooth clip edges without jagged artifacts. The coverage is
//! computed by rasterizing the clip path with sub-pixel precision.
//!
//! ## Region-Based Clipping
//!
//! Region clips use `skia_rs_core::Region` to represent complex clip areas
//! composed of multiple rectangles. This is efficient for non-anti-aliased
//! clips with complex shapes.

use skia_rs_core::{IRect, Point, Rect, Region, RegionOp, Scalar};
use skia_rs_path::Path;

/// A coverage mask for anti-aliased clipping.
///
/// Stores per-pixel coverage values (0-255) where 0 is fully clipped
/// and 255 is fully visible. Intermediate values provide smooth edges.
#[derive(Debug, Clone)]
pub struct ClipMask {
    /// Width in pixels.
    width: i32,
    /// Height in pixels.
    height: i32,
    /// Coverage data (one byte per pixel).
    coverage: Vec<u8>,
    /// Bounds of the mask in device coordinates.
    bounds: IRect,
}

impl ClipMask {
    /// Create a new clip mask filled with the given coverage value.
    pub fn new(width: i32, height: i32, initial_coverage: u8) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            coverage: vec![initial_coverage; size],
            bounds: IRect::new(0, 0, width, height),
        }
    }

    /// Create a clip mask from a rectangle with anti-aliased edges.
    pub fn from_rect_aa(rect: &Rect, device_bounds: &IRect) -> Self {
        let width = device_bounds.width();
        let height = device_bounds.height();
        let mut mask = Self::new(width, height, 0);

        // Rasterize the rectangle with sub-pixel coverage
        let left = rect.left - device_bounds.left as f32;
        let top = rect.top - device_bounds.top as f32;
        let right = rect.right - device_bounds.left as f32;
        let bottom = rect.bottom - device_bounds.top as f32;

        for y in 0..height {
            for x in 0..width {
                let px = x as f32;
                let py = y as f32;

                // Calculate coverage for this pixel
                let coverage = compute_rect_coverage(px, py, left, top, right, bottom);
                mask.set_coverage(x, y, coverage);
            }
        }

        mask.bounds = *device_bounds;
        mask
    }

    /// Create a clip mask from a path with anti-aliased edges.
    pub fn from_path_aa(path: &Path, device_bounds: &IRect) -> Self {
        let width = device_bounds.width();
        let height = device_bounds.height();
        let mut mask = Self::new(width, height, 0);

        // Use supersampling for path coverage
        const SAMPLES: i32 = 4;
        let sample_offsets: [(f32, f32); 16] = [
            (0.125, 0.125),
            (0.375, 0.125),
            (0.625, 0.125),
            (0.875, 0.125),
            (0.125, 0.375),
            (0.375, 0.375),
            (0.625, 0.375),
            (0.875, 0.375),
            (0.125, 0.625),
            (0.375, 0.625),
            (0.625, 0.625),
            (0.875, 0.625),
            (0.125, 0.875),
            (0.375, 0.875),
            (0.625, 0.875),
            (0.875, 0.875),
        ];

        for y in 0..height {
            for x in 0..width {
                let px = (x + device_bounds.left) as f32;
                let py = (y + device_bounds.top) as f32;

                // Count samples inside the path
                let mut inside_count = 0;
                for (ox, oy) in &sample_offsets {
                    let sample_x = px + ox;
                    let sample_y = py + oy;
                    if path.contains(Point::new(sample_x, sample_y)) {
                        inside_count += 1;
                    }
                }

                let coverage = ((inside_count * 255) / 16) as u8;
                mask.set_coverage(x, y, coverage);
            }
        }

        mask.bounds = *device_bounds;
        mask
    }

    /// Get the coverage value at (x, y) in local coordinates.
    #[inline]
    pub fn get_coverage(&self, x: i32, y: i32) -> u8 {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return 0;
        }
        self.coverage[(y * self.width + x) as usize]
    }

    /// Get the coverage value at device coordinates.
    #[inline]
    pub fn get_coverage_device(&self, x: i32, y: i32) -> u8 {
        let lx = x - self.bounds.left;
        let ly = y - self.bounds.top;
        self.get_coverage(lx, ly)
    }

    /// Set the coverage value at (x, y).
    #[inline]
    pub fn set_coverage(&mut self, x: i32, y: i32, coverage: u8) {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            self.coverage[(y * self.width + x) as usize] = coverage;
        }
    }

    /// Returns the bounds of this mask.
    #[inline]
    pub fn bounds(&self) -> IRect {
        self.bounds
    }

    /// Returns the width of this mask.
    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns the height of this mask.
    #[inline]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Intersect this mask with another mask.
    pub fn intersect(&mut self, other: &ClipMask) {
        // Find intersection bounds
        let Some(intersection) = self.bounds.intersect(&other.bounds) else {
            // No intersection - clear the mask
            self.coverage.fill(0);
            return;
        };

        // For pixels in the intersection, multiply coverage
        for y in intersection.top..intersection.bottom {
            for x in intersection.left..intersection.right {
                let self_cov = self.get_coverage_device(x, y) as u32;
                let other_cov = other.get_coverage_device(x, y) as u32;
                let combined = ((self_cov * other_cov) / 255) as u8;

                let lx = x - self.bounds.left;
                let ly = y - self.bounds.top;
                self.set_coverage(lx, ly, combined);
            }
        }

        // Clear pixels outside the intersection
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x + self.bounds.left;
                let dy = y + self.bounds.top;
                if dx < intersection.left
                    || dx >= intersection.right
                    || dy < intersection.top
                    || dy >= intersection.bottom
                {
                    self.coverage[(y * self.width + x) as usize] = 0;
                }
            }
        }
    }

    /// Apply a rectangular clip to this mask.
    pub fn clip_rect(&mut self, rect: &IRect) {
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x + self.bounds.left;
                let dy = y + self.bounds.top;
                if !rect.contains(dx, dy) {
                    self.coverage[(y * self.width + x) as usize] = 0;
                }
            }
        }
    }
}

/// Compute rectangle coverage for a pixel.
fn compute_rect_coverage(px: f32, py: f32, left: f32, top: f32, right: f32, bottom: f32) -> u8 {
    // Calculate how much of the pixel is inside the rectangle
    let x_coverage = (right.min(px + 1.0) - left.max(px)).clamp(0.0, 1.0);
    let y_coverage = (bottom.min(py + 1.0) - top.max(py)).clamp(0.0, 1.0);
    let coverage = x_coverage * y_coverage;
    (coverage * 255.0) as u8
}

/// Represents a clip state that can be either rectangular, regional, or masked.
#[derive(Debug, Clone)]
pub enum ClipState {
    /// Simple rectangular clip (fast path).
    Rect(Rect),
    /// Region-based clip (multiple rectangles).
    Region(Region),
    /// Anti-aliased clip with coverage mask.
    Mask(ClipMask),
    /// Combined region and mask clip.
    RegionAndMask(Region, ClipMask),
}

impl ClipState {
    /// Create a clip state from a rectangle.
    pub fn from_rect(rect: Rect) -> Self {
        ClipState::Rect(rect)
    }

    /// Create a clip state from a region.
    pub fn from_region(region: Region) -> Self {
        ClipState::Region(region)
    }

    /// Create an anti-aliased clip state from a rectangle.
    pub fn from_rect_aa(rect: &Rect, device_bounds: &IRect) -> Self {
        ClipState::Mask(ClipMask::from_rect_aa(rect, device_bounds))
    }

    /// Create an anti-aliased clip state from a path.
    pub fn from_path_aa(path: &Path, device_bounds: &IRect) -> Self {
        ClipState::Mask(ClipMask::from_path_aa(path, device_bounds))
    }

    /// Get the bounding rectangle of this clip.
    pub fn bounds(&self) -> Rect {
        match self {
            ClipState::Rect(r) => *r,
            ClipState::Region(r) => {
                let b = r.bounds();
                Rect::new(b.left as f32, b.top as f32, b.right as f32, b.bottom as f32)
            }
            ClipState::Mask(m) => {
                let b = m.bounds();
                Rect::new(b.left as f32, b.top as f32, b.right as f32, b.bottom as f32)
            }
            ClipState::RegionAndMask(r, _) => {
                let b = r.bounds();
                Rect::new(b.left as f32, b.top as f32, b.right as f32, b.bottom as f32)
            }
        }
    }

    /// Check if a point is inside the clip.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        match self {
            ClipState::Rect(r) => r.contains(Point::new(x as f32, y as f32)),
            ClipState::Region(r) => r.contains(x, y),
            ClipState::Mask(m) => m.get_coverage_device(x, y) > 0,
            ClipState::RegionAndMask(r, m) => r.contains(x, y) && m.get_coverage_device(x, y) > 0,
        }
    }

    /// Get the coverage at a point (0-255).
    pub fn get_coverage(&self, x: i32, y: i32) -> u8 {
        match self {
            ClipState::Rect(r) => {
                if r.contains(Point::new(x as f32, y as f32)) {
                    255
                } else {
                    0
                }
            }
            ClipState::Region(r) => {
                if r.contains(x, y) {
                    255
                } else {
                    0
                }
            }
            ClipState::Mask(m) => m.get_coverage_device(x, y),
            ClipState::RegionAndMask(r, m) => {
                if r.contains(x, y) {
                    m.get_coverage_device(x, y)
                } else {
                    0
                }
            }
        }
    }

    /// Check if this is an anti-aliased clip.
    pub fn is_anti_aliased(&self) -> bool {
        matches!(self, ClipState::Mask(_) | ClipState::RegionAndMask(_, _))
    }

    /// Intersect this clip with a rectangle.
    pub fn intersect_rect(&mut self, rect: &Rect) {
        match self {
            ClipState::Rect(r) => {
                if let Some(intersection) = r.intersect(rect) {
                    *r = intersection;
                } else {
                    *r = Rect::EMPTY;
                }
            }
            ClipState::Region(r) => {
                r.op_rect(rect.round_out(), skia_rs_core::RegionOp::Intersect);
            }
            ClipState::Mask(m) => {
                m.clip_rect(&rect.round_out());
            }
            ClipState::RegionAndMask(r, m) => {
                let irect = rect.round_out();
                r.op_rect(irect, skia_rs_core::RegionOp::Intersect);
                m.clip_rect(&irect);
            }
        }
    }

    /// Intersect this clip with a region.
    pub fn intersect_region(&mut self, region: &Region) {
        match self {
            ClipState::Rect(r) => {
                let mut new_region = Region::from_rect_f(r);
                new_region.op_region(region, skia_rs_core::RegionOp::Intersect);
                *self = ClipState::Region(new_region);
            }
            ClipState::Region(r) => {
                r.op_region(region, skia_rs_core::RegionOp::Intersect);
            }
            ClipState::Mask(m) => {
                // Convert region to mask intersection
                for y in 0..m.height {
                    for x in 0..m.width {
                        let dx = x + m.bounds.left;
                        let dy = y + m.bounds.top;
                        if !region.contains(dx, dy) {
                            m.set_coverage(x, y, 0);
                        }
                    }
                }
            }
            ClipState::RegionAndMask(r, m) => {
                r.op_region(region, skia_rs_core::RegionOp::Intersect);
                // Also update mask
                for y in 0..m.height {
                    for x in 0..m.width {
                        let dx = x + m.bounds.left;
                        let dy = y + m.bounds.top;
                        if !region.contains(dx, dy) {
                            m.set_coverage(x, y, 0);
                        }
                    }
                }
            }
        }
    }

    /// Subtract a rectangle from the current clip (difference operation).
    pub fn difference_rect(&mut self, rect: &Rect) {
        match self {
            ClipState::Rect(r) => {
                // Convert to region for difference operation
                let mut region = Region::from_rect_f(r);
                region.op_rect(rect.round_out(), skia_rs_core::RegionOp::Difference);
                *self = ClipState::Region(region);
            }
            ClipState::Region(r) => {
                r.op_rect(rect.round_out(), skia_rs_core::RegionOp::Difference);
            }
            ClipState::Mask(m) => {
                // Zero out coverage in the rect area
                let irect = rect.round_out();
                for y in 0..m.height {
                    for x in 0..m.width {
                        let dx = x + m.bounds.left;
                        let dy = y + m.bounds.top;
                        if irect.contains(dx, dy) {
                            m.set_coverage(x, y, 0);
                        }
                    }
                }
            }
            ClipState::RegionAndMask(r, m) => {
                r.op_rect(rect.round_out(), skia_rs_core::RegionOp::Difference);
                // Also zero out coverage in the rect area
                let irect = rect.round_out();
                for y in 0..m.height {
                    for x in 0..m.width {
                        let dx = x + m.bounds.left;
                        let dy = y + m.bounds.top;
                        if irect.contains(dx, dy) {
                            m.set_coverage(x, y, 0);
                        }
                    }
                }
            }
        }
    }
}

/// A stack of clip states for save/restore semantics.
#[derive(Debug, Clone)]
pub struct ClipStack {
    /// Stack of saved clip states.
    stack: Vec<ClipState>,
    /// Current clip state.
    current: ClipState,
}

impl ClipStack {
    /// Create a new clip stack with the given device bounds.
    pub fn new(device_bounds: &Rect) -> Self {
        Self {
            stack: Vec::new(),
            current: ClipState::Rect(*device_bounds),
        }
    }

    /// Save the current clip state.
    pub fn save(&mut self) {
        self.stack.push(self.current.clone());
    }

    /// Restore the previous clip state.
    pub fn restore(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.current = state;
        }
    }

    /// Get the current save count.
    pub fn save_count(&self) -> usize {
        self.stack.len()
    }

    /// Restore to a specific save count.
    pub fn restore_to_count(&mut self, count: usize) {
        while self.stack.len() > count {
            self.restore();
        }
    }

    /// Get the current clip state.
    pub fn current(&self) -> &ClipState {
        &self.current
    }

    /// Get the current clip bounds.
    pub fn bounds(&self) -> Rect {
        self.current.bounds()
    }

    /// Check if a point is inside the current clip.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.current.contains(x, y)
    }

    /// Get the coverage at a point.
    pub fn get_coverage(&self, x: i32, y: i32) -> u8 {
        self.current.get_coverage(x, y)
    }

    /// Intersect the current clip with a rectangle.
    pub fn clip_rect(&mut self, rect: &Rect) {
        self.current.intersect_rect(rect);
    }

    /// Apply a clip operation with a rectangle.
    pub fn clip_rect_op(&mut self, rect: &Rect, op: super::ClipOp) {
        use super::ClipOp;
        match op {
            ClipOp::Intersect => self.current.intersect_rect(rect),
            ClipOp::Difference => self.current.difference_rect(rect),
        }
    }

    /// Intersect the current clip with a rectangle (anti-aliased).
    pub fn clip_rect_aa(&mut self, rect: &Rect, device_bounds: &IRect) {
        let mask = ClipMask::from_rect_aa(rect, device_bounds);
        match &mut self.current {
            ClipState::Rect(r) => {
                let mut new_mask = mask;
                new_mask.clip_rect(&r.round_out());
                self.current = ClipState::Mask(new_mask);
            }
            ClipState::Region(r) => {
                let mut new_mask = mask;
                for y in 0..new_mask.height {
                    for x in 0..new_mask.width {
                        let dx = x + new_mask.bounds.left;
                        let dy = y + new_mask.bounds.top;
                        if !r.contains(dx, dy) {
                            new_mask.set_coverage(x, y, 0);
                        }
                    }
                }
                self.current = ClipState::Mask(new_mask);
            }
            ClipState::Mask(m) => {
                m.intersect(&mask);
            }
            ClipState::RegionAndMask(r, m) => {
                // Intersect both the region and the mask
                let rounded_rect = IRect::new(
                    rect.left.floor() as i32,
                    rect.top.floor() as i32,
                    rect.right.ceil() as i32,
                    rect.bottom.ceil() as i32,
                );
                r.op_rect(rounded_rect, RegionOp::Intersect);
                m.intersect(&mask);
            }
        }
    }

    /// Intersect the current clip with a region.
    pub fn clip_region(&mut self, region: &Region) {
        self.current.intersect_region(region);
    }

    /// Apply a clip operation with a rectangle, optionally anti-aliased.
    pub fn clip_rect_with_op(&mut self, rect: &Rect, op: super::ClipOp, device_bounds: &IRect, anti_alias: bool) {
        use super::ClipOp;
        if anti_alias {
            // For AA clips, we need to create a mask
            self.clip_rect_aa(rect, device_bounds);
            // Note: Difference with AA is approximated via mask zeroing
            if op == ClipOp::Difference {
                // The AA path already created a mask; now apply difference
                match &mut self.current {
                    ClipState::Mask(m) => {
                        // Invert the mask coverage (AA difference approximation)
                        let irect = rect.round_out();
                        for y in 0..m.height {
                            for x in 0..m.width {
                                let dx = x + m.bounds.left;
                                let dy = y + m.bounds.top;
                                if irect.contains(dx, dy) {
                                    m.set_coverage(x, y, 0);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else {
            self.clip_rect_op(rect, op);
        }
    }

    /// Intersect the current clip with a path.
    pub fn clip_path(&mut self, path: &Path, device_bounds: &IRect, anti_alias: bool) {
        if anti_alias {
            let mask = ClipMask::from_path_aa(path, device_bounds);
            match &mut self.current {
                ClipState::Rect(r) => {
                    let mut new_mask = mask;
                    new_mask.clip_rect(&r.round_out());
                    self.current = ClipState::Mask(new_mask);
                }
                ClipState::Region(r) => {
                    self.current = ClipState::RegionAndMask(r.clone(), mask);
                }
                ClipState::Mask(m) => {
                    m.intersect(&mask);
                }
                ClipState::RegionAndMask(_, m) => {
                    m.intersect(&mask);
                }
            }
        } else {
            // Non-AA path clip - convert path bounds to region
            let bounds = path.bounds();
            let region = Region::from_rect_f(&bounds);
            self.current.intersect_region(&region);
        }
    }

    /// Apply a clip operation with a path, optionally anti-aliased.
    pub fn clip_path_with_op(&mut self, path: &Path, device_bounds: &IRect, op: super::ClipOp, anti_alias: bool) {
        use super::ClipOp;
        match op {
            ClipOp::Intersect => self.clip_path(path, device_bounds, anti_alias),
            ClipOp::Difference => {
                // For difference, we need to handle path clipping differently
                // Currently only intersect is fully supported for paths
                // Difference with paths requires more complex region operations
                // For now, approximate with bounds
                if anti_alias {
                    // Create a mask and invert it in the path area
                    let mask = ClipMask::from_path_aa(path, device_bounds);
                    // Apply difference by zeroing coverage where path is
                    match &mut self.current {
                        ClipState::Rect(r) => {
                            let mut region = Region::from_rect_f(r);
                            let path_region = Region::from_rect_f(&path.bounds());
                            region.op_region(&path_region, skia_rs_core::RegionOp::Difference);
                            *self = ClipStack {
                                stack: self.stack.clone(),
                                current: ClipState::RegionAndMask(region, mask),
                            };
                        }
                        ClipState::Region(r) => {
                            let path_region = Region::from_rect_f(&path.bounds());
                            r.op_region(&path_region, skia_rs_core::RegionOp::Difference);
                        }
                        ClipState::Mask(m) => {
                            // Zero out coverage where the path mask is non-zero
                            for y in 0..m.height.min(mask.height) {
                                for x in 0..m.width.min(mask.width) {
                                    if mask.get_coverage(x, y) > 0 {
                                        m.set_coverage(x, y, 0);
                                    }
                                }
                            }
                        }
                        ClipState::RegionAndMask(r, m) => {
                            let path_region = Region::from_rect_f(&path.bounds());
                            r.op_region(&path_region, skia_rs_core::RegionOp::Difference);
                            // Zero out coverage where the path mask is non-zero
                            for y in 0..m.height.min(mask.height) {
                                for x in 0..m.width.min(mask.width) {
                                    if mask.get_coverage(x, y) > 0 {
                                        m.set_coverage(x, y, 0);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let bounds = path.bounds();
                    let region = Region::from_rect_f(&bounds);
                    match &mut self.current {
                        ClipState::Rect(r) => {
                            let mut new_region = Region::from_rect_f(r);
                            new_region.op_region(&region, skia_rs_core::RegionOp::Difference);
                            *self = ClipStack {
                                stack: self.stack.clone(),
                                current: ClipState::Region(new_region),
                            };
                        }
                        ClipState::Region(r) => {
                            r.op_region(&region, skia_rs_core::RegionOp::Difference);
                        }
                        ClipState::Mask(_) | ClipState::RegionAndMask(_, _) => {
                            // Can't directly difference region from mask
                            // This is a limitation we'll document
                        }
                    }
                }
            }
        }
    }

    /// Check if the current clip is anti-aliased.
    pub fn is_anti_aliased(&self) -> bool {
        self.current.is_anti_aliased()
    }

    /// Reset the clip to device bounds.
    pub fn reset(&mut self, device_bounds: &Rect) {
        self.stack.clear();
        self.current = ClipState::Rect(*device_bounds);
    }

    /// Check if a rectangle is completely outside the current clip.
    pub fn quick_reject(&self, rect: &Rect) -> bool {
        let clip_bounds = self.bounds();
        !clip_bounds.intersects(rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_mask_new() {
        let mask = ClipMask::new(100, 100, 255);
        assert_eq!(mask.width(), 100);
        assert_eq!(mask.height(), 100);
        assert_eq!(mask.get_coverage(50, 50), 255);
    }

    #[test]
    fn test_clip_mask_from_rect_aa() {
        let rect = Rect::new(10.5, 10.5, 90.5, 90.5);
        let bounds = IRect::new(0, 0, 100, 100);
        let mask = ClipMask::from_rect_aa(&rect, &bounds);

        // Fully inside should be 255
        assert_eq!(mask.get_coverage(50, 50), 255);

        // Fully outside should be 0
        assert_eq!(mask.get_coverage(0, 0), 0);
        assert_eq!(mask.get_coverage(99, 99), 0);

        // Edge pixels should have partial coverage
        let edge_coverage = mask.get_coverage(10, 50);
        assert!(edge_coverage > 0 && edge_coverage < 255);
    }

    #[test]
    fn test_clip_state_rect() {
        let state = ClipState::from_rect(Rect::new(10.0, 10.0, 90.0, 90.0));

        assert!(state.contains(50, 50));
        assert!(!state.contains(0, 0));
        assert_eq!(state.get_coverage(50, 50), 255);
        assert_eq!(state.get_coverage(0, 0), 0);
    }

    #[test]
    fn test_clip_state_region() {
        let mut region = Region::new();
        region.set_rect(IRect::new(10, 10, 50, 50));
        region.op_rect(IRect::new(50, 50, 90, 90), skia_rs_core::RegionOp::Union);

        let state = ClipState::from_region(region);

        assert!(state.contains(25, 25)); // In first rect
        assert!(state.contains(75, 75)); // In second rect
        assert!(!state.contains(25, 75)); // Not in either
    }

    #[test]
    fn test_clip_stack_save_restore() {
        let mut stack = ClipStack::new(&Rect::new(0.0, 0.0, 100.0, 100.0));

        assert_eq!(stack.save_count(), 0);

        stack.save();
        assert_eq!(stack.save_count(), 1);

        stack.clip_rect(&Rect::new(10.0, 10.0, 50.0, 50.0));
        assert!(stack.contains(25, 25));
        assert!(!stack.contains(75, 75));

        stack.restore();
        assert_eq!(stack.save_count(), 0);
        assert!(stack.contains(75, 75)); // Original clip restored
    }

    #[test]
    fn test_clip_stack_nested() {
        let mut stack = ClipStack::new(&Rect::new(0.0, 0.0, 100.0, 100.0));

        stack.save();
        stack.clip_rect(&Rect::new(0.0, 0.0, 50.0, 100.0));

        stack.save();
        stack.clip_rect(&Rect::new(0.0, 0.0, 100.0, 50.0));

        // Should be clipped to intersection: (0,0) - (50,50)
        assert!(stack.contains(25, 25));
        assert!(!stack.contains(75, 25)); // Outside horizontal clip
        assert!(!stack.contains(25, 75)); // Outside vertical clip

        stack.restore();
        // Back to horizontal clip only
        assert!(stack.contains(25, 75));

        stack.restore();
        // Back to full bounds
        assert!(stack.contains(75, 75));
    }

    #[test]
    fn test_clip_region_integration() {
        let mut stack = ClipStack::new(&Rect::new(0.0, 0.0, 100.0, 100.0));

        let mut region = Region::new();
        region.set_rect(IRect::new(0, 0, 50, 50));
        region.op_rect(IRect::new(50, 50, 100, 100), skia_rs_core::RegionOp::Union);

        stack.clip_region(&region);

        // L-shaped region
        assert!(stack.contains(25, 25)); // Top-left
        assert!(stack.contains(75, 75)); // Bottom-right
        assert!(!stack.contains(75, 25)); // Top-right (not in region)
        assert!(!stack.contains(25, 75)); // Bottom-left (not in region)
    }

    #[test]
    fn test_compute_rect_coverage() {
        // Fully inside
        assert_eq!(compute_rect_coverage(5.0, 5.0, 0.0, 0.0, 10.0, 10.0), 255);

        // Fully outside
        assert_eq!(compute_rect_coverage(15.0, 15.0, 0.0, 0.0, 10.0, 10.0), 0);

        // Partial coverage (50% horizontal)
        let coverage = compute_rect_coverage(9.0, 5.0, 0.0, 0.0, 9.5, 10.0);
        assert!(coverage > 0 && coverage < 255);
    }

    #[test]
    fn test_clip_rect_aa_preserves_region_on_region_and_mask() {
        use skia_rs_path::PathBuilder;

        // Create a clip stack with a RegionAndMask state and verify that
        // clip_rect_aa intersects both the region and the mask components
        let device_bounds = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let device_bounds_i = IRect::new(0, 0, 100, 100);
        let mut stack = ClipStack::new(&device_bounds);

        // Set up an initial rect clip
        let initial_rect = Rect::from_xywh(10.0, 10.0, 50.0, 50.0);
        stack.clip_rect(&initial_rect);

        // Apply a path clip to force RegionAndMask state
        let mut builder = PathBuilder::new();
        builder.add_rect(&Rect::from_xywh(15.0, 15.0, 40.0, 40.0));
        let path = builder.build();
        stack.clip_path(&path, &device_bounds_i, true);

        // Now apply an AA rect clip
        let aa_rect = Rect::from_xywh(20.0, 20.0, 30.0, 30.0);
        stack.clip_rect_aa(&aa_rect, &device_bounds_i);

        // Test points that should be clipped based on the region intersection
        // Point outside the AA rect but inside the mask should be clipped
        assert!(!stack.contains(5, 5), "Point outside AA rect should be clipped");

        // Point inside the AA rect intersection should be included
        assert!(stack.contains(25, 25), "Point inside intersection should be included");

        // Point outside the AA rect should be clipped even if mask would include it
        assert!(!stack.contains(55, 55), "Point outside AA rect should be clipped");
    }
}
