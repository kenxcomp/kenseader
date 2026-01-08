//! Image renderer module with multiple backend support
//!
//! Priority: Kitty > iTerm2 > Sixel > Üeberzug++ > Halfblocks

mod kitty;
mod ueberzug;

use std::path::Path;

pub use kitty::KittyRenderer;
pub use ueberzug::UeberzugInstance;

/// Render backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    /// Kitty Graphics Protocol
    Kitty,
    /// iTerm2 Inline Images Protocol
    ITerm2,
    /// Sixel Graphics
    Sixel,
    /// Üeberzug++ (X11/Wayland overlay)
    Ueberzug,
    /// Unicode Halfblocks (universal fallback)
    Halfblocks,
}

impl std::fmt::Display for RenderBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderBackend::Kitty => write!(f, "Kitty"),
            RenderBackend::ITerm2 => write!(f, "iTerm2"),
            RenderBackend::Sixel => write!(f, "Sixel"),
            RenderBackend::Ueberzug => write!(f, "Üeberzug++"),
            RenderBackend::Halfblocks => write!(f, "Halfblocks"),
        }
    }
}

/// Image renderer with automatic backend detection
pub struct ImageRenderer {
    backend: RenderBackend,
    ueberzug: Option<UeberzugInstance>,
    kitty: Option<KittyRenderer>,
    /// Track which image identifiers are currently displayed
    displayed_images: std::collections::HashSet<String>,
}

impl ImageRenderer {
    /// Detect the best available rendering backend and create a new renderer
    pub fn new() -> Self {
        let backend = Self::detect_backend();

        // Initialize Ueberzug if selected
        let ueberzug = if backend == RenderBackend::Ueberzug {
            match UeberzugInstance::new() {
                Ok(instance) => Some(instance),
                Err(e) => {
                    tracing::warn!("Failed to start ueberzugpp: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Initialize Kitty renderer if Kitty backend
        let kitty = if backend == RenderBackend::Kitty {
            Some(KittyRenderer::new())
        } else {
            None
        };

        // If ueberzug was selected but failed to start, fall back to halfblocks
        let backend = if backend == RenderBackend::Ueberzug && ueberzug.is_none() {
            RenderBackend::Halfblocks
        } else {
            backend
        };

        tracing::info!("Image renderer using backend: {}", backend);

        Self {
            backend,
            ueberzug,
            kitty,
            displayed_images: std::collections::HashSet::new(),
        }
    }

    /// Get the current backend
    pub fn backend(&self) -> RenderBackend {
        self.backend
    }

    /// Detect the best available rendering backend
    fn detect_backend() -> RenderBackend {
        // 1. Check for Kitty terminal
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            tracing::debug!("Detected Kitty terminal");
            return RenderBackend::Kitty;
        }

        // 2. Check for iTerm2
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            if term_program.contains("iTerm") {
                tracing::debug!("Detected iTerm2 terminal");
                return RenderBackend::ITerm2;
            }
        }

        // 3. Check for WezTerm (supports iTerm2 protocol)
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            if term_program.contains("WezTerm") {
                tracing::debug!("Detected WezTerm terminal (using iTerm2 protocol)");
                return RenderBackend::ITerm2;
            }
        }

        // 4. Check for Sixel support via TERM
        if let Ok(term) = std::env::var("TERM") {
            // Common Sixel-capable terminals
            let sixel_terms = ["mlterm", "yaft", "foot", "contour"];
            if sixel_terms.iter().any(|t| term.contains(t)) {
                tracing::debug!("Detected Sixel-capable terminal: {}", term);
                return RenderBackend::Sixel;
            }
        }

        // 5. Check for X11/Wayland + ueberzugpp
        if Self::is_graphical_environment() && Self::ueberzug_available() {
            tracing::debug!("Detected graphical environment with ueberzugpp");
            return RenderBackend::Ueberzug;
        }

        // 6. Fallback to halfblocks
        tracing::debug!("Using halfblock fallback");
        RenderBackend::Halfblocks
    }

    /// Check if running in a graphical environment (X11 or Wayland)
    fn is_graphical_environment() -> bool {
        std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    /// Check if ueberzugpp is available in PATH
    fn ueberzug_available() -> bool {
        std::process::Command::new("ueberzugpp")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Render an image at the specified position
    ///
    /// For Üeberzug backend, this uses subprocess communication.
    /// For native protocols (Kitty/iTerm2/Sixel), returns false to indicate
    /// that the caller should handle rendering.
    /// For Halfblocks, returns false to indicate inline rendering is needed.
    pub fn render(
        &mut self,
        identifier: &str,
        path: &Path,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> bool {
        match self.backend {
            RenderBackend::Ueberzug => {
                if let Some(ref mut ueberzug) = self.ueberzug {
                    if let Err(e) = ueberzug.add(identifier, path, x, y, width, height) {
                        tracing::error!("Failed to render image via ueberzugpp: {}", e);
                        return false;
                    }
                    self.displayed_images.insert(identifier.to_string());
                    true
                } else {
                    false
                }
            }
            // Native protocols and halfblocks are handled by the TUI layer
            RenderBackend::Kitty | RenderBackend::ITerm2 | RenderBackend::Sixel => false,
            RenderBackend::Halfblocks => false,
        }
    }

    /// Clear a specific image by identifier
    pub fn clear(&mut self, identifier: &str) {
        if let Some(ref mut ueberzug) = self.ueberzug {
            if self.displayed_images.remove(identifier) {
                if let Err(e) = ueberzug.remove(identifier) {
                    tracing::error!("Failed to clear image: {}", e);
                }
            }
        }
    }

    /// Clear all displayed images
    pub fn clear_all(&mut self) {
        // Clear Ueberzug images
        if let Some(ref mut ueberzug) = self.ueberzug {
            for identifier in self.displayed_images.drain() {
                if let Err(e) = ueberzug.remove(&identifier) {
                    tracing::error!("Failed to clear image {}: {}", identifier, e);
                }
            }
        }

        // Clear Kitty images
        if let Some(ref mut kitty) = self.kitty {
            if let Err(e) = kitty.clear_all() {
                tracing::error!("Failed to clear Kitty images: {}", e);
            }
        }
    }

    /// Get mutable reference to Kitty renderer (if available)
    pub fn kitty_renderer(&mut self) -> Option<&mut KittyRenderer> {
        self.kitty.as_mut()
    }

    /// Check if this backend requires external rendering (not inline in TUI)
    pub fn is_external_renderer(&self) -> bool {
        matches!(self.backend, RenderBackend::Ueberzug)
    }

    /// Check if this backend uses native terminal protocols
    pub fn uses_native_protocol(&self) -> bool {
        matches!(
            self.backend,
            RenderBackend::Kitty | RenderBackend::ITerm2 | RenderBackend::Sixel
        )
    }
}

impl Default for ImageRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ImageRenderer {
    fn drop(&mut self) {
        // Clear all images when renderer is dropped
        self.clear_all();
    }
}
