//! Keystroke and pointer timings for the perf report.

/// Keystroke-to-present timings (v0.7 perf report): microseconds from an
/// input event to the end of its refresh call, for key presses.
#[derive(Clone, Debug, Default)]
pub struct Perf {
    pub samples: Vec<u32>,
    /// Pointer moves: event to the end of the (never waited) submit, µs
    /// (v1.0.3).
    pub pointer: Vec<u32>,
}

impl Perf {
    /// (count, p50, p95, max) in milliseconds.
    #[must_use]
    pub fn summary(&self) -> Option<(usize, f64, f64, f64)> {
        Self::stats(&self.samples)
    }

    /// (count, p50, p95, max) of the pointer samples, in milliseconds.
    #[must_use]
    pub fn pointer_summary(&self) -> Option<(usize, f64, f64, f64)> {
        Self::stats(&self.pointer)
    }

    fn stats(samples: &[u32]) -> Option<(usize, f64, f64, f64)> {
        if samples.is_empty() {
            return None;
        }
        let mut v = samples.to_vec();
        v.sort_unstable();
        let at = |q: f64| f64::from(v[((v.len() - 1) as f64 * q).round() as usize]) / 1000.0;
        Some((v.len(), at(0.5), at(0.95), at(1.0)))
    }
}
