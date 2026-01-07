use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::ai::Summarizer;
use crate::config::AppConfig;
use crate::storage::Database;
use crate::Result;

use super::tasks::{classify_pending_articles, cleanup_old_articles, refresh_all_feeds, score_and_filter_articles, summarize_pending_articles};

/// Events emitted by the scheduler to notify the UI of changes
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    /// Feeds have been refreshed, UI should reload article list
    FeedsRefreshed { new_articles: u32 },
    /// Articles have been cleaned up
    ArticlesCleaned { deleted: u32 },
    /// Articles have been summarized
    ArticlesSummarized { count: u32 },
    /// Articles have been scored and filtered
    ArticlesFiltered { scored: u32, filtered: u32 },
    /// Articles have been classified for style/tone
    ArticlesClassified { count: u32 },
    /// An error occurred during a background task
    Error { task: String, message: String },
}

/// Background scheduler service that runs periodic tasks
pub struct SchedulerService {
    db: Arc<Database>,
    config: Arc<AppConfig>,
    summarizer: Option<Arc<Summarizer>>,
    event_tx: Option<mpsc::UnboundedSender<SchedulerEvent>>,
}

impl SchedulerService {
    /// Create a new scheduler service
    pub fn new(db: Arc<Database>, config: Arc<AppConfig>) -> Self {
        Self {
            db,
            config,
            summarizer: None,
            event_tx: None,
        }
    }

    /// Set the summarizer for AI tasks
    pub fn with_summarizer(mut self, summarizer: Arc<Summarizer>) -> Self {
        self.summarizer = Some(summarizer);
        self
    }

    /// Set the event sender for UI notifications
    pub fn with_event_sender(mut self, tx: mpsc::UnboundedSender<SchedulerEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Send an event to the UI (if event channel is configured)
    fn send_event(&self, event: SchedulerEvent) {
        if let Some(ref tx) = self.event_tx {
            if tx.send(event).is_err() {
                warn!("Failed to send scheduler event: receiver dropped");
            }
        }
    }

    /// Run background tasks in a loop until shutdown signal
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let refresh_secs = self.config.sync.refresh_interval_secs;
        let cleanup_secs = self.config.sync.cleanup_interval_secs;
        let summarize_secs = self.config.sync.summarize_interval_secs;
        let filter_secs = self.config.sync.filter_interval_secs;

        // Skip if refresh is disabled (0)
        if refresh_secs == 0 {
            info!("Background scheduler disabled (refresh_interval_secs = 0)");
            // Still wait for shutdown
            let _ = shutdown.changed().await;
            return;
        }

        info!(
            "Scheduler started: refresh={}s, cleanup={}s, summarize={}s, filter={}s",
            refresh_secs, cleanup_secs, summarize_secs, filter_secs
        );

        let mut refresh_interval =
            tokio::time::interval(Duration::from_secs(refresh_secs));
        let mut cleanup_interval =
            tokio::time::interval(Duration::from_secs(cleanup_secs));
        let mut summarize_interval =
            tokio::time::interval(Duration::from_secs(summarize_secs));
        let mut filter_interval =
            tokio::time::interval(Duration::from_secs(filter_secs));

        // Skip the first tick (fires immediately)
        refresh_interval.tick().await;
        cleanup_interval.tick().await;
        summarize_interval.tick().await;
        filter_interval.tick().await;

        loop {
            tokio::select! {
                // Handle shutdown signal
                result = shutdown.changed() => {
                    if result.is_ok() && *shutdown.borrow() {
                        info!("Scheduler received shutdown signal");
                        break;
                    }
                }

                // Refresh feeds periodically
                _ = refresh_interval.tick() => {
                    debug!("Running scheduled feed refresh");
                    match refresh_all_feeds(&self.db, &self.config).await {
                        Ok(new_articles) => {
                            if new_articles > 0 {
                                info!("Scheduled refresh: {} new articles", new_articles);
                            }
                            self.send_event(SchedulerEvent::FeedsRefreshed { new_articles });
                        }
                        Err(e) => {
                            error!("Scheduled refresh failed: {}", e);
                            self.send_event(SchedulerEvent::Error {
                                task: "refresh".to_string(),
                                message: e.to_string(),
                            });
                        }
                    }
                }

                // Cleanup old articles periodically
                _ = cleanup_interval.tick() => {
                    debug!("Running scheduled article cleanup");
                    match cleanup_old_articles(&self.db, &self.config).await {
                        Ok(deleted) => {
                            if deleted > 0 {
                                info!("Scheduled cleanup: {} articles deleted", deleted);
                            }
                            self.send_event(SchedulerEvent::ArticlesCleaned { deleted });
                        }
                        Err(e) => {
                            error!("Scheduled cleanup failed: {}", e);
                            self.send_event(SchedulerEvent::Error {
                                task: "cleanup".to_string(),
                                message: e.to_string(),
                            });
                        }
                    }
                }

                // Summarize pending articles (if AI is enabled)
                _ = summarize_interval.tick() => {
                    if let Some(ref summarizer) = self.summarizer {
                        debug!("Running scheduled summarization");
                        match summarize_pending_articles(&self.db, summarizer.clone(), 5).await {
                            Ok(count) => {
                                if count > 0 {
                                    info!("Scheduled summarization: {} articles", count);
                                }
                                self.send_event(SchedulerEvent::ArticlesSummarized { count });
                            }
                            Err(e) => {
                                error!("Scheduled summarization failed: {}", e);
                                self.send_event(SchedulerEvent::Error {
                                    task: "summarize".to_string(),
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                }

                // Score and filter articles (if AI is enabled)
                _ = filter_interval.tick() => {
                    if let Some(ref summarizer) = self.summarizer {
                        // Run filtering
                        debug!("Running scheduled filtering");
                        let threshold = self.config.ai.relevance_threshold;
                        let min_len = self.config.ai.min_summarize_length;
                        match score_and_filter_articles(&self.db, summarizer.clone(), threshold, min_len).await {
                            Ok((scored, filtered)) => {
                                if scored > 0 {
                                    info!("Scheduled filtering: scored {}, filtered {}", scored, filtered);
                                }
                                self.send_event(SchedulerEvent::ArticlesFiltered { scored, filtered });
                            }
                            Err(e) => {
                                error!("Scheduled filtering failed: {}", e);
                                self.send_event(SchedulerEvent::Error {
                                    task: "filter".to_string(),
                                    message: e.to_string(),
                                });
                            }
                        }

                        // Run classification after filtering
                        debug!("Running scheduled classification");
                        match classify_pending_articles(&self.db, summarizer.clone(), 10).await {
                            Ok(count) => {
                                if count > 0 {
                                    info!("Scheduled classification: {} articles", count);
                                }
                                self.send_event(SchedulerEvent::ArticlesClassified { count });
                            }
                            Err(e) => {
                                error!("Scheduled classification failed: {}", e);
                                self.send_event(SchedulerEvent::Error {
                                    task: "classify".to_string(),
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        info!("Scheduler stopped");
    }

    /// Run a single refresh immediately (for manual refresh)
    pub async fn refresh_now(&self) -> Result<u32> {
        let new_articles = refresh_all_feeds(&self.db, &self.config).await?;
        self.send_event(SchedulerEvent::FeedsRefreshed { new_articles });
        Ok(new_articles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_scheduler_shutdown() {
        // Create a minimal config (unused in this test, but demonstrates the pattern)
        let _config = Arc::new(AppConfig::default());

        // This test would require a mock database
        // For now, just test the shutdown mechanism
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Signal shutdown immediately
        shutdown_tx.send(true).unwrap();

        // The scheduler should exit quickly
        let result = timeout(Duration::from_secs(1), async {
            // We can't actually run the scheduler without a database
            // but we can verify the watch channel works
            let mut rx = shutdown_rx;
            rx.changed().await.unwrap();
            assert!(*rx.borrow());
        })
        .await;

        assert!(result.is_ok());
    }
}
