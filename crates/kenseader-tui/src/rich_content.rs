use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::Protocol;

/// Get the global image picker instance with automatic protocol detection
pub fn get_image_picker() -> &'static Picker {
    static PICKER: OnceLock<Picker> = OnceLock::new();
    PICKER.get_or_init(|| {
        // Try to query terminal capabilities for best protocol
        // Falls back to halfblocks if query fails
        Picker::from_query_stdio().unwrap_or_else(|_| {
            Picker::from_fontsize((8, 16))
        })
    })
}

/// A cached image with its data and render state
pub struct CachedImageData {
    /// The raw image data
    pub image: DynamicImage,
    /// The protocol-specific rendered data (cached)
    pub protocol: Option<Protocol>,
}

impl CachedImageData {
    pub fn new(image: DynamicImage) -> Self {
        Self {
            image,
            protocol: None,
        }
    }

    /// Get the protocol for rendering, generating it if necessary
    /// Creates a new Picker each time since the global one is immutable
    pub fn get_protocol(&mut self, area: ratatui::layout::Rect) -> &mut Protocol {
        if self.protocol.is_none() {
            // Create a new picker for this operation
            let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| {
                Picker::from_fontsize((8, 16))
            });
            self.protocol = Some(picker.new_protocol(
                self.image.clone(),
                area,
                ratatui_image::Resize::Fit(None),
            ).unwrap());
        }
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

    /// Load image from disk cache
    pub fn load(&self, url: &str) -> Option<DynamicImage> {
        let path = self.cache_path(url);
        if path.exists() {
            image::open(&path).ok()
        } else {
            None
        }
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

/// Parsed rich content ready for rendering
#[derive(Clone)]
pub struct RichContent {
    /// The parsed content elements
    pub elements: Vec<ContentElement>,
    /// Image URLs found in content (for preloading)
    pub image_urls: Vec<String>,
}

impl RichContent {
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

        Self {
            elements,
            image_urls,
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

        Self {
            elements,
            image_urls: Vec::new(),
        }
    }
}

/// Remove specific HTML tags and their content
fn remove_tags(html: &str, tags: &[&str]) -> String {
    let mut result = html.to_string();
    for tag in tags {
        // Remove <tag>...</tag>
        let pattern = format!("<{}[^>]*>", tag);
        let end_pattern = format!("</{}>", tag);

        loop {
            if let Some(start) = result.to_lowercase().find(&pattern.to_lowercase()) {
                if let Some(end_start) = result[start..].to_lowercase().find(&end_pattern.to_lowercase()) {
                    let end = start + end_start + end_pattern.len();
                    result = format!("{}{}", &result[..start], &result[end..]);
                } else {
                    break;
                }
            } else {
                break;
            }
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

    /// Set image as loaded
    pub fn set_loaded(&mut self, url: &str, image: DynamicImage) {
        self.images.insert(url.to_string(), ImageState::Loaded(CachedImageData::new(image)));
    }

    /// Set image as failed
    pub fn set_failed(&mut self, url: &str, error: String) {
        self.images.insert(url.to_string(), ImageState::Failed(error));
    }

    /// Try to load from disk cache
    pub fn try_load_from_disk(&mut self, url: &str) -> bool {
        if let Some(ref disk) = self.disk_cache {
            if let Some(img) = disk.load(url) {
                self.images.insert(url.to_string(), ImageState::Loaded(CachedImageData::new(img)));
                return true;
            }
        }
        false
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

/// Download image and decode it
pub async fn download_image(url: &str) -> Result<(Vec<u8>, DynamicImage), String> {
    let bytes = download_image_bytes(url).await?;
    let image = decode_image_bytes(&bytes)?;
    Ok((bytes, image))
}

/// Download image bytes using curl (most compatible)
async fn download_image_bytes(url: &str) -> Result<Vec<u8>, String> {
    use std::process::Command;

    let referer = url::Url::parse(url)
        .ok()
        .map(|u| format!("{}:/{{}}/{}", u.scheme(), u.host_str().unwrap_or("")))
        .unwrap_or_default();

    let output = tokio::task::spawn_blocking({
        let url = url.to_string();
        let referer = referer.clone();
        move || {
            Command::new("curl")
                .args([
                    "-sL",
                    "--max-time", "15",
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
        Ok(output.stdout)
    } else {
        // Fallback to reqwest
        download_with_reqwest(url).await
    }
}

/// Fallback download using reqwest
async fn download_with_reqwest(url: &str) -> Result<Vec<u8>, String> {
    let referer = url::Url::parse(url)
        .ok()
        .map(|u| format!("{}:/{{}}/{}", u.scheme(), u.host_str().unwrap_or("")))
        .unwrap_or_default();

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

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Read error: {}", e))
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
}