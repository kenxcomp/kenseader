//! Kitty graphics protocol implementation
//!
//! Implements direct image rendering using Kitty's graphics protocol.
//! See: https://sw.kovidgoyal.net/kitty/graphics-protocol/

use image::{DynamicImage, GenericImageView};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Write};

/// Information about a displayed image
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayedImage {
    /// URL or identifier of the image
    pub url: String,
    /// X position in terminal cells
    pub x: u16,
    /// Y position in terminal cells
    pub y: u16,
    /// Width in terminal cells
    pub width: u16,
    /// Height in terminal cells
    pub height: u16,
}

/// Result of encoding an image with aspect ratio preserved
struct EncodedImage {
    /// PNG data
    data: Vec<u8>,
    /// Actual width in cells (after preserving aspect ratio)
    cols: u16,
    /// Actual height in cells (after preserving aspect ratio)
    rows: u16,
}

/// Kitty graphics protocol renderer with state tracking
pub struct KittyRenderer {
    /// Counter for unique image IDs
    next_id: u32,
    /// Currently displayed images: url -> (kitty_id, display_info)
    displayed: HashMap<String, (u32, DisplayedImage)>,
    /// Cached encoded images: (url, quantized_cols, quantized_rows) -> EncodedImage
    encoded_cache: HashMap<(String, u16, u16), EncodedImage>,
    /// Flag indicating if display state has changed
    dirty: bool,
    /// Cell dimensions (width, height) in pixels
    cell_size: (u32, u32),
}

/// Dimension quantization bucket size for cache stability
const DIMENSION_BUCKET_SIZE: u16 = 4;

impl KittyRenderer {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            displayed: HashMap::new(),
            encoded_cache: HashMap::new(),
            dirty: false,
            // Default cell size for Kitty (can be detected via escape sequence)
            cell_size: (8, 16),
        }
    }

    /// Quantize dimensions to bucket for cache stability during scroll
    fn quantize_dimensions(cols: u16, rows: u16) -> (u16, u16) {
        let q_cols = ((cols + DIMENSION_BUCKET_SIZE - 1) / DIMENSION_BUCKET_SIZE)
            * DIMENSION_BUCKET_SIZE;
        let q_rows = ((rows + DIMENSION_BUCKET_SIZE - 1) / DIMENSION_BUCKET_SIZE)
            * DIMENSION_BUCKET_SIZE;
        (q_cols.max(DIMENSION_BUCKET_SIZE), q_rows.max(DIMENSION_BUCKET_SIZE))
    }

    /// Clear all images from the terminal
    pub fn clear_all(&mut self) -> io::Result<()> {
        if self.displayed.is_empty() {
            return Ok(());
        }

        // Delete all images: a=d (delete), d=A (all images)
        let cmd = "\x1b_Ga=d,d=A\x1b\\";
        let mut stdout = io::stdout();
        stdout.write_all(cmd.as_bytes())?;
        stdout.flush()?;

        self.displayed.clear();
        self.encoded_cache.clear();
        self.dirty = false;

        Ok(())
    }

    /// Clear a specific image by ID
    pub fn clear_image(&mut self, id: u32) -> io::Result<()> {
        // Delete specific image: a=d (delete), d=I (by ID), i=<id>
        let cmd = format!("\x1b_Ga=d,d=I,i={}\x1b\\", id);
        let mut stdout = io::stdout();
        stdout.write_all(cmd.as_bytes())?;
        stdout.flush()
    }

    /// Check if an image needs to be updated at the given position
    pub fn needs_update(&self, url: &str, x: u16, y: u16, width: u16, height: u16) -> bool {
        match self.displayed.get(url) {
            Some((_, info)) => {
                info.x != x || info.y != y || info.width != width || info.height != height
            }
            None => true,
        }
    }

    /// Begin a new render frame - call this before rendering images
    /// Returns true if we need to do a full redraw
    pub fn begin_frame(&mut self) -> bool {
        self.dirty
    }

    /// Mark the renderer as needing a full redraw
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// End render frame - clean up images that were not updated
    pub fn end_frame(&mut self, active_urls: &[String]) -> io::Result<()> {
        // Find images to remove (displayed but not in active list)
        let to_remove: Vec<(String, u32)> = self
            .displayed
            .iter()
            .filter(|(url, _)| !active_urls.contains(url))
            .map(|(url, (id, _))| (url.clone(), *id))
            .collect();

        // Remove stale images
        for (url, id) in to_remove {
            self.clear_image(id)?;
            self.displayed.remove(&url);
        }

        self.dirty = false;
        Ok(())
    }

    /// Display or update an image at the specified position
    /// Only sends data if the image is new or position changed
    /// Preserves aspect ratio within the given bounds
    pub fn display_or_update(
        &mut self,
        url: &str,
        img: &DynamicImage,
        x: u16,
        y: u16,
        max_cols: u16,
        max_rows: u16,
    ) -> io::Result<u32> {
        // Check if we need to update
        if !self.needs_update(url, x, y, max_cols, max_rows) {
            // Image already displayed at correct position
            return Ok(self.displayed.get(url).map(|(id, _)| *id).unwrap_or(0));
        }

        // If image exists but position changed, delete old one first
        if let Some((old_id, _)) = self.displayed.remove(url) {
            self.clear_image(old_id)?;
        }

        // Get or create encoded image with aspect ratio preserved
        // Use quantized dimensions for cache key to improve hit rate during scroll
        let (q_cols, q_rows) = Self::quantize_dimensions(max_cols, max_rows);
        let cache_key = (url.to_string(), q_cols, q_rows);
        let encoded = if let Some(cached) = self.encoded_cache.get(&cache_key) {
            cached
        } else {
            // Use quantized dimensions for encoding to match cache key
            let enc = self.encode_png_preserve_aspect(img, q_cols, q_rows)?;
            self.encoded_cache.insert(cache_key.clone(), enc);
            self.encoded_cache.get(&cache_key).unwrap()
        };

        // Calculate centered position
        let actual_cols = encoded.cols;
        let actual_rows = encoded.rows;
        let x_offset = (max_cols.saturating_sub(actual_cols)) / 2;
        let centered_x = x + x_offset;

        // Display the image
        let id = self.next_id;
        self.next_id += 1;

        self.send_image_at_position(id, &encoded.data, centered_x, y, actual_cols, actual_rows)?;

        // Track the displayed image (use max dimensions for cache key matching)
        self.displayed.insert(
            url.to_string(),
            (
                id,
                DisplayedImage {
                    url: url.to_string(),
                    x,
                    y,
                    width: max_cols,
                    height: max_rows,
                },
            ),
        );

        Ok(id)
    }

    /// Encode image to PNG preserving aspect ratio
    /// Returns the encoded data and actual cell dimensions
    fn encode_png_preserve_aspect(
        &self,
        img: &DynamicImage,
        max_cols: u16,
        max_rows: u16,
    ) -> io::Result<EncodedImage> {
        let (cell_width, cell_height) = self.cell_size;
        let max_pixel_width = max_cols as u32 * cell_width;
        let max_pixel_height = max_rows as u32 * cell_height;

        // Get original image dimensions
        let (img_width, img_height) = img.dimensions();

        // Calculate scale to fit within bounds while preserving aspect ratio
        let scale_w = max_pixel_width as f32 / img_width as f32;
        let scale_h = max_pixel_height as f32 / img_height as f32;
        let scale = scale_w.min(scale_h).min(1.0); // Don't upscale

        let new_pixel_width = ((img_width as f32 * scale) as u32).max(1);
        let new_pixel_height = ((img_height as f32 * scale) as u32).max(1);

        // Calculate actual cell dimensions needed
        let actual_cols = ((new_pixel_width + cell_width - 1) / cell_width) as u16;
        let actual_rows = ((new_pixel_height + cell_height - 1) / cell_height) as u16;

        // Resize image preserving aspect ratio using Cow to avoid unnecessary clones
        // Use Triangle filter (bilinear) for fast realtime encoding - much faster than Lanczos3
        // while still providing good quality for terminal display
        let to_encode: Cow<DynamicImage> = if scale < 1.0 {
            Cow::Owned(img.resize(
                new_pixel_width,
                new_pixel_height,
                image::imageops::FilterType::Triangle,
            ))
        } else {
            // No resize needed - borrow instead of clone
            Cow::Borrowed(img)
        };

        // Encode to PNG
        let mut png_data = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
        to_encode
            .write_with_encoder(encoder)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        Ok(EncodedImage {
            data: png_data,
            cols: actual_cols.min(max_cols),
            rows: actual_rows.min(max_rows),
        })
    }

    /// Send image data at a specific position
    fn send_image_at_position(
        &self,
        id: u32,
        data: &[u8],
        x: u16,
        y: u16,
        cols: u16,
        rows: u16,
    ) -> io::Result<()> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let mut stdout = io::stdout();

        // Save cursor position
        stdout.write_all(b"\x1b[s")?;

        // Move to position (1-indexed)
        let pos_cmd = format!("\x1b[{};{}H", y + 1, x + 1);
        stdout.write_all(pos_cmd.as_bytes())?;

        // Send image in chunks
        let chunk_size = 4096;
        let total_chunks = (data.len() + chunk_size - 1) / chunk_size;

        for (i, chunk) in data.chunks(chunk_size).enumerate() {
            let b64_chunk = STANDARD.encode(chunk);
            let is_first = i == 0;
            let is_last = i == total_chunks - 1;
            let more = if is_last { 0 } else { 1 };

            if is_first {
                // First chunk: include all parameters
                // a=T: transmit and display
                // f=100: PNG format
                // t=d: direct transmission
                // i=<id>: image ID
                // c=<cols>: cell columns (actual size, not max)
                // r=<rows>: cell rows (actual size, not max)
                // q=2: suppress all responses (quiet mode)
                // C=1: do not move cursor
                let cmd = format!(
                    "\x1b_Ga=T,f=100,t=d,i={},c={},r={},q=2,C=1,m={};{}\x1b\\",
                    id, cols, rows, more, b64_chunk
                );
                stdout.write_all(cmd.as_bytes())?;
            } else {
                // Continuation chunks
                let cmd = format!("\x1b_Gm={};{}\x1b\\", more, b64_chunk);
                stdout.write_all(cmd.as_bytes())?;
            }
        }

        // Restore cursor position
        stdout.write_all(b"\x1b[u")?;
        stdout.flush()
    }

    /// Display an image at the current cursor position (legacy API)
    /// Returns the image ID for later reference
    pub fn display_image(
        &mut self,
        img: &DynamicImage,
        cols: u16,
        rows: u16,
    ) -> io::Result<u32> {
        let id = self.next_id;
        self.next_id += 1;

        let encoded = self.encode_png_preserve_aspect(img, cols, rows)?;
        self.send_chunked_image(id, &encoded.data, encoded.cols, encoded.rows)?;

        Ok(id)
    }

    /// Send image data in chunks (Kitty protocol requirement for large images)
    fn send_chunked_image(
        &self,
        id: u32,
        data: &[u8],
        cols: u16,
        rows: u16,
    ) -> io::Result<()> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let mut stdout = io::stdout();
        let chunk_size = 4096; // Max chunk size for Kitty
        let total_chunks = (data.len() + chunk_size - 1) / chunk_size;

        for (i, chunk) in data.chunks(chunk_size).enumerate() {
            let b64_chunk = STANDARD.encode(chunk);
            let is_first = i == 0;
            let is_last = i == total_chunks - 1;
            let more = if is_last { 0 } else { 1 };

            if is_first {
                let cmd = format!(
                    "\x1b_Ga=T,f=100,t=d,i={},c={},r={},q=2,m={};{}\x1b\\",
                    id, cols, rows, more, b64_chunk
                );
                stdout.write_all(cmd.as_bytes())?;
            } else {
                let cmd = format!("\x1b_Gm={};{}\x1b\\", more, b64_chunk);
                stdout.write_all(cmd.as_bytes())?;
            }
        }

        stdout.flush()
    }

    /// Move cursor to position and display image (legacy API)
    pub fn display_at_position(
        &mut self,
        img: &DynamicImage,
        x: u16,
        y: u16,
        cols: u16,
        rows: u16,
    ) -> io::Result<u32> {
        let mut stdout = io::stdout();

        // Save cursor position
        stdout.write_all(b"\x1b[s")?;

        // Move to position (1-indexed)
        let cmd = format!("\x1b[{};{}H", y + 1, x + 1);
        stdout.write_all(cmd.as_bytes())?;

        // Display image with aspect ratio preserved
        let id = self.display_image(img, cols, rows)?;

        // Restore cursor position
        stdout.write_all(b"\x1b[u")?;
        stdout.flush()?;

        Ok(id)
    }
}

impl Default for KittyRenderer {
    fn default() -> Self {
        Self::new()
    }
}
