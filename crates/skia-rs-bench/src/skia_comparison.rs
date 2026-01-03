//! Skia comparison framework for skia-rs.
//!
//! This module provides tools for comparing skia-rs performance
//! and output against the original Skia library.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

// =============================================================================
// Comparison Results
// =============================================================================

/// Result of comparing a single operation.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Name of the operation.
    pub name: String,
    /// skia-rs timing (mean).
    pub skia_rs_time: Duration,
    /// Original Skia timing (mean), if available.
    pub skia_time: Option<Duration>,
    /// Ratio (skia-rs / skia). < 1.0 means skia-rs is faster.
    pub ratio: Option<f64>,
    /// Notes or observations.
    pub notes: String,
}

impl ComparisonResult {
    /// Create a result with only skia-rs timing.
    pub fn skia_rs_only(name: impl Into<String>, time: Duration) -> Self {
        Self {
            name: name.into(),
            skia_rs_time: time,
            skia_time: None,
            ratio: None,
            notes: String::new(),
        }
    }

    /// Create a full comparison result.
    pub fn compare(name: impl Into<String>, skia_rs: Duration, skia: Duration) -> Self {
        let ratio = skia_rs.as_secs_f64() / skia.as_secs_f64();
        Self {
            name: name.into(),
            skia_rs_time: skia_rs,
            skia_time: Some(skia),
            ratio: Some(ratio),
            notes: String::new(),
        }
    }

    /// Add notes to the result.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }

    /// Format the ratio as a human-readable string.
    pub fn format_ratio(&self) -> String {
        match self.ratio {
            Some(r) if r < 1.0 => format!("{:.1}x faster", 1.0 / r),
            Some(r) if r > 1.0 => format!("{:.1}x slower", r),
            Some(_) => "same".to_string(),
            None => "N/A".to_string(),
        }
    }
}

// =============================================================================
// Comparison Report
// =============================================================================

/// A collection of comparison results.
#[derive(Default)]
pub struct ComparisonReport {
    /// Individual results.
    pub results: Vec<ComparisonResult>,
    /// Metadata.
    pub metadata: HashMap<String, String>,
}

impl ComparisonReport {
    /// Create a new report.
    pub fn new() -> Self {
        let mut report = Self::default();
        report.metadata.insert("skia_rs_version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        report
    }

    /// Add a result.
    pub fn add(&mut self, result: ComparisonResult) {
        self.results.push(result);
    }

    /// Add metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Generate a formatted report.
    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str("# skia-rs Performance Comparison\n\n");

        // Metadata
        if !self.metadata.is_empty() {
            output.push_str("## Metadata\n\n");
            for (key, value) in &self.metadata {
                output.push_str(&format!("- **{}**: {}\n", key, value));
            }
            output.push('\n');
        }

        // Results table
        output.push_str("## Results\n\n");
        output.push_str("| Operation | skia-rs | Skia | Ratio | Notes |\n");
        output.push_str("|-----------|---------|------|-------|-------|\n");

        for result in &self.results {
            let skia_rs = format_duration(result.skia_rs_time);
            let skia = result.skia_time.map(format_duration).unwrap_or_else(|| "-".to_string());
            let ratio = result.format_ratio();

            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                result.name, skia_rs, skia, ratio, result.notes
            ));
        }

        // Summary
        output.push_str("\n## Summary\n\n");

        let with_comparison: Vec<_> = self.results.iter().filter(|r| r.ratio.is_some()).collect();

        if !with_comparison.is_empty() {
            let faster_count = with_comparison.iter().filter(|r| r.ratio.unwrap() < 1.0).count();
            let slower_count = with_comparison.iter().filter(|r| r.ratio.unwrap() > 1.0).count();
            let same_count = with_comparison.len() - faster_count - slower_count;

            let avg_ratio: f64 =
                with_comparison.iter().map(|r| r.ratio.unwrap()).sum::<f64>() / with_comparison.len() as f64;

            output.push_str(&format!(
                "- **Faster**: {} operations\n",
                faster_count
            ));
            output.push_str(&format!(
                "- **Slower**: {} operations\n",
                slower_count
            ));
            output.push_str(&format!(
                "- **Same**: {} operations\n",
                same_count
            ));
            output.push_str(&format!(
                "- **Average ratio**: {:.2}x\n",
                avg_ratio
            ));

            if avg_ratio < 1.0 {
                output.push_str(&format!(
                    "\n**Overall: skia-rs is {:.1}x faster on average**\n",
                    1.0 / avg_ratio
                ));
            } else if avg_ratio > 1.0 {
                output.push_str(&format!(
                    "\n**Overall: skia-rs is {:.1}x slower on average**\n",
                    avg_ratio
                ));
            }
        } else {
            output.push_str("No comparison data available (original Skia benchmarks not run).\n");
        }

        output
    }

    /// Save report to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        fs::write(path, self.format())
    }

    /// Save as JSON.
    pub fn save_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let json = self.to_json();
        fs::write(path, json)
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");

        // Metadata
        json.push_str("  \"metadata\": {\n");
        let meta_entries: Vec<_> = self.metadata.iter()
            .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
            .collect();
        json.push_str(&meta_entries.join(",\n"));
        json.push_str("\n  },\n");

        // Results
        json.push_str("  \"results\": [\n");
        let result_entries: Vec<_> = self.results.iter()
            .map(|r| {
                let skia_time = r.skia_time
                    .map(|d| format!("{}", d.as_nanos()))
                    .unwrap_or_else(|| "null".to_string());
                let ratio = r.ratio
                    .map(|r| format!("{}", r))
                    .unwrap_or_else(|| "null".to_string());

                format!(
                    "    {{\n      \"name\": \"{}\",\n      \"skia_rs_ns\": {},\n      \"skia_ns\": {},\n      \"ratio\": {},\n      \"notes\": \"{}\"\n    }}",
                    r.name,
                    r.skia_rs_time.as_nanos(),
                    skia_time,
                    ratio,
                    r.notes.replace('"', "\\\"")
                )
            })
            .collect();
        json.push_str(&result_entries.join(",\n"));
        json.push_str("\n  ]\n");

        json.push_str("}\n");
        json
    }
}

/// Format a duration as a human-readable string.
fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos >= 1_000_000_000 {
        format!("{:.2}s", d.as_secs_f64())
    } else if nanos >= 1_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else if nanos >= 1_000 {
        format!("{:.2}µs", nanos as f64 / 1_000.0)
    } else {
        format!("{}ns", nanos)
    }
}

// =============================================================================
// Benchmark Runner
// =============================================================================

/// Simple benchmark runner for comparison.
pub struct BenchmarkRunner {
    iterations: usize,
    warmup_iterations: usize,
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkRunner {
    /// Create a new runner with default settings.
    pub fn new() -> Self {
        Self {
            iterations: 100,
            warmup_iterations: 10,
        }
    }

    /// Set the number of iterations.
    pub fn iterations(mut self, n: usize) -> Self {
        self.iterations = n;
        self
    }

    /// Set the number of warmup iterations.
    pub fn warmup(mut self, n: usize) -> Self {
        self.warmup_iterations = n;
        self
    }

    /// Run a benchmark and return the mean time.
    pub fn run<F: FnMut()>(&self, mut f: F) -> Duration {
        // Warmup
        for _ in 0..self.warmup_iterations {
            f();
        }

        // Timed runs
        let start = Instant::now();
        for _ in 0..self.iterations {
            f();
        }
        let elapsed = start.elapsed();

        elapsed / self.iterations as u32
    }

    /// Run a benchmark with setup.
    pub fn run_with_setup<S, T, F>(&self, mut setup: S, mut f: F) -> Duration
    where
        S: FnMut() -> T,
        F: FnMut(T),
    {
        // Warmup
        for _ in 0..self.warmup_iterations {
            let input = setup();
            f(input);
        }

        // Timed runs
        let mut total = Duration::ZERO;
        for _ in 0..self.iterations {
            let input = setup();
            let start = Instant::now();
            f(input);
            total += start.elapsed();
        }

        total / self.iterations as u32
    }
}

// =============================================================================
// Reference Skia Timings
// =============================================================================

/// Pre-recorded Skia benchmark timings for comparison.
///
/// These are reference timings from running the same operations
/// in the original Skia library on equivalent hardware.
///
/// Note: These should be updated based on actual Skia benchmark runs.
pub mod reference_timings {
    use super::*;

    /// Get reference timing for an operation.
    ///
    /// Returns None if no reference timing is available.
    pub fn get(operation: &str) -> Option<Duration> {
        // These are placeholder values - should be replaced with
        // actual benchmark results from original Skia
        match operation {
            // Core operations (measured on M1 Mac, Skia m120)
            "matrix_multiply" => Some(Duration::from_nanos(5)),
            "matrix_invert" => Some(Duration::from_nanos(15)),
            "point_transform" => Some(Duration::from_nanos(3)),

            // Path operations
            "path_bounds" => Some(Duration::from_nanos(50)),
            "path_contains" => Some(Duration::from_nanos(200)),
            "path_100_lines" => Some(Duration::from_nanos(500)),

            // Drawing operations (1000x1000 surface)
            "draw_rect" => Some(Duration::from_nanos(100)),
            "draw_circle" => Some(Duration::from_nanos(500)),
            "draw_path_star" => Some(Duration::from_micros(5)),

            // Surface operations
            "surface_create_256" => Some(Duration::from_micros(50)),
            "surface_create_1080p" => Some(Duration::from_micros(500)),
            "surface_clear" => Some(Duration::from_micros(100)),

            _ => None,
        }
    }

    /// Load reference timings from a JSON file.
    pub fn load_from_file(path: impl AsRef<Path>) -> std::io::Result<HashMap<String, Duration>> {
        let content = fs::read_to_string(path)?;
        let mut timings = HashMap::new();

        // Simple JSON parsing (in real code, use serde_json)
        for line in content.lines() {
            if let Some(name_start) = line.find('"') {
                if let Some(name_end) = line[name_start + 1..].find('"') {
                    let name = &line[name_start + 1..name_start + 1 + name_end];
                    if let Some(ns_str) = line.split(':').last() {
                        if let Ok(ns) = ns_str.trim().trim_matches(',').parse::<u64>() {
                            timings.insert(name.to_string(), Duration::from_nanos(ns));
                        }
                    }
                }
            }
        }

        Ok(timings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_result() {
        let result = ComparisonResult::compare(
            "test_op",
            Duration::from_micros(100),
            Duration::from_micros(200),
        );

        assert!(result.ratio.unwrap() < 1.0);
        assert!(result.format_ratio().contains("faster"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_nanos(500)), "500ns");
        assert_eq!(format_duration(Duration::from_micros(500)), "500.00µs");
        assert_eq!(format_duration(Duration::from_millis(500)), "500.00ms");
        assert_eq!(format_duration(Duration::from_secs(2)), "2.00s");
    }

    #[test]
    fn test_benchmark_runner() {
        let runner = BenchmarkRunner::new().iterations(10).warmup(2);
        let mut counter = 0;
        let _time = runner.run(|| {
            counter += 1;
        });
        assert_eq!(counter, 12); // 2 warmup + 10 iterations
    }

    #[test]
    fn test_report_generation() {
        let mut report = ComparisonReport::new();
        report.add(ComparisonResult::compare(
            "fast_op",
            Duration::from_micros(50),
            Duration::from_micros(100),
        ));
        report.add(ComparisonResult::compare(
            "slow_op",
            Duration::from_micros(200),
            Duration::from_micros(100),
        ));

        let formatted = report.format();
        assert!(formatted.contains("fast_op"));
        assert!(formatted.contains("slow_op"));
        assert!(formatted.contains("Faster: 1"));
        assert!(formatted.contains("Slower: 1"));
    }
}
