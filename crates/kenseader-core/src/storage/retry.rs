//! Shared retry logic for SQLite operations in cloud sync scenarios
//!
//! When the database is stored in a cloud-synced directory (iCloud, Dropbox, etc.),
//! transient I/O errors can occur during file synchronization. This module provides
//! retry mechanisms for both read and write operations to handle these gracefully.

use std::future::Future;
use std::time::Duration;

/// Maximum number of retry attempts for database operations
pub const MAX_RETRIES: u32 = 5;

/// Check if a SQLite error is transient and should be retried
///
/// This includes:
/// - SQLITE_BUSY (5): Database locked by another connection
/// - SQLITE_LOCKED (6): Database table is locked
/// - SQLITE_IOERR (10): Base I/O error
/// - SQLITE_IOERR_READ (266): I/O error during read (10 | 1<<8)
/// - SQLITE_IOERR_SHORT_READ (522): Read returned fewer bytes than expected (10 | 2<<8)
/// - SQLITE_BUSY_SNAPSHOT (1032): Busy due to WAL snapshot (5 | 4<<8)
/// - SQLITE_IOERR_WRITE (2314): I/O error during write (10 | 9<<8)
/// - SQLITE_IOERR_FSYNC (3338): I/O error during fsync (10 | 13<<8)
/// - SQLITE_IOERR_DIR_FSYNC (4618): I/O error during dir fsync (10 | 18<<8)
/// - SQLITE_IOERR_LOCK (5386): I/O error getting file lock (10 | 21<<8)
/// - SQLITE_IOERR_CLOSE (5642): I/O error during close (10 | 22<<8)
pub fn is_transient_error(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            let code = db_err.code().map(|c| c.to_string());
            matches!(
                code.as_deref(),
                Some("5")     // SQLITE_BUSY
                | Some("6")   // SQLITE_LOCKED
                | Some("10")  // SQLITE_IOERR
                | Some("266") // SQLITE_IOERR_READ
                | Some("522") // SQLITE_IOERR_SHORT_READ
                | Some("1032") // SQLITE_BUSY_SNAPSHOT
                | Some("2314") // SQLITE_IOERR_WRITE
                | Some("3338") // SQLITE_IOERR_FSYNC
                | Some("4618") // SQLITE_IOERR_DIR_FSYNC
                | Some("5386") // SQLITE_IOERR_LOCK
                | Some("5642") // SQLITE_IOERR_CLOSE
            )
        }
        _ => false,
    }
}

/// Calculate exponential backoff delay for retry attempt
///
/// Base delay: 200ms, doubling each attempt
/// Delays: 200ms, 400ms, 800ms, 1600ms, 3200ms
fn backoff_delay(attempt: u32) -> Duration {
    Duration::from_millis(200 * 2u64.pow(attempt.saturating_sub(1)))
}

/// Execute a write operation with exponential backoff retry for transient errors
///
/// This is essential for cloud sync scenarios where multiple devices may
/// access the same database file (via iCloud, Dropbox, etc.)
pub async fn execute_with_retry<F, Fut>(operation: F) -> std::result::Result<(), sqlx::Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = std::result::Result<(), sqlx::Error>>,
{
    let mut attempts = 0;
    loop {
        match operation().await {
            Ok(_) => return Ok(()),
            Err(e) if is_transient_error(&e) && attempts < MAX_RETRIES => {
                attempts += 1;
                let delay = backoff_delay(attempts);
                tracing::debug!(
                    error = %e,
                    attempt = attempts,
                    max_retries = MAX_RETRIES,
                    delay_ms = delay.as_millis(),
                    "Database transient error, retrying write operation"
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Execute a query operation with exponential backoff retry for transient errors
///
/// Generic over the return type T to support various query result types.
/// Essential for handling I/O errors during read operations in cloud sync scenarios.
pub async fn query_with_retry<F, Fut, T>(operation: F) -> std::result::Result<T, sqlx::Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = std::result::Result<T, sqlx::Error>>,
{
    let mut attempts = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if is_transient_error(&e) && attempts < MAX_RETRIES => {
                attempts += 1;
                let delay = backoff_delay(attempts);
                tracing::debug!(
                    error = %e,
                    attempt = attempts,
                    max_retries = MAX_RETRIES,
                    delay_ms = delay.as_millis(),
                    "Database transient error, retrying query operation"
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_delay() {
        assert_eq!(backoff_delay(1), Duration::from_millis(200));
        assert_eq!(backoff_delay(2), Duration::from_millis(400));
        assert_eq!(backoff_delay(3), Duration::from_millis(800));
        assert_eq!(backoff_delay(4), Duration::from_millis(1600));
        assert_eq!(backoff_delay(5), Duration::from_millis(3200));
    }
}
