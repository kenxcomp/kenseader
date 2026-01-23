use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use image::DynamicImage;

/// Event handler for terminal events
pub struct EventHandler {
    /// Default tick rate (used when idle)
    tick_rate: Duration,
    /// Fast tick rate (used during animations)
    animation_tick_rate: Duration,
}

/// Result of an async image load operation
pub enum ImageLoadResult {
    /// Image loaded successfully
    Success {
        url: String,
        image: DynamicImage,
        bytes: Vec<u8>,
        /// Path to the cached image file on disk
        cache_path: Option<PathBuf>,
    },
    /// Image failed to load
    Failure {
        url: String,
        error: String,
    },
}

/// Result of an async refresh operation
pub enum RefreshResult {
    /// Refresh completed successfully
    Success {
        new_count: u32,
    },
    /// Refresh failed
    Failure {
        error: String,
    },
}

impl EventHandler {
    /// Create a new event handler with default and animation tick rates
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
            animation_tick_rate: Duration::from_millis(16), // ~60 FPS for animations
        }
    }

    /// Create with custom animation FPS
    pub fn with_animation_fps(tick_rate_ms: u64, animation_fps: u32) -> Self {
        let animation_tick_ms = if animation_fps == 0 { 16 } else { 1000 / animation_fps as u64 };
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
            animation_tick_rate: Duration::from_millis(animation_tick_ms),
        }
    }

    /// Poll for the next event with default tick rate
    pub fn next(&self) -> Result<Option<AppEvent>> {
        self.next_with_rate(self.tick_rate)
    }

    /// Poll for the next event with animation tick rate (higher FPS)
    pub fn next_animation(&self) -> Result<Option<AppEvent>> {
        self.next_with_rate(self.animation_tick_rate)
    }

    /// Poll for the next event with custom tick rate
    fn next_with_rate(&self, tick_rate: Duration) -> Result<Option<AppEvent>> {
        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key) => {
                    // Only handle key press events, ignore release events
                    // (crossterm 0.27+ sends release events on some systems)
                    if key.kind == KeyEventKind::Press {
                        Ok(Some(AppEvent::Key(key)))
                    } else {
                        Ok(None)
                    }
                }
                Event::Resize(w, h) => Ok(Some(AppEvent::Resize(w, h))),
                _ => Ok(None),
            }
        } else {
            Ok(Some(AppEvent::Tick))
        }
    }
}

/// Application events
#[derive(Debug)]
pub enum AppEvent {
    /// A key was pressed
    Key(KeyEvent),
    /// Terminal was resized
    Resize(u16, u16),
    /// Tick event for periodic updates
    Tick,
}
