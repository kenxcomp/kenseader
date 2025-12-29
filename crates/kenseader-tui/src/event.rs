use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
use anyhow::Result;

/// Event handler for terminal events
pub struct EventHandler {
    tick_rate: Duration,
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
                Event::Key(key) => Ok(Some(AppEvent::Key(key))),
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
