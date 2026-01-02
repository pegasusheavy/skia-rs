//! Region operations for complex clipping.
//!
//! This module provides Skia-compatible region types for representing
//! complex clip areas composed of multiple rectangles.

use crate::geometry::{IRect, Rect};

/// Operation type for combining regions.
///
/// Corresponds to Skia's `SkRegion::Op`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RegionOp {
    /// Subtract the second region from the first.
    Difference = 0,
    /// Intersect the two regions.
    Intersect,
    /// Union the two regions.
    Union,
    /// XOR the two regions (areas in one but not both).
    Xor,
    /// Reverse difference (subtract first from second).
    ReverseDifference,
    /// Replace with the second region.
    Replace,
}

/// A region composed of rectangles.
///
/// Represents a complex area that can be the result of multiple boolean
/// operations on rectangles. Used for efficient clipping operations.
///
/// Corresponds to Skia's `SkRegion`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Region {
    /// The rectangles composing this region (in scanline order).
    rects: Vec<IRect>,
    /// Cached bounds of the region.
    bounds: IRect,
}

impl Region {
    /// Create an empty region.
    #[inline]
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            bounds: IRect::empty(),
        }
    }

    /// Create a region from a single rectangle.
    pub fn from_rect(rect: IRect) -> Self {
        if rect.is_empty() {
            return Self::new();
        }
        Self {
            rects: vec![rect],
            bounds: rect,
        }
    }

    /// Create a region from a floating-point rectangle (rounds outward).
    pub fn from_rect_f(rect: &Rect) -> Self {
        Self::from_rect(rect.round_out())
    }

    /// Returns true if the region is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    /// Returns true if the region is a single rectangle.
    #[inline]
    pub fn is_rect(&self) -> bool {
        self.rects.len() == 1
    }

    /// Returns true if the region is complex (more than one rectangle).
    #[inline]
    pub fn is_complex(&self) -> bool {
        self.rects.len() > 1
    }

    /// Returns the bounds of the region.
    #[inline]
    pub fn bounds(&self) -> IRect {
        self.bounds
    }

    /// Returns the number of rectangles in the region.
    #[inline]
    pub fn rect_count(&self) -> usize {
        self.rects.len()
    }

    /// Returns an iterator over the rectangles in the region.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = IRect> + '_ {
        self.rects.iter().copied()
    }

    /// Returns the rectangles composing this region.
    #[inline]
    pub fn rects(&self) -> &[IRect] {
        &self.rects
    }

    /// Clear the region to empty.
    pub fn set_empty(&mut self) {
        self.rects.clear();
        self.bounds = IRect::empty();
    }

    /// Set the region to a single rectangle.
    pub fn set_rect(&mut self, rect: IRect) -> bool {
        if rect.is_empty() {
            self.set_empty();
            return false;
        }
        self.rects.clear();
        self.rects.push(rect);
        self.bounds = rect;
        true
    }

    /// Set the region to another region.
    pub fn set_region(&mut self, other: &Region) -> bool {
        self.rects = other.rects.clone();
        self.bounds = other.bounds;
        !self.is_empty()
    }

    /// Returns true if the point is contained in the region.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if !self.bounds.contains(x, y) {
            return false;
        }
        for rect in &self.rects {
            if rect.contains(x, y) {
                return true;
            }
        }
        false
    }

    /// Returns true if the rectangle is completely contained in the region.
    pub fn contains_rect(&self, rect: &IRect) -> bool {
        if rect.is_empty() {
            return true;
        }
        if self.is_empty() {
            return false;
        }

        // Quick bounds check
        if let Some(intersection) = self.bounds.intersect(rect) {
            if intersection != *rect {
                return false;
            }
        } else {
            return false;
        }

        // For a single rectangle, simple containment check
        if self.is_rect() {
            return true;
        }

        // For complex regions, we'd need a more sophisticated algorithm
        // This is a simplified check that may have false negatives
        for r in &self.rects {
            if r.left <= rect.left
                && r.top <= rect.top
                && r.right >= rect.right
                && r.bottom >= rect.bottom
            {
                return true;
            }
        }
        false
    }

    /// Returns true if this region intersects with a rectangle.
    pub fn intersects_rect(&self, rect: &IRect) -> bool {
        if self.is_empty() || rect.is_empty() {
            return false;
        }
        if self.bounds.intersect(rect).is_none() {
            return false;
        }
        for r in &self.rects {
            if r.intersect(rect).is_some() {
                return true;
            }
        }
        false
    }

    /// Returns true if this region intersects with another region.
    pub fn intersects_region(&self, other: &Region) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        if self.bounds.intersect(&other.bounds).is_none() {
            return false;
        }
        for r1 in &self.rects {
            for r2 in &other.rects {
                if r1.intersect(r2).is_some() {
                    return true;
                }
            }
        }
        false
    }

    /// Translate the region by (dx, dy).
    pub fn translate(&mut self, dx: i32, dy: i32) {
        for rect in &mut self.rects {
            *rect = rect.offset(dx, dy);
        }
        self.bounds = self.bounds.offset(dx, dy);
    }

    /// Returns a translated copy of this region.
    pub fn translated(&self, dx: i32, dy: i32) -> Self {
        let mut result = self.clone();
        result.translate(dx, dy);
        result
    }

    /// Combine this region with a rectangle using the specified operation.
    pub fn op_rect(&mut self, rect: IRect, op: RegionOp) -> bool {
        let other = Region::from_rect(rect);
        self.op_region(&other, op)
    }

    /// Combine this region with another region using the specified operation.
    pub fn op_region(&mut self, other: &Region, op: RegionOp) -> bool {
        match op {
            RegionOp::Replace => self.set_region(other),
            RegionOp::Intersect => self.intersect(other),
            RegionOp::Union => self.union(other),
            RegionOp::Xor => self.xor(other),
            RegionOp::Difference => self.difference(other),
            RegionOp::ReverseDifference => {
                let mut temp = other.clone();
                temp.difference(self);
                *self = temp;
                !self.is_empty()
            }
        }
    }

    /// Intersect this region with another.
    fn intersect(&mut self, other: &Region) -> bool {
        if self.is_empty() || other.is_empty() {
            self.set_empty();
            return false;
        }

        // Quick bounds check
        if self.bounds.intersect(&other.bounds).is_none() {
            self.set_empty();
            return false;
        }

        let mut result_rects = Vec::new();
        for r1 in &self.rects {
            for r2 in &other.rects {
                if let Some(intersection) = r1.intersect(r2) {
                    result_rects.push(intersection);
                }
            }
        }

        self.rects = result_rects;
        self.recompute_bounds();
        !self.is_empty()
    }

    /// Union this region with another.
    fn union(&mut self, other: &Region) -> bool {
        if other.is_empty() {
            return !self.is_empty();
        }
        if self.is_empty() {
            return self.set_region(other);
        }

        // Simple implementation: just combine all rectangles
        // A proper implementation would merge overlapping rectangles
        self.rects.extend(other.rects.iter().cloned());
        self.bounds = self.bounds.union(&other.bounds);
        true
    }

    /// XOR this region with another.
    fn xor(&mut self, other: &Region) -> bool {
        // XOR = (A - B) + (B - A)
        let mut a_minus_b = self.clone();
        a_minus_b.difference(other);

        let mut b_minus_a = other.clone();
        b_minus_a.difference(self);

        a_minus_b.union(&b_minus_a);
        *self = a_minus_b;
        !self.is_empty()
    }

    /// Subtract another region from this one.
    fn difference(&mut self, other: &Region) -> bool {
        if self.is_empty() || other.is_empty() {
            return !self.is_empty();
        }

        // Quick bounds check
        if self.bounds.intersect(&other.bounds).is_none() {
            return !self.is_empty();
        }

        // For each rectangle in self, subtract all rectangles from other
        let mut result_rects = Vec::new();
        for rect in &self.rects {
            let mut fragments = vec![*rect];
            for other_rect in &other.rects {
                let mut new_fragments = Vec::new();
                for frag in fragments {
                    new_fragments.extend(subtract_rect(&frag, other_rect));
                }
                fragments = new_fragments;
            }
            result_rects.extend(fragments);
        }

        self.rects = result_rects;
        self.recompute_bounds();
        !self.is_empty()
    }

    /// Recompute the bounds from the rectangles.
    fn recompute_bounds(&mut self) {
        if self.rects.is_empty() {
            self.bounds = IRect::empty();
            return;
        }

        self.bounds = self.rects[0];
        for rect in &self.rects[1..] {
            self.bounds = self.bounds.union(rect);
        }
    }
}

/// Subtract rect2 from rect1, returning the resulting fragments.
fn subtract_rect(rect1: &IRect, rect2: &IRect) -> Vec<IRect> {
    if rect1.intersect(rect2).is_none() {
        return vec![*rect1];
    }

    let mut result = Vec::new();

    // Top fragment
    if rect1.top < rect2.top {
        let top = IRect::new(rect1.left, rect1.top, rect1.right, rect2.top);
        if !top.is_empty() {
            result.push(top);
        }
    }

    // Bottom fragment
    if rect1.bottom > rect2.bottom {
        let bottom = IRect::new(rect1.left, rect2.bottom, rect1.right, rect1.bottom);
        if !bottom.is_empty() {
            result.push(bottom);
        }
    }

    // Left fragment (in the middle band)
    let middle_top = rect1.top.max(rect2.top);
    let middle_bottom = rect1.bottom.min(rect2.bottom);
    if middle_top < middle_bottom {
        if rect1.left < rect2.left {
            let left = IRect::new(rect1.left, middle_top, rect2.left, middle_bottom);
            if !left.is_empty() {
                result.push(left);
            }
        }

        // Right fragment (in the middle band)
        if rect1.right > rect2.right {
            let right = IRect::new(rect2.right, middle_top, rect1.right, middle_bottom);
            if !right.is_empty() {
                result.push(right);
            }
        }
    }

    result
}

/// Iterator over the rectangles in a region.
pub struct RegionIter<'a> {
    region: &'a Region,
    index: usize,
}

impl<'a> Iterator for RegionIter<'a> {
    type Item = &'a IRect;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.region.rects.len() {
            let rect = &self.region.rects[self.index];
            self.index += 1;
            Some(rect)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.region.rects.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for RegionIter<'a> {}

impl<'a> IntoIterator for &'a Region {
    type Item = &'a IRect;
    type IntoIter = RegionIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RegionIter {
            region: self,
            index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_region() {
        let region = Region::new();
        assert!(region.is_empty());
        assert!(!region.is_rect());
        assert!(!region.is_complex());
    }

    #[test]
    fn test_rect_region() {
        let rect = IRect::new(10, 20, 100, 200);
        let region = Region::from_rect(rect);
        assert!(!region.is_empty());
        assert!(region.is_rect());
        assert!(!region.is_complex());
        assert_eq!(region.bounds(), rect);
    }

    #[test]
    fn test_contains_point() {
        let region = Region::from_rect(IRect::new(0, 0, 100, 100));
        assert!(region.contains(50, 50));
        assert!(region.contains(0, 0));
        assert!(!region.contains(100, 100)); // Exclusive
        assert!(!region.contains(-1, 50));
    }

    #[test]
    fn test_translate() {
        let mut region = Region::from_rect(IRect::new(0, 0, 100, 100));
        region.translate(50, 50);
        assert_eq!(region.bounds(), IRect::new(50, 50, 150, 150));
    }

    #[test]
    fn test_intersect() {
        let mut region = Region::from_rect(IRect::new(0, 0, 100, 100));
        region.op_rect(IRect::new(50, 50, 150, 150), RegionOp::Intersect);
        assert_eq!(region.bounds(), IRect::new(50, 50, 100, 100));
    }

    #[test]
    fn test_union() {
        let mut region = Region::from_rect(IRect::new(0, 0, 50, 50));
        region.op_rect(IRect::new(50, 50, 100, 100), RegionOp::Union);
        assert_eq!(region.bounds(), IRect::new(0, 0, 100, 100));
        assert!(region.is_complex());
    }

    #[test]
    fn test_difference() {
        let mut region = Region::from_rect(IRect::new(0, 0, 100, 100));
        region.op_rect(IRect::new(25, 25, 75, 75), RegionOp::Difference);
        assert!(!region.is_empty());
        assert!(region.is_complex());
        // Should have 4 fragments around the hole
        assert_eq!(region.rect_count(), 4);
    }
}
