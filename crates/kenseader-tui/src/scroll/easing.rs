//! L4 Atomic Layer: Pure easing functions for smooth scrolling animations
//!
//! Provides mathematical easing functions that map input [0, 1] to output [0, 1]
//! with various acceleration curves.

// Re-export EasingType from core
pub use kenseader_core::EasingType;

/// Extension trait for EasingType with calculation methods
pub trait EasingTypeExt {
    /// Apply the easing function to a progress value
    ///
    /// # Arguments
    /// * `t` - Progress value in range [0, 1]
    ///
    /// # Returns
    /// Eased value in range [0, 1]
    fn apply(&self, t: f64) -> f64;
}

impl EasingTypeExt for EasingType {
    #[inline]
    fn apply(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EasingType::None => if t < 1.0 { 0.0 } else { 1.0 },
            EasingType::Linear => t,
            EasingType::Cubic => cubic_ease_out(t),
            EasingType::Quintic => quintic_ease_out(t),
            EasingType::EaseOut => exponential_ease_out(t),
        }
    }
}

/// Cubic ease-out: f(t) = 1 - (1-t)³
#[inline]
fn cubic_ease_out(t: f64) -> f64 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// Quintic ease-out: f(t) = 1 - (1-t)⁵
#[inline]
fn quintic_ease_out(t: f64) -> f64 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv * inv * inv
}

/// Exponential ease-out: f(t) = 1 - 2^(-10t)
#[inline]
fn exponential_ease_out(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_boundaries() {
        for easing in [
            EasingType::None,
            EasingType::Linear,
            EasingType::Cubic,
            EasingType::Quintic,
            EasingType::EaseOut,
        ] {
            // t=0 should give 0 (except None which jumps)
            if easing != EasingType::None {
                assert!((easing.apply(0.0) - 0.0).abs() < 0.001, "{:?} at t=0", easing);
            }
            // t=1 should give 1
            assert!((easing.apply(1.0) - 1.0).abs() < 0.001, "{:?} at t=1", easing);
        }
    }

    #[test]
    fn test_easing_monotonic() {
        for easing in [
            EasingType::Linear,
            EasingType::Cubic,
            EasingType::Quintic,
            EasingType::EaseOut,
        ] {
            let mut prev = 0.0;
            for i in 0..=10 {
                let t = i as f64 / 10.0;
                let v = easing.apply(t);
                assert!(v >= prev, "{:?} not monotonic at t={}", easing, t);
                prev = v;
            }
        }
    }
}
