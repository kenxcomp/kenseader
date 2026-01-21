use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use regex::Regex;

use image::{DynamicImage, RgbaImage};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;

const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Get the global image picker instance with automatic protocol detection
pub fn get_image_picker() -> &'static Picker {
    static PICKER: OnceLock<Picker> = OnceLock::new();
    PICKER.get_or_init(|| {
        // Try to query terminal capabilities for best protocol
        // Falls back to halfblocks if query fails
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::from_fontsize((8, 16)))
    })
}

/// Check if the terminal supports graphics protocols (not just halfblocks)
pub fn supports_graphics_protocol() -> bool {
    static SUPPORTS_GRAPHICS: OnceLock<bool> = OnceLock::new();
    *SUPPORTS_GRAPHICS.get_or_init(|| {
        let picker = get_image_picker();
        !matches!(picker.protocol_type(), ProtocolType::Halfblocks)
    })
}

/// A cached image with its data and render state
pub struct CachedImageData {
    /// The raw image data (wrapped in Arc to avoid expensive deep clones)
    pub image: Arc<DynamicImage>,
    /// The stateful protocol for StatefulImage rendering
    pub protocol: Option<StatefulProtocol>,
    /// Disk cache path for external viewer fallback
    pub cache_path: Option<PathBuf>,
}

impl CachedImageData {
    pub fn new(image: DynamicImage, cache_path: Option<PathBuf>) -> Self {
        Self {
            image: Arc::new(image),
            protocol: None,
            cache_path,
        }
    }

    /// Create from an already-Arc'd image (avoids double-wrapping)
    pub fn new_arc(image: Arc<DynamicImage>, cache_path: Option<PathBuf>) -> Self {
        Self {
            image,
            protocol: None,
            cache_path,
        }
    }

    /// Initialize the StatefulProtocol using a new picker
    /// This should be called after the image is loaded
    pub fn init_protocol(&mut self) {
        if self.protocol.is_none() {
            // Create a new picker for this protocol (new_resize_protocol requires &mut self)
            let mut picker = Picker::from_query_stdio()
                .unwrap_or_else(|_| Picker::from_fontsize((8, 16)));
            // Note: new_resize_protocol needs owned DynamicImage, but this is only called
            // for protocol initialization which is relatively rare
            self.protocol = Some(picker.new_resize_protocol((*self.image).clone()));
        }
    }

    /// Get or initialize the protocol for rendering
    pub fn get_protocol(&mut self) -> &mut StatefulProtocol {
        self.init_protocol();
        self.protocol.as_mut().unwrap()
    }
}

/// Image loading state
pub enum ImageState {
    /// Image is being downloaded
    Loading,
    /// Image loaded successfully
    Loaded(CachedImageData),
    /// Image failed to load
    Failed(String),
}

/// Local disk cache for images
pub struct ImageDiskCache {
    /// Cache directory path
    cache_dir: PathBuf,
}

impl ImageDiskCache {
    pub fn new(data_dir: &PathBuf) -> std::io::Result<Self> {
        let cache_dir = data_dir.join("image_cache");
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Generate a cache filename from URL
    fn url_to_filename(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();

        // Extract extension from URL if possible
        let ext = url
            .rsplit('.')
            .next()
            .and_then(|e| {
                let e = e.split('?').next().unwrap_or(e);
                if ["jpg", "jpeg", "png", "gif", "webp"].contains(&e.to_lowercase().as_str()) {
                    Some(e.to_lowercase())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "bin".to_string());

        format!("{:016x}.{}", hash, ext)
    }

    /// Get the cache path for a URL
    pub fn cache_path(&self, url: &str) -> PathBuf {
        self.cache_dir.join(Self::url_to_filename(url))
    }

    /// Check if image is cached
    pub fn is_cached(&self, url: &str) -> bool {
        self.cache_path(url).exists()
    }

    /// Load image from disk cache (synchronous)
    pub fn load(&self, url: &str) -> Option<DynamicImage> {
        let path = self.cache_path(url);
        if path.exists() {
            image::open(&path).ok()
        } else {
            None
        }
    }

    /// Load image from disk cache asynchronously
    /// Uses spawn_blocking to avoid blocking the async runtime during I/O and decoding
    pub async fn load_async(&self, url: &str) -> Option<DynamicImage> {
        let path = self.cache_path(url);
        if !path.exists() {
            return None;
        }
        Self::load_async_from_path(&path).await
    }

    /// Load image from a specific path asynchronously (static method)
    /// Uses spawn_blocking to avoid blocking the async runtime during I/O and decoding
    pub async fn load_async_from_path(path: &PathBuf) -> Option<DynamicImage> {
        let path = path.clone();
        tokio::task::spawn_blocking(move || image::open(&path).ok())
            .await
            .ok()
            .flatten()
    }

    /// Save image to disk cache
    pub fn save(&self, url: &str, data: &[u8]) -> std::io::Result<()> {
        let path = self.cache_path(url);
        std::fs::write(path, data)
    }
}

/// Represents a content element in the article
#[derive(Clone, Debug)]
pub enum ContentElement {
    /// Plain text paragraph
    Text(String),
    /// Heading with level (1-6) and text
    Heading(u8, String),
    /// Image with URL
    Image { url: String, alt: Option<String> },
    /// Horizontal rule / separator
    Separator,
    /// Block quote
    Quote(String),
    /// Code block
    Code(String),
    /// List item
    ListItem(String),
    /// Empty line
    EmptyLine,
}

/// A span of text, optionally a hyperlink
#[derive(Clone, Debug)]
pub struct TextSpan {
    pub text: String,
    pub link_url: Option<String>,
}

/// Represents a focusable item in the article content (images and links)
#[derive(Clone, Debug)]
pub enum FocusableItem {
    /// An image with its index in image_urls
    Image { url_index: usize },
    /// A hyperlink
    Link {
        url: String,
        text: String,
        element_index: usize,
    },
}

/// Parsed rich content ready for rendering
#[derive(Clone)]
pub struct RichContent {
    /// The parsed content elements
    pub elements: Vec<ContentElement>,
    /// Image URLs found in content (for preloading)
    pub image_urls: Vec<String>,
    /// All focusable items (images + links) in document order
    pub focusable_items: Vec<FocusableItem>,
}

impl RichContent {
    /// Extract all image URLs from HTML content without full parsing
    /// Used for preloading images before entering article detail view
    pub fn extract_image_urls(html: &str) -> Vec<String> {
        let mut urls = Vec::new();
        let mut remaining = html;

        while let Some(img_start) = remaining.to_lowercase().find("<img") {
            remaining = &remaining[img_start..];
            if let Some(tag_end) = remaining.find('>') {
                let tag = &remaining[..=tag_end];
                if let Some(src) = extract_attr(tag, "src") {
                    if !src.is_empty() && !urls.contains(&src) {
                        urls.push(src);
                    }
                }
                remaining = &remaining[tag_end + 1..];
            } else {
                break;
            }
        }

        urls
    }

    /// Parse HTML content into rich content elements
    pub fn from_html(html: &str) -> Self {
        let mut elements = Vec::new();
        let mut image_urls = Vec::new();

        // Remove script and style tags first
        let cleaned = remove_tags(html, &["script", "style", "noscript"]);

        // Parse the HTML content
        parse_html_content(&cleaned, &mut elements, &mut image_urls);

        // Clean up consecutive empty lines
        let elements = collapse_empty_lines(elements);

        // Build focusable items (images + links) in document order
        let focusable_items = build_focusable_items(&elements, &image_urls);

        Self {
            elements,
            image_urls,
            focusable_items,
        }
    }

    /// Create from plain text (for fallback)
    pub fn from_text(text: &str) -> Self {
        let elements: Vec<ContentElement> = text
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    ContentElement::EmptyLine
                } else {
                    ContentElement::Text(line.to_string())
                }
            })
            .collect();

        let image_urls = Vec::new();
        // Build focusable items (only bare URLs in plain text, no images)
        let focusable_items = build_focusable_items(&elements, &image_urls);

        Self {
            elements,
            image_urls,
            focusable_items,
        }
    }
}

/// Remove specific HTML tags and their content
fn remove_tags(html: &str, tags: &[&str]) -> String {
    let mut result = html.to_string();
    for tag in tags {
        let start_pattern = format!("<{}", tag);
        let end_pattern = format!("</{}>", tag);

        loop {
            let lower: Vec<u8> = result.as_bytes().iter().map(|b| b.to_ascii_lowercase()).collect();
            let Some(start) = find_subslice(&lower, start_pattern.as_bytes()) else {
                break;
            };
            let Some(end_start) = find_subslice(&lower[start..], end_pattern.as_bytes()) else {
                break;
            };
            let end = start + end_start + end_pattern.len();
            result.replace_range(start..end, "");
        }
    }
    result
}

/// Parse HTML content into elements
fn parse_html_content(html: &str, elements: &mut Vec<ContentElement>, image_urls: &mut Vec<String>) {
    let mut remaining = html;
    let mut current_text = String::new();

    while !remaining.is_empty() {
        if let Some(tag_start) = remaining.find('<') {
            // Add text before the tag
            let text_before = &remaining[..tag_start];
            if !text_before.trim().is_empty() {
                current_text.push_str(&decode_html_entities(text_before));
            }

            remaining = &remaining[tag_start..];

            // Find end of tag
            if let Some(tag_end) = remaining.find('>') {
                let full_tag = &remaining[..=tag_end];
                let tag_content = &remaining[1..tag_end];
                remaining = &remaining[tag_end + 1..];

                // Parse the tag
                let tag_name = tag_content
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches('/')
                    .to_lowercase();

                match tag_name.as_str() {
                    "img" => {
                        // Flush current text
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }

                        // Extract src and alt
                        if let Some(src) = extract_attr(tag_content, "src") {
                            if !src.is_empty() && !image_urls.contains(&src) {
                                image_urls.push(src.clone());
                            }
                            let alt = extract_attr(tag_content, "alt");
                            elements.push(ContentElement::Image { url: src, alt });
                        }
                    }
                    "br" => {
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        elements.push(ContentElement::EmptyLine);
                    }
                    "p" | "div" => {
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        // Check for closing tag with content
                        if !full_tag.ends_with("/>") {
                            let close_tag = format!("</{}>", tag_name);
                            if let Some(close_pos) = remaining.to_lowercase().find(&close_tag) {
                                let inner = &remaining[..close_pos];
                                let skip = close_pos + close_tag.len();
                                remaining = if skip <= remaining.len() {
                                    &remaining[skip..]
                                } else {
                                    ""
                                };

                                // Recursively parse inner content
                                parse_html_content(inner, elements, image_urls);
                                elements.push(ContentElement::EmptyLine);
                            }
                        }
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        let level = tag_name.chars().nth(1).and_then(|c| c.to_digit(10)).unwrap_or(1) as u8;
                        let close_tag = format!("</{}>", tag_name);
                        if let Some(close_pos) = remaining.to_lowercase().find(&close_tag) {
                            let inner = &remaining[..close_pos];
                            let skip = close_pos + close_tag.len();
                            remaining = if skip <= remaining.len() {
                                &remaining[skip..]
                            } else {
                                ""
                            };
                            let text = strip_html_tags(inner);
                            if !text.trim().is_empty() {
                                elements.push(ContentElement::Heading(level, text.trim().to_string()));
                            }
                        }
                    }
                    "hr" => {
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        elements.push(ContentElement::Separator);
                    }
                    "blockquote" => {
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        let close_tag = "</blockquote>";
                        if let Some(close_pos) = remaining.to_lowercase().find(close_tag) {
                            let inner = &remaining[..close_pos];
                            let skip = close_pos + close_tag.len();
                            remaining = remaining.get(skip..).unwrap_or("");
                            let text = strip_html_tags(inner);
                            if !text.trim().is_empty() {
                                elements.push(ContentElement::Quote(text.trim().to_string()));
                            }
                        }
                    }
                    "pre" | "code" => {
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        let close_tag = format!("</{}>", tag_name);
                        if let Some(close_pos) = remaining.to_lowercase().find(&close_tag) {
                            let inner = &remaining[..close_pos];
                            let skip = close_pos + close_tag.len();
                            remaining = remaining.get(skip..).unwrap_or("");
                            let text = strip_html_tags(inner);
                            if !text.trim().is_empty() {
                                elements.push(ContentElement::Code(decode_html_entities(&text)));
                            }
                        }
                    }
                    "li" => {
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        let close_tag = "</li>";
                        if let Some(close_pos) = remaining.to_lowercase().find(close_tag) {
                            let inner = &remaining[..close_pos];
                            let skip = close_pos + close_tag.len();
                            remaining = remaining.get(skip..).unwrap_or("");
                            let text = strip_html_tags(inner);
                            if !text.trim().is_empty() {
                                elements.push(ContentElement::ListItem(text.trim().to_string()));
                            }
                        }
                    }
                    "a" => {
                        // Handle anchor/link elements - extract href and link text
                        if let Some(href) = extract_attr(tag_content, "href") {
                            let close_tag = "</a>";
                            if let Some(close_pos) = remaining.to_lowercase().find(close_tag) {
                                let inner = &remaining[..close_pos];
                                let skip = close_pos + close_tag.len();
                                remaining = remaining.get(skip..).unwrap_or("");
                                let link_text = strip_html_tags(inner).trim().to_string();

                                // Only add if href is a valid URL (http/https)
                                if href.starts_with("http://") || href.starts_with("https://") {
                                    if !link_text.is_empty() {
                                        // If link text is different from URL, show both
                                        if link_text != href && !link_text.contains(&href) {
                                            current_text.push_str(&format!("{} ({})", link_text, href));
                                        } else {
                                            // Link text is the URL or contains it
                                            current_text.push_str(&link_text);
                                        }
                                    } else {
                                        // No link text, just show URL
                                        current_text.push_str(&href);
                                    }
                                } else if !link_text.is_empty() {
                                    // Non-http link (mailto, etc.), just add the text
                                    current_text.push_str(&link_text);
                                }
                            }
                        }
                    }
                    "figure" => {
                        // Handle figure element (common in modern articles)
                        if !current_text.trim().is_empty() {
                            elements.push(ContentElement::Text(current_text.trim().to_string()));
                            current_text.clear();
                        }
                        let close_tag = "</figure>";
                        if let Some(close_pos) = remaining.to_lowercase().find(close_tag) {
                            let inner = &remaining[..close_pos];
                            let skip = close_pos + close_tag.len();
                            remaining = remaining.get(skip..).unwrap_or("");
                            // Recursively parse to find images inside
                            parse_html_content(inner, elements, image_urls);
                        }
                    }
                    _ => {
                        // For other tags, just continue parsing
                    }
                }
            } else {
                // Malformed HTML, skip the <
                current_text.push('<');
                remaining = &remaining[1..];
            }
        } else {
            // No more tags, add remaining text
            if !remaining.trim().is_empty() {
                current_text.push_str(&decode_html_entities(remaining));
            }
            break;
        }
    }

    // Flush any remaining text
    if !current_text.trim().is_empty() {
        elements.push(ContentElement::Text(current_text.trim().to_string()));
    }
}

/// Extract an attribute value from an HTML tag
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let patterns = [
        format!("{}=\"", attr),
        format!("{}='", attr),
    ];

    for pattern in &patterns {
        if let Some(start) = tag.to_lowercase().find(&pattern.to_lowercase()) {
            let value_start = start + pattern.len();
            let rest = &tag[value_start..];
            let end_char = if pattern.ends_with('"') { '"' } else { '\'' };
            if let Some(end) = rest.find(end_char) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Strip all HTML tags from text
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }

    decode_html_entities(&result)
}

/// Decode common HTML entities
fn decode_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'" )
        .replace("&#39;", "'" )
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'" )
        .replace("&#x2F;", "/")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
        .replace("&rsquo;", "'" )
        .replace("&lsquo;", "'" )
        .replace("&rdquo;", "\"")
        .replace("&ldquo;", "\"")
}

/// Get compiled URL regex (lazy initialization)
fn get_url_regex() -> &'static Regex {
    static URL_REGEX: OnceLock<Regex> = OnceLock::new();
    URL_REGEX.get_or_init(|| {
        Regex::new(r#"https?://[^\s<>\[\](){}"']+"#).unwrap()
    })
}

/// Parse text and extract bare URLs, returning TextSpans
pub fn parse_text_with_urls(text: &str) -> Vec<TextSpan> {
    let regex = get_url_regex();
    let mut spans = Vec::new();
    let mut last_end = 0;

    for mat in regex.find_iter(text) {
        // Add text before the URL
        if mat.start() > last_end {
            spans.push(TextSpan {
                text: text[last_end..mat.start()].to_string(),
                link_url: None,
            });
        }

        // Add the URL as a link
        let url = mat.as_str().to_string();
        spans.push(TextSpan {
            text: url.clone(),
            link_url: Some(url),
        });

        last_end = mat.end();
    }

    // Add remaining text
    if last_end < text.len() {
        spans.push(TextSpan {
            text: text[last_end..].to_string(),
            link_url: None,
        });
    }

    spans
}

/// Build focusable items list from parsed elements in document order
fn build_focusable_items(elements: &[ContentElement], _image_urls: &[String]) -> Vec<FocusableItem> {
    let mut items = Vec::new();
    let mut image_index = 0;

    for (elem_idx, element) in elements.iter().enumerate() {
        match element {
            ContentElement::Image { .. } => {
                items.push(FocusableItem::Image { url_index: image_index });
                image_index += 1;
            }
            ContentElement::Text(text) => {
                // Check for bare URLs in plain text
                let spans = parse_text_with_urls(text);
                for span in &spans {
                    if let Some(ref url) = span.link_url {
                        items.push(FocusableItem::Link {
                            url: url.clone(),
                            text: span.text.clone(),
                            element_index: elem_idx,
                        });
                    }
                }
            }
            ContentElement::Quote(text) | ContentElement::ListItem(text) => {
                // Also check quotes and list items for bare URLs
                let spans = parse_text_with_urls(text);
                for span in &spans {
                    if let Some(ref url) = span.link_url {
                        items.push(FocusableItem::Link {
                            url: url.clone(),
                            text: span.text.clone(),
                            element_index: elem_idx,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    items
}

/// Collapse multiple consecutive empty lines into one
fn collapse_empty_lines(elements: Vec<ContentElement>) -> Vec<ContentElement> {
    let mut result = Vec::new();
    let mut last_was_empty = false;

    for elem in elements {
        match elem {
            ContentElement::EmptyLine => {
                if !last_was_empty && !result.is_empty() {
                    result.push(ContentElement::EmptyLine);
                    last_was_empty = true;
                }
            }
            _ => {
                result.push(elem);
                last_was_empty = false;
            }
        }
    }

    // Remove trailing empty line
    if matches!(result.last(), Some(ContentElement::EmptyLine)) {
        result.pop();
    }

    result
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// In-memory image cache for the current article
pub struct ArticleImageCache {
    /// Image states keyed by URL
    pub images: HashMap<String, ImageState>,
    /// Disk cache reference
    disk_cache: Option<ImageDiskCache>,
}

impl ArticleImageCache {
    pub fn new(data_dir: Option<&PathBuf>) -> Self {
        let disk_cache = data_dir.and_then(|d| ImageDiskCache::new(d).ok());
        Self {
            images: HashMap::new(),
            disk_cache,
        }
    }

    /// Check if an image is ready
    pub fn is_ready(&self, url: &str) -> bool {
        matches!(self.images.get(url), Some(ImageState::Loaded(_)))
    }

    /// Check if an image is loading
    pub fn is_loading(&self, url: &str) -> bool {
        matches!(self.images.get(url), Some(ImageState::Loading))
    }

    /// Get a loaded image
    pub fn get(&self, url: &str) -> Option<&CachedImageData> {
        match self.images.get(url) {
            Some(ImageState::Loaded(data)) => Some(data),
            _ => None,
        }
    }

    /// Get a mutable loaded image for rendering
    pub fn get_mut(&mut self, url: &str) -> Option<&mut CachedImageData> {
        match self.images.get_mut(url) {
            Some(ImageState::Loaded(data)) => Some(data),
            _ => None,
        }
    }

    /// Mark an image as loading
    pub fn start_loading(&mut self, url: &str) {
        if !self.images.contains_key(url) {
            self.images.insert(url.to_string(), ImageState::Loading);
        }
    }

    /// Set image as loaded with optional cache path
    pub fn set_loaded(&mut self, url: &str, image: DynamicImage, cache_path: Option<PathBuf>) {
        let cached = CachedImageData::new(image, cache_path);
        // Note: We no longer pre-initialize StatefulProtocol as it can cause
        // ghost images in Kitty terminal. Images are rendered using halfblocks
        // or native protocol implementations.
        self.images
            .insert(url.to_string(), ImageState::Loaded(cached));
    }

    /// Set image as loaded from an Arc (avoids unnecessary cloning)
    pub fn set_loaded_arc(&mut self, url: &str, image: Arc<DynamicImage>, cache_path: Option<PathBuf>) {
        let cached = CachedImageData::new_arc(image, cache_path);
        self.images
            .insert(url.to_string(), ImageState::Loaded(cached));
    }

    /// Set image as failed
    pub fn set_failed(&mut self, url: &str, error: String) {
        self.images.insert(url.to_string(), ImageState::Failed(error));
    }

    /// Try to load from disk cache
    pub fn try_load_from_disk(&mut self, url: &str) -> bool {
        if let Some(ref disk) = self.disk_cache {
            if let Some(img) = disk.load(url) {
                let cache_path = Some(disk.cache_path(url));
                let cached = CachedImageData::new(img, cache_path);
                self.images
                    .insert(url.to_string(), ImageState::Loaded(cached));
                return true;
            }
        }
        false
    }

    /// Get the disk cache path for a URL
    pub fn get_cache_path(&self, url: &str) -> Option<PathBuf> {
        self.disk_cache.as_ref().map(|d| d.cache_path(url))
    }

    /// Save to disk cache
    pub fn save_to_disk(&self, url: &str, data: &[u8]) {
        if let Some(ref disk) = self.disk_cache {
            let _ = disk.save(url, data);
        }
    }

    /// Clear all cached images
    pub fn clear(&mut self) {
        self.images.clear();
    }

    /// Get loading status message
    pub fn get_status(&self, url: &str) -> Option<String> {
        match self.images.get(url) {
            Some(ImageState::Loading) => Some("[Loading...]".to_string()),
            Some(ImageState::Failed(err)) => Some(format!("[Failed: {} ]", err)),
            _ => None,
        }
    }
}

/// Cache key for pre-resized images with quantized dimensions
#[derive(Hash, Eq, PartialEq, Clone)]
struct ResizedCacheKey {
    url: String,
    /// Quantized width bucket (rounded to WIDTH_BUCKET_SIZE)
    width_bucket: u16,
    /// Quantized height bucket (rounded to HEIGHT_BUCKET_SIZE)
    height_bucket: u16,
}

/// Pre-resized and converted image data ready for halfblock rendering
pub struct ResizedImageData {
    /// RGBA pixel data ready for rendering
    pub rgba: RgbaImage,
    /// Actual pixel dimensions after resize
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Timestamp for LRU eviction
    last_used: Instant,
}

/// LRU cache for pre-resized images to avoid repeated resize operations
pub struct ResizedImageCache {
    cache: HashMap<ResizedCacheKey, ResizedImageData>,
}

impl ResizedImageCache {
    /// Maximum number of cached resized images (increased from 50 for better scroll performance)
    const MAX_ENTRIES: usize = 100;
    /// Width quantization bucket size (columns)
    const WIDTH_BUCKET_SIZE: u16 = 8;
    /// Height quantization bucket size (rows)
    const HEIGHT_BUCKET_SIZE: u16 = 4;

    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Quantize dimensions to bucket for cache stability during scroll
    fn quantize_dimensions(width: u16, height: u16) -> (u16, u16) {
        let w_bucket = ((width + Self::WIDTH_BUCKET_SIZE - 1) / Self::WIDTH_BUCKET_SIZE)
            * Self::WIDTH_BUCKET_SIZE;
        let h_bucket = ((height + Self::HEIGHT_BUCKET_SIZE - 1) / Self::HEIGHT_BUCKET_SIZE)
            * Self::HEIGHT_BUCKET_SIZE;
        (w_bucket.max(Self::WIDTH_BUCKET_SIZE), h_bucket.max(Self::HEIGHT_BUCKET_SIZE))
    }

    /// Get or create a pre-resized image for the given dimensions
    /// Returns the cached RGBA data, avoiding expensive resize/convert on every frame
    pub fn get_or_resize(
        &mut self,
        url: &str,
        source_image: &DynamicImage,
        target_width: u16,
        target_height: u16,
    ) -> &ResizedImageData {
        let (width_bucket, height_bucket) = Self::quantize_dimensions(target_width, target_height);
        let key = ResizedCacheKey {
            url: url.to_string(),
            width_bucket,
            height_bucket,
        };

        // Check if already cached
        if self.cache.contains_key(&key) {
            // Update last_used and return
            let entry = self.cache.get_mut(&key).unwrap();
            entry.last_used = Instant::now();
            return self.cache.get(&key).unwrap();
        }

        // Not cached - resize and store
        // Calculate actual pixel dimensions (halfblocks use 2 pixels per row)
        let pixel_width = width_bucket as u32;
        let pixel_height = (height_bucket as u32) * 2;

        // Resize image
        let resized = source_image.resize_exact(
            pixel_width,
            pixel_height,
            image::imageops::FilterType::Triangle,
        );

        // Convert to RGBA
        let rgba = resized.to_rgba8();

        let data = ResizedImageData {
            rgba,
            pixel_width,
            pixel_height,
            last_used: Instant::now(),
        };

        // Evict old entries if needed
        self.evict_if_needed();

        // Store and return
        self.cache.insert(key.clone(), data);
        self.cache.get(&key).unwrap()
    }

    /// Evict oldest entries if cache is over capacity
    fn evict_if_needed(&mut self) {
        if self.cache.len() < Self::MAX_ENTRIES {
            return;
        }

        // Find oldest entries by last_used timestamp
        let mut entries: Vec<_> = self.cache.iter()
            .map(|(k, v)| (k.clone(), v.last_used))
            .collect();
        entries.sort_by_key(|(_, t)| *t);

        // Evict oldest 25%
        let to_remove = self.cache.len() / 4;
        for (key, _) in entries.into_iter().take(to_remove) {
            self.cache.remove(&key);
        }
    }

    /// Clear all cached images (call on article switch)
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics for debugging
    #[allow(dead_code)]
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), Self::MAX_ENTRIES)
    }
}

impl Default for ResizedImageCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode image bytes asynchronously using spawn_blocking to avoid blocking the async runtime
pub async fn decode_image_bytes_async(bytes: Vec<u8>) -> Result<DynamicImage, String> {
    tokio::task::spawn_blocking(move || decode_image_bytes(&bytes))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

/// Download image and decode it
pub async fn download_image(url: &str) -> Result<(Vec<u8>, DynamicImage), String> {
    parse_http_url(url)?;
    let bytes = download_image_bytes(url).await?;
    // Use async decoding to avoid blocking the main thread
    let image = decode_image_bytes_async(bytes.clone()).await?;
    Ok((bytes, image))
}

/// Download image bytes using curl (most compatible)
async fn download_image_bytes(url: &str) -> Result<Vec<u8>, String> {
    use std::process::Command;

    let parsed = parse_http_url(url)?;
    let referer = build_referer(&parsed);

    let output = tokio::task::spawn_blocking({
        let url = url.to_string();
        let referer = referer.clone();
        move || {
            let max_bytes = MAX_IMAGE_BYTES.to_string();
            Command::new("curl")
                .args([
                    "-sL",
                    "--max-time", "15",
                    "--max-filesize", &max_bytes,
                    "-A", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
                    "-H", &format!("Referer: {}", referer),
                    "-H", "Accept: image/png,image/jpeg,image/gif,image/*;q=0.8",
                    &url,
                ])
                .output()
        }
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))? // Added missing closing parenthesis
    .map_err(|e| format!("Curl failed: {}", e))?;

    if output.status.success() && !output.stdout.is_empty() {
        if output.stdout.len() > MAX_IMAGE_BYTES {
            return Err(format!("Image too large ({}B)", output.stdout.len()));
        }
        Ok(output.stdout)
    } else {
        // Fallback to reqwest
        download_with_reqwest(url).await
    }
}

/// Fallback download using reqwest
async fn download_with_reqwest(url: &str) -> Result<Vec<u8>, String> {
    let parsed = parse_http_url(url)?;
    let referer = build_referer(&parsed);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let response = client
        .get(url)
        .header("Accept", "image/png,image/jpeg,image/gif,image/*;q=0.8")
        .header("Referer", &referer)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    if let Some(len) = response.content_length() {
        if len as usize > MAX_IMAGE_BYTES {
            return Err(format!("Image too large ({}B)", len));
        }
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!("Image too large ({}B)", bytes.len()));
    }

    Ok(bytes.to_vec())
}

/// Decode image bytes with format detection
fn decode_image_bytes(bytes: &[u8]) -> Result<DynamicImage, String> {
    if bytes.is_empty() {
        return Err("Empty data".to_string());
    }

    // Try auto-detection first
    if let Ok(img) = image::load_from_memory(bytes) {
        return Ok(img);
    }

    // Try based on magic bytes
    if bytes.len() >= 8 {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .map_err(|e| format!("PNG: {}", e));
        }
        if bytes.starts_with(b"\xff\xd8\xff") {
            return image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
                .map_err(|e| format!("JPEG: {}", e));
        }
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            return image::load_from_memory_with_format(bytes, image::ImageFormat::Gif)
                .map_err(|e| format!("GIF: {}", e));
        }
        if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
            return image::load_from_memory_with_format(bytes, image::ImageFormat::WebP)
                .map_err(|e| format!("WebP: {}", e));
        }
    }

    Err(format!("Unknown format ({}B)", bytes.len()))
}

fn parse_http_url(url: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        scheme => Err(format!("Unsupported URL scheme: {}", scheme)),
    }
}

fn build_referer(parsed: &url::Url) -> String {
    match parsed.host_str() {
        Some(host) => match parsed.port() {
            Some(port) => format!("{}://{}:{}", parsed.scheme(), host, port),
            None => format!("{}://{}", parsed.scheme(), host),
        },
        None => String::new(),
    }
}

/// Global preload cache for prefetching images before entering article detail
/// Shares the same ImageState enum as ArticleImageCache
pub struct PreloadCache {
    /// Image states keyed by URL
    pub images: HashMap<String, ImageState>,
    /// Disk cache reference
    disk_cache: Option<ImageDiskCache>,
}

impl PreloadCache {
    pub fn new(data_dir: Option<&PathBuf>) -> Self {
        let disk_cache = data_dir.and_then(|d| ImageDiskCache::new(d).ok());
        Self {
            images: HashMap::new(),
            disk_cache,
        }
    }

    /// Check if an image is ready (loaded successfully)
    pub fn is_ready(&self, url: &str) -> bool {
        matches!(self.images.get(url), Some(ImageState::Loaded(_)))
    }

    /// Check if an image is currently loading
    pub fn is_loading(&self, url: &str) -> bool {
        matches!(self.images.get(url), Some(ImageState::Loading))
    }

    /// Get a loaded image
    pub fn get(&self, url: &str) -> Option<&CachedImageData> {
        match self.images.get(url) {
            Some(ImageState::Loaded(data)) => Some(data),
            _ => None,
        }
    }

    /// Mark an image as loading
    pub fn start_loading(&mut self, url: &str) {
        if !self.images.contains_key(url) {
            self.images.insert(url.to_string(), ImageState::Loading);
        }
    }

    /// Set image as loaded with optional cache path
    pub fn set_loaded(&mut self, url: &str, image: DynamicImage, cache_path: Option<PathBuf>) {
        let cached = CachedImageData::new(image, cache_path);
        self.images
            .insert(url.to_string(), ImageState::Loaded(cached));
    }

    /// Set image as loaded from an Arc (avoids unnecessary cloning)
    pub fn set_loaded_arc(&mut self, url: &str, image: Arc<DynamicImage>, cache_path: Option<PathBuf>) {
        let cached = CachedImageData::new_arc(image, cache_path);
        self.images
            .insert(url.to_string(), ImageState::Loaded(cached));
    }

    /// Set image as failed
    pub fn set_failed(&mut self, url: &str, error: String) {
        self.images.insert(url.to_string(), ImageState::Failed(error));
    }

    /// Get the disk cache instance for loading images
    pub fn disk_cache(&self) -> Option<&ImageDiskCache> {
        self.disk_cache.as_ref()
    }

    /// Clear all cached images (call on feed switch)
    pub fn clear(&mut self) {
        self.images.clear();
    }

    /// Get cache statistics for debugging
    #[allow(dead_code)]
    pub fn stats(&self) -> usize {
        self.images.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let html = r#"<p>Hello world</p><img src="test.jpg" alt="Test"><p>More text</p>"#;
        let content = RichContent::from_html(html);

        assert!(!content.elements.is_empty());
        assert_eq!(content.image_urls.len(), 1);
        assert_eq!(content.image_urls[0], "test.jpg");
    }

    #[test]
    fn test_parse_heading() {
        let html = "<h1>Title</h1><h2>Subtitle</h2>";
        let content = RichContent::from_html(html);

        let headings: Vec<_> = content.elements.iter()
            .filter(|e| matches!(e, ContentElement::Heading(_, _)))
            .collect();
        assert_eq!(headings.len(), 2);
    }

    #[test]
    fn test_image_extraction() {
        let html = r#"<img src="a.jpg"><div><img src="b.png"></div><img src="c.gif">"#;
        let content = RichContent::from_html(html);

        assert_eq!(content.image_urls.len(), 3);
    }

    #[test]
    fn test_extract_image_urls() {
        let html = r#"<p>Some text</p><img src="cover.jpg"><div><IMG SRC="nested.png"></div><img src='single.gif'>"#;
        let urls = RichContent::extract_image_urls(html);

        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "cover.jpg");
        assert_eq!(urls[1], "nested.png");
        assert_eq!(urls[2], "single.gif");
    }

    #[test]
    fn test_extract_image_urls_no_duplicates() {
        let html = r#"<img src="same.jpg"><img src="same.jpg"><img src="different.png">"#;
        let urls = RichContent::extract_image_urls(html);

        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "same.jpg");
        assert_eq!(urls[1], "different.png");
    }
}
