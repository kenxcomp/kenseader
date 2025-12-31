use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use image::DynamicImage;
use anyhow::Result;

/// Event handler for terminal events
pub struct EventHandler {
    tick_rate: Duration,
}

/// Result of an async image load operation
pub enum ImageLoadResult {
    /// Image loaded successfully
    Success {
        url: String,
        image: DynamicImage,
        bytes: Vec<u8>,
    },
    /// Image failed to load
    Failure {
        url: String,
        error: String,
    },
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next event
    pub fn next(&self) -> Result<Option<AppEvent>> {
        if event::poll(self.tick_rate)? {
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
