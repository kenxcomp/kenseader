use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::Result;

/// A feed entry extracted from OPML file
#[derive(Debug, Clone)]
pub struct OpmlFeed {
    pub url: String,
    pub name: String,
}

/// Parse OPML file and extract feed entries
pub fn parse_opml_file(path: &Path) -> Result<Vec<OpmlFeed>> {
    let content = std::fs::read_to_string(path)?;
    parse_opml(&content)
}

/// Parse OPML content string
pub fn parse_opml(content: &str) -> Result<Vec<OpmlFeed>> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut feeds = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) if e.name().as_ref() == b"outline" => {
                let mut xml_url = None;
                let mut name = None;

                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"xmlUrl" => {
                            xml_url = Some(String::from_utf8_lossy(&attr.value).to_string())
                        }
                        b"title" => name = Some(String::from_utf8_lossy(&attr.value).to_string()),
                        b"text" if name.is_none() => {
                            name = Some(String::from_utf8_lossy(&attr.value).to_string())
                        }
                        _ => {}
                    }
                }

                // Only add if xmlUrl exists (actual feed, not category)
                if let Some(url) = xml_url {
                    feeds.push(OpmlFeed {
                        url,
                        name: name.unwrap_or_else(|| "Unnamed Feed".to_string()),
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::Error::FeedParse(format!("Failed to parse OPML: {}", e)));
            }
            _ => {}
        }
    }

    Ok(feeds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_opml() {
        let opml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head><title>Test</title></head>
  <body>
    <outline text="Category">
      <outline text="Feed 1" title="Feed One" xmlUrl="https://example.com/feed1.xml" type="rss"/>
      <outline text="Feed 2" xmlUrl="https://example.com/feed2.xml" type="rss"/>
    </outline>
    <outline text="Direct Feed" xmlUrl="https://example.com/feed3.xml" type="rss"/>
  </body>
</opml>"#;

        let feeds = parse_opml(opml).unwrap();
        assert_eq!(feeds.len(), 3);
        assert_eq!(feeds[0].name, "Feed One"); // title takes precedence
        assert_eq!(feeds[0].url, "https://example.com/feed1.xml");
        assert_eq!(feeds[1].name, "Feed 2"); // fallback to text
        assert_eq!(feeds[2].name, "Direct Feed");
    }

    #[test]
    fn test_parse_opml_empty_category() {
        let opml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <body>
    <outline text="Empty Category"/>
  </body>
</opml>"#;

        let feeds = parse_opml(opml).unwrap();
        assert_eq!(feeds.len(), 0); // Categories without xmlUrl are skipped
    }
}
