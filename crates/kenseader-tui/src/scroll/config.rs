//! L4 Atomic Layer: Configuration types for smooth scrolling
//!
//! Re-exports configuration from kenseader-core and provides additional utilities.

use std::time::Duration;

// Re-export config types from core
pub use kenseader_core::{EasingType, ScrollConfig};

/// Extension trait for ScrollConfig with utility methods
pub trait ScrollConfigExt {
    /// Get animation duration as Duration
    fn animation_duration(&self) -> Duration;

    /// Get tick duration for animation FPS
    fn animation_tick_duration(&self) -> Duration;

    /// Check if smooth scrolling is effectively enabled
    fn is_smooth(&self) -> bool;
}

impl ScrollConfigExt for ScrollConfig {
    #[inline]
    fn animation_duration(&self) -> Duration {
        Duration::from_millis(self.animation_duration_ms)
    }

    #[inline]
    fn animation_tick_duration(&self) -> Duration {
        if self.animation_fps == 0 {
            Duration::from_millis(16) // ~60fps fallback
        } else {
            Duration::from_millis(1000 / self.animation_fps as u64)
        }
    }

    #[inline]
    fn is_smooth(&self) -> bool {
        self.smooth_enabled && self.animation_duration_ms > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScrollConfig::default();
        assert!(config.smooth_enabled);
        assert_eq!(config.animation_duration_ms, 150);
        assert_eq!(config.easing, EasingType::Cubic);
        assert_eq!(config.scroll_lines, 1);
        assert_eq!(config.animation_fps, 60);
    }

    #[test]
    fn test_animation_duration() {
        let config = ScrollConfig {
            animation_duration_ms: 200,
            ..Default::default()
        };
        assert_eq!(config.animation_duration(), Duration::from_millis(200));
    }

    #[test]
    fn test_is_smooth() {
        let mut config = ScrollConfig::default();
        assert!(config.is_smooth());

        config.smooth_enabled = false;
        assert!(!config.is_smooth());

        config.smooth_enabled = true;
        config.animation_duration_ms = 0;
        assert!(!config.is_smooth());
    }
}
