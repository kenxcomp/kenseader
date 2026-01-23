//! L4 Atomic Layer: Time calculation utilities for scroll animations
//!
//! Provides pure functions for calculating animation progress and interpolation.

use std::time::{Duration, Instant};

/// Calculate animation progress (0.0 to 1.0) from start time and duration
///
/// # Arguments
/// * `start` - Animation start time
/// * `duration` - Total animation duration
///
/// # Returns
/// Progress value clamped to [0.0, 1.0]
#[inline]
pub fn progress(start: Instant, duration: Duration) -> f64 {
    if duration.is_zero() {
        return 1.0;
    }
    let elapsed = start.elapsed();
    let ratio = elapsed.as_secs_f64() / duration.as_secs_f64();
    ratio.clamp(0.0, 1.0)
}

/// Check if animation is complete
#[inline]
pub fn is_complete(start: Instant, duration: Duration) -> bool {
    start.elapsed() >= duration
}

/// Linear interpolation between two values
///
/// # Arguments
/// * `from` - Start value
/// * `to` - End value
/// * `t` - Interpolation factor [0.0, 1.0]
///
/// # Returns
/// Interpolated value
#[inline]
pub fn lerp(from: f64, to: f64, t: f64) -> f64 {
    from + (to - from) * t
}

/// Linear interpolation for u16 values (scroll positions)
#[inline]
pub fn lerp_u16(from: u16, to: u16, t: f64) -> u16 {
    lerp(from as f64, to as f64, t).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 100.0, 0.0) - 0.0).abs() < 0.001);
        assert!((lerp(0.0, 100.0, 0.5) - 50.0).abs() < 0.001);
        assert!((lerp(0.0, 100.0, 1.0) - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_lerp_u16() {
        assert_eq!(lerp_u16(0, 100, 0.0), 0);
        assert_eq!(lerp_u16(0, 100, 0.5), 50);
        assert_eq!(lerp_u16(0, 100, 1.0), 100);
    }

    #[test]
    fn test_progress_zero_duration() {
        let start = Instant::now();
        assert!((progress(start, Duration::ZERO) - 1.0).abs() < 0.001);
    }
}
