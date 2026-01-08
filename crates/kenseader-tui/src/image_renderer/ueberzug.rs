//! Üeberzug++ subprocess communication
//!
//! Implements the JSON protocol for communicating with ueberzugpp.
//! See: https://github.com/jstkdng/ueberzugpp

use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Üeberzug++ instance that manages the subprocess
pub struct UeberzugInstance {
    process: Child,
    writer: BufWriter<std::process::ChildStdin>,
}

impl UeberzugInstance {
    /// Start a new ueberzugpp layer process
    pub fn new() -> std::io::Result<Self> {
        // Start ueberzugpp in layer mode with silent output
        let mut process = Command::new("ueberzugpp")
            .args(["layer", "--silent"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to get stdin"))?;

        let writer = BufWriter::new(stdin);

        tracing::info!("Started ueberzugpp subprocess (PID: {})", process.id());

        Ok(Self { process, writer })
    }

    /// Add/update an image at the specified position
    ///
    /// # Arguments
    /// * `identifier` - Unique identifier for this image (used for updates/removal)
    /// * `path` - Path to the image file
    /// * `x` - X position in terminal columns
    /// * `y` - Y position in terminal rows
    /// * `width` - Maximum width in terminal columns
    /// * `height` - Maximum height in terminal rows
    pub fn add(
        &mut self,
        identifier: &str,
        path: &Path,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> std::io::Result<()> {
        // Build JSON command
        // Format: {"action":"add","identifier":"id","path":"/path","x":0,"y":0,"max_width":10,"max_height":10}
        let cmd = format!(
            r#"{{"action":"add","identifier":"{}","path":"{}","x":{},"y":{},"max_width":{},"max_height":{}}}"#,
            escape_json_string(identifier),
            escape_json_string(&path.to_string_lossy()),
            x,
            y,
            width,
            height
        );

        writeln!(self.writer, "{}", cmd)?;
        self.writer.flush()?;

        tracing::trace!(
            "ueberzugpp add: id={}, path={}, pos=({},{}), size={}x{}",
            identifier,
            path.display(),
            x,
            y,
            width,
            height
        );

        Ok(())
    }

    /// Remove an image by identifier
    pub fn remove(&mut self, identifier: &str) -> std::io::Result<()> {
        let cmd = format!(
            r#"{{"action":"remove","identifier":"{}"}}"#,
            escape_json_string(identifier)
        );

        writeln!(self.writer, "{}", cmd)?;
        self.writer.flush()?;

        tracing::trace!("ueberzugpp remove: id={}", identifier);

        Ok(())
    }

    /// Check if the subprocess is still running
    pub fn is_alive(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => false,
            Err(_) => false,
        }
    }
}

impl Drop for UeberzugInstance {
    fn drop(&mut self) {
        // Try to gracefully terminate the process
        let _ = self.process.kill();
        let _ = self.process.wait();
        tracing::debug!("Terminated ueberzugpp subprocess");
    }
}

/// Escape a string for JSON
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_json_string() {
        assert_eq!(escape_json_string("hello"), "hello");
        assert_eq!(escape_json_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json_string("path\\to\\file"), "path\\\\to\\\\file");
        assert_eq!(escape_json_string("line1\nline2"), "line1\\nline2");
    }
}
