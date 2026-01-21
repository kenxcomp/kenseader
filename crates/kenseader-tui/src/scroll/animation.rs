//! L3 Molecular Layer: Scroll animation controller
//!
//! Combines easing functions and timing utilities to manage scroll animations.
//! This is the main interface for the smooth scrolling system.

use std::time::{Duration, Instant};

use super::config::{ScrollConfig, ScrollConfigExt};
use super::easing::{EasingType, EasingTypeExt};
use super::timing::{is_complete, lerp_u16, progress};

/// Active scroll animation state
#[derive(Debug, Clone)]
struct ActiveAnimation {
    /// Animation start time
    start: Instant,
    /// Starting scroll position
    from: u16,
    /// Target scroll position
    to: u16,
    /// Animation duration
    duration: Duration,
    /// Easing function
    easing: EasingType,
}

/// Scroll animation controller
///
/// Manages smooth scrolling animations for article detail view.
/// Call `start_scroll()` to begin an animation, then `update()` each frame
/// to get the current interpolated scroll position.
#[derive(Debug, Clone)]
pub struct ScrollAnimator {
    /// Current active animation (if any)
    animation: Option<ActiveAnimation>,
    /// Configuration
    config: ScrollConfig,
    /// Current scroll position (always up-to-date)
    current_scroll: u16,
    /// Pending scroll delta for batching multiple scroll events
    pending_delta: i32,
}

impl Default for ScrollAnimator {
    fn default() -> Self {
        Self {
            animation: None,
            config: ScrollConfig::default(),
            current_scroll: 0,
            pending_delta: 0,
        }
    }
}

impl ScrollAnimator {
    /// Create a new scroll animator with configuration
    pub fn new(config: ScrollConfig) -> Self {
        Self {
            animation: None,
            config,
            current_scroll: 0,
            pending_delta: 0,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ScrollConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &ScrollConfig {
        &self.config
    }

    /// Check if an animation is currently active
    #[inline]
    pub fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    /// Check if there's pending work (animation or pending delta)
    /// Use this to determine if we need high frame rate
    #[inline]
    pub fn needs_update(&self) -> bool {
        self.animation.is_some() || self.pending_delta != 0
    }

    /// Get the target scroll position (final position after animation)
    pub fn target_scroll(&self) -> u16 {
        self.animation
            .as_ref()
            .map(|a| a.to)
            .unwrap_or(self.current_scroll)
    }

    /// Get the current interpolated scroll position
    #[inline]
    pub fn current_scroll(&self) -> u16 {
        self.current_scroll
    }

    /// Set scroll position immediately (no animation)
    pub fn set_scroll(&mut self, scroll: u16) {
        self.animation = None;
        self.current_scroll = scroll;
        self.pending_delta = 0;
    }

    /// Start a scroll animation to a target position
    ///
    /// If smooth scrolling is disabled, jumps immediately to target.
    /// If an animation is in progress, the new target is set relative to
    /// the current animation's target (for smooth chaining).
    pub fn scroll_to(&mut self, target: u16, max_scroll: u16) {
        let target = target.min(max_scroll);

        if !self.config.is_smooth() {
            // Instant jump when smooth scrolling is disabled
            self.current_scroll = target;
            self.animation = None;
            return;
        }

        // Start from current visible position
        let from = self.current_scroll;

        // Skip animation if already at target
        if from == target {
            self.animation = None;
            return;
        }

        self.animation = Some(ActiveAnimation {
            start: Instant::now(),
            from,
            to: target,
            duration: self.config.animation_duration(),
            easing: self.config.easing,
        });
    }

    /// Scroll by a delta amount (positive = down, negative = up)
    ///
    /// Multiple scroll events within the same animation frame are batched
    /// together for smoother handling of rapid key presses.
    pub fn scroll_by(&mut self, delta: i32, max_scroll: u16) {
        if !self.config.is_smooth() {
            // Instant scroll
            let new_scroll = (self.current_scroll as i32 + delta)
                .clamp(0, max_scroll as i32) as u16;
            self.current_scroll = new_scroll;
            self.animation = None;
            return;
        }

        // Accumulate delta for batching
        self.pending_delta += delta;
    }

    /// Scroll down by configured line count
    pub fn scroll_down(&mut self, max_scroll: u16) {
        let lines = if self.config.is_smooth() {
            1 // Smooth scroll moves 1 line at a time for fine control
        } else {
            self.config.scroll_lines as i32
        };
        self.scroll_by(lines, max_scroll);
    }

    /// Scroll up by configured line count
    pub fn scroll_up(&mut self, max_scroll: u16) {
        let lines = if self.config.is_smooth() {
            1
        } else {
            self.config.scroll_lines as i32
        };
        self.scroll_by(-lines, max_scroll);
    }

    /// Scroll down by half page
    pub fn scroll_half_page_down(&mut self, viewport_height: u16, max_scroll: u16) {
        let half_page = (viewport_height / 2).max(1) as i32;
        self.scroll_by(half_page, max_scroll);
    }

    /// Scroll up by half page
    pub fn scroll_half_page_up(&mut self, viewport_height: u16, max_scroll: u16) {
        let half_page = (viewport_height / 2).max(1) as i32;
        self.scroll_by(-half_page, max_scroll);
    }

    /// Scroll down by full page
    pub fn scroll_full_page_down(&mut self, viewport_height: u16, max_scroll: u16) {
        self.scroll_by(viewport_height as i32, max_scroll);
    }

    /// Scroll up by full page
    pub fn scroll_full_page_up(&mut self, viewport_height: u16, max_scroll: u16) {
        self.scroll_by(-(viewport_height as i32), max_scroll);
    }

    /// Update animation state and return current scroll position
    ///
    /// Call this every frame to advance the animation.
    /// Returns the current interpolated scroll position.
    pub fn update(&mut self, max_scroll: u16) -> u16 {
        // Process any pending scroll delta
        if self.pending_delta != 0 {
            let target = self.target_scroll();
            let new_target = (target as i32 + self.pending_delta)
                .clamp(0, max_scroll as i32) as u16;
            self.pending_delta = 0;

            // Start or update animation to new target
            if new_target != self.current_scroll {
                self.animation = Some(ActiveAnimation {
                    start: Instant::now(),
                    from: self.current_scroll,
                    to: new_target,
                    duration: self.config.animation_duration(),
                    easing: self.config.easing,
                });
            }
        }

        // Update active animation
        if let Some(ref anim) = self.animation {
            if is_complete(anim.start, anim.duration) {
                // Animation complete
                self.current_scroll = anim.to.min(max_scroll);
                self.animation = None;
            } else {
                // Calculate interpolated position
                let t = progress(anim.start, anim.duration);
                let eased_t = anim.easing.apply(t);
                self.current_scroll = lerp_u16(anim.from, anim.to, eased_t).min(max_scroll);
            }
        }

        self.current_scroll
    }

    /// Cancel any active animation and stop at current position
    pub fn cancel(&mut self) {
        self.animation = None;
        self.pending_delta = 0;
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.animation = None;
        self.current_scroll = 0;
        self.pending_delta = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instant_scroll_when_disabled() {
        let config = ScrollConfig {
            smooth_enabled: false,
            ..Default::default()
        };
        let mut animator = ScrollAnimator::new(config);

        animator.scroll_to(100, 200);
        assert_eq!(animator.current_scroll(), 100);
        assert!(!animator.is_animating());
    }

    #[test]
    fn test_animation_starts() {
        let config = ScrollConfig {
            smooth_enabled: true,
            animation_duration_ms: 100,
            ..Default::default()
        };
        let mut animator = ScrollAnimator::new(config);

        animator.scroll_to(100, 200);
        assert!(animator.is_animating());
        assert_eq!(animator.target_scroll(), 100);
    }

    #[test]
    fn test_scroll_by_batching() {
        let config = ScrollConfig {
            smooth_enabled: true,
            animation_duration_ms: 100,
            ..Default::default()
        };
        let mut animator = ScrollAnimator::new(config);

        // Multiple scroll_by calls should batch
        animator.scroll_by(10, 200);
        animator.scroll_by(10, 200);
        animator.scroll_by(10, 200);

        // Update should process all pending deltas
        animator.update(200);
        assert_eq!(animator.target_scroll(), 30);
    }

    #[test]
    fn test_scroll_clamp_max() {
        let mut animator = ScrollAnimator::with_defaults();
        animator.set_scroll(50);
        animator.scroll_to(300, 100);
        animator.update(100);
        // Target should be clamped to max_scroll
        assert!(animator.target_scroll() <= 100);
    }
}
