//! Memory usage profiling for skia-rs.
//!
//! This module provides tools for measuring memory allocation patterns,
//! peak memory usage, and allocation counts for various operations.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// =============================================================================
// Tracking Allocator
// =============================================================================

/// A global allocator that tracks memory usage.
///
/// This allocator wraps the system allocator and counts:
/// - Total bytes allocated
/// - Total bytes deallocated
/// - Peak memory usage
/// - Number of allocations
/// - Number of deallocations
pub struct TrackingAllocator {
    /// Inner system allocator.
    inner: System,
}

impl TrackingAllocator {
    /// Create a new tracking allocator.
    pub const fn new() -> Self {
        Self { inner: System }
    }
}

// Global counters
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static TRACKING_ENABLED: AtomicBool = AtomicBool::new(false);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);

        if TRACKING_ENABLED.load(Ordering::Relaxed) && !ptr.is_null() {
            let size = layout.size();
            let total = ALLOCATED.fetch_add(size, Ordering::Relaxed) + size;
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);

            // Update peak
            let current = total - DEALLOCATED.load(Ordering::Relaxed);
            let mut peak = PEAK.load(Ordering::Relaxed);
            while current > peak {
                match PEAK.compare_exchange_weak(
                    peak,
                    current,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if TRACKING_ENABLED.load(Ordering::Relaxed) {
            DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        // SAFETY: We're delegating to the inner allocator
        unsafe { self.inner.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: We're delegating to the inner allocator
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };

        if TRACKING_ENABLED.load(Ordering::Relaxed) && !new_ptr.is_null() {
            let old_size = layout.size();
            if new_size > old_size {
                let diff = new_size - old_size;
                let total = ALLOCATED.fetch_add(diff, Ordering::Relaxed) + diff;
                ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);

                // Update peak
                let current = total - DEALLOCATED.load(Ordering::Relaxed);
                let mut peak = PEAK.load(Ordering::Relaxed);
                while current > peak {
                    match PEAK.compare_exchange_weak(
                        peak,
                        current,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(p) => peak = p,
                    }
                }
            } else {
                DEALLOCATED.fetch_add(old_size - new_size, Ordering::Relaxed);
            }
        }

        new_ptr
    }
}

// =============================================================================
// Memory Stats
// =============================================================================

/// Memory usage statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    /// Total bytes allocated during measurement.
    pub allocated: usize,
    /// Total bytes deallocated during measurement.
    pub deallocated: usize,
    /// Peak memory usage (high water mark).
    pub peak: usize,
    /// Number of allocations.
    pub alloc_count: usize,
    /// Number of deallocations.
    pub dealloc_count: usize,
}

impl MemoryStats {
    /// Get the current memory in use.
    pub fn current(&self) -> usize {
        self.allocated.saturating_sub(self.deallocated)
    }

    /// Get bytes per allocation (average).
    pub fn bytes_per_alloc(&self) -> f64 {
        if self.alloc_count == 0 {
            0.0
        } else {
            self.allocated as f64 / self.alloc_count as f64
        }
    }

    /// Format stats as human-readable string.
    pub fn format(&self) -> String {
        format!(
            "Memory: {} allocated, {} peak, {} allocs ({:.1} bytes/alloc)",
            format_bytes(self.allocated),
            format_bytes(self.peak),
            self.alloc_count,
            self.bytes_per_alloc()
        )
    }
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Format bytes as human-readable string.
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// =============================================================================
// Measurement API
// =============================================================================

/// Reset all memory counters.
pub fn reset_counters() {
    ALLOCATED.store(0, Ordering::SeqCst);
    DEALLOCATED.store(0, Ordering::SeqCst);
    PEAK.store(0, Ordering::SeqCst);
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    DEALLOC_COUNT.store(0, Ordering::SeqCst);
}

/// Enable memory tracking.
pub fn enable_tracking() {
    TRACKING_ENABLED.store(true, Ordering::SeqCst);
}

/// Disable memory tracking.
pub fn disable_tracking() {
    TRACKING_ENABLED.store(false, Ordering::SeqCst);
}

/// Check if tracking is enabled.
pub fn is_tracking_enabled() -> bool {
    TRACKING_ENABLED.load(Ordering::SeqCst)
}

/// Get current memory statistics.
pub fn get_stats() -> MemoryStats {
    MemoryStats {
        allocated: ALLOCATED.load(Ordering::SeqCst),
        deallocated: DEALLOCATED.load(Ordering::SeqCst),
        peak: PEAK.load(Ordering::SeqCst),
        alloc_count: ALLOC_COUNT.load(Ordering::SeqCst),
        dealloc_count: DEALLOC_COUNT.load(Ordering::SeqCst),
    }
}

/// RAII guard for memory measurement.
///
/// Automatically enables tracking on creation and captures stats on drop.
pub struct MemoryMeasurement {
    start_stats: MemoryStats,
    label: String,
}

impl MemoryMeasurement {
    /// Start a new memory measurement with a label.
    pub fn start(label: impl Into<String>) -> Self {
        reset_counters();
        enable_tracking();

        Self {
            start_stats: get_stats(),
            label: label.into(),
        }
    }

    /// Get the current stats (delta from start).
    pub fn current(&self) -> MemoryStats {
        let now = get_stats();
        MemoryStats {
            allocated: now.allocated - self.start_stats.allocated,
            deallocated: now.deallocated - self.start_stats.deallocated,
            peak: now.peak,
            alloc_count: now.alloc_count - self.start_stats.alloc_count,
            dealloc_count: now.dealloc_count - self.start_stats.dealloc_count,
        }
    }

    /// Finish measurement and return stats.
    pub fn finish(self) -> MemoryStats {
        let stats = self.current();
        disable_tracking();
        stats
    }
}

impl Drop for MemoryMeasurement {
    fn drop(&mut self) {
        disable_tracking();
    }
}

/// Measure memory usage of a closure.
///
/// # Example
///
/// ```ignore
/// let stats = measure_memory("create surface", || {
///     Surface::new_raster_n32_premul(1000, 1000)
/// });
/// println!("{}", stats);
/// ```
pub fn measure_memory<T, F: FnOnce() -> T>(label: &str, f: F) -> (T, MemoryStats) {
    let measurement = MemoryMeasurement::start(label);
    let result = f();
    let stats = measurement.finish();
    (result, stats)
}

// =============================================================================
// Memory Profile Report
// =============================================================================

/// A collection of memory measurements.
#[derive(Default)]
pub struct MemoryProfile {
    measurements: Vec<(String, MemoryStats)>,
}

impl MemoryProfile {
    /// Create a new memory profile.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a measurement.
    pub fn add(&mut self, label: impl Into<String>, stats: MemoryStats) {
        self.measurements.push((label.into(), stats));
    }

    /// Run a closure and add its memory stats.
    pub fn measure<T, F: FnOnce() -> T>(&mut self, label: &str, f: F) -> T {
        let (result, stats) = measure_memory(label, f);
        self.add(label, stats);
        result
    }

    /// Generate a formatted report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("Memory Profile Report\n");
        report.push_str("=====================\n\n");
        report.push_str(&format!(
            "{:<40} {:>12} {:>12} {:>10} {:>12}\n",
            "Operation", "Allocated", "Peak", "Allocs", "Bytes/Alloc"
        ));
        report.push_str(&"-".repeat(90));
        report.push('\n');

        for (label, stats) in &self.measurements {
            report.push_str(&format!(
                "{:<40} {:>12} {:>12} {:>10} {:>12.1}\n",
                label,
                format_bytes(stats.allocated),
                format_bytes(stats.peak),
                stats.alloc_count,
                stats.bytes_per_alloc()
            ));
        }

        report.push_str(&"-".repeat(90));
        report.push('\n');

        // Summary
        let total_allocated: usize = self.measurements.iter().map(|(_, s)| s.allocated).sum();
        let total_allocs: usize = self.measurements.iter().map(|(_, s)| s.alloc_count).sum();
        let max_peak = self
            .measurements
            .iter()
            .map(|(_, s)| s.peak)
            .max()
            .unwrap_or(0);

        report.push_str(&format!(
            "{:<40} {:>12} {:>12} {:>10}\n",
            "TOTAL",
            format_bytes(total_allocated),
            format_bytes(max_peak),
            total_allocs
        ));

        report
    }
}

impl std::fmt::Display for MemoryProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.report())
    }
}

// =============================================================================
// Size Estimation (without tracking allocator)
// =============================================================================

/// Estimate the memory size of common types.
pub mod size_of {
    use skia_rs_core::{Color4f, Matrix, Matrix44, Point, Rect};
    use skia_rs_paint::Paint;
    use skia_rs_path::{Path, PathBuilder};

    /// Get size of Point.
    pub fn point() -> usize {
        std::mem::size_of::<Point>()
    }

    /// Get size of Rect.
    pub fn rect() -> usize {
        std::mem::size_of::<Rect>()
    }

    /// Get size of Matrix (3x3).
    pub fn matrix() -> usize {
        std::mem::size_of::<Matrix>()
    }

    /// Get size of Matrix44 (4x4).
    pub fn matrix44() -> usize {
        std::mem::size_of::<Matrix44>()
    }

    /// Get size of Color4f.
    pub fn color4f() -> usize {
        std::mem::size_of::<Color4f>()
    }

    /// Get size of Paint (base struct only).
    pub fn paint() -> usize {
        std::mem::size_of::<Paint>()
    }

    /// Get size of Path (base struct only).
    pub fn path() -> usize {
        std::mem::size_of::<Path>()
    }

    /// Get size of PathBuilder (base struct only).
    pub fn path_builder() -> usize {
        std::mem::size_of::<PathBuilder>()
    }

    /// Print a summary of type sizes.
    pub fn print_summary() {
        println!("Type Size Summary");
        println!("=================");
        println!("Point:       {:>6} bytes", point());
        println!("Rect:        {:>6} bytes", rect());
        println!("Matrix:      {:>6} bytes", matrix());
        println!("Matrix44:    {:>6} bytes", matrix44());
        println!("Color4f:     {:>6} bytes", color4f());
        println!("Paint:       {:>6} bytes", paint());
        println!("Path:        {:>6} bytes", path());
        println!("PathBuilder: {:>6} bytes", path_builder());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats {
            allocated: 1000,
            deallocated: 400,
            peak: 800,
            alloc_count: 10,
            dealloc_count: 4,
        };

        assert_eq!(stats.current(), 600);
        assert_eq!(stats.bytes_per_alloc(), 100.0);
    }

    #[test]
    fn test_size_of() {
        // Just verify these don't panic
        assert!(size_of::point() > 0);
        assert!(size_of::rect() > 0);
        assert!(size_of::matrix() > 0);
        assert!(size_of::paint() > 0);
        assert!(size_of::path() > 0);
    }
}
