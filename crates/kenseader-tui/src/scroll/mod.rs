//! Smooth scrolling system for Kenseader TUI
//!
//! This module implements nvim-like smooth scrolling with configurable easing
//! functions and animation parameters.
//!
//! # Architecture (following CLAUDE.md atomic architecture)
//!
//! ## L4 Atomic Layer
//! - `easing` - Pure easing functions (cubic, quintic, exponential)
//! - `timing` - Time calculation utilities (progress, interpolation)
//! - `config` - Configuration types and defaults (re-exported from kenseader-core)
//!
//! ## L3 Molecular Layer
//! - `animation` - Animation controller combining atoms
//!
//! # Usage
//!
//! ```ignore
//! use kenseader_tui::scroll::{ScrollAnimator, ScrollConfig};
//!
//! // Create with default config (smooth scrolling enabled)
//! let mut animator = ScrollAnimator::with_defaults();
//!
//! // Or with custom config
//! let config = ScrollConfig {
//!     smooth_enabled: true,
//!     animation_duration_ms: 200,
//!     ..Default::default()
//! };
//! let mut animator = ScrollAnimator::new(config);
//!
//! // Start a scroll animation
//! animator.scroll_by(10, max_scroll);
//!
//! // In main loop, update each frame and get current position
//! let scroll = animator.update(max_scroll);
//! ```

// L4 Atomic Layer
pub mod config;
pub mod easing;
pub mod timing;

// L3 Molecular Layer
pub mod animation;

// Re-exports for convenient access
pub use animation::ScrollAnimator;
pub use config::{ScrollConfig, ScrollConfigExt};
pub use easing::{EasingType, EasingTypeExt};
