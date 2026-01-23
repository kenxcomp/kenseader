//! IPC Server for daemon
//!
//! Listens on Unix socket and handles client requests.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{watch, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::protocol::*;
use crate::config::AppConfig;
use crate::feed::NewFeed;
use crate::profile::{BehaviorEventType, BehaviorTracker};
use crate::scheduler::tasks;
use crate::storage::{ArticleRepository, Database, FeedRepository};
use crate::Result;

/// Maximum number of concurrent IPC requests to prevent connection pool exhaustion
const MAX_CONCURRENT_REQUESTS: usize = 10;

/// IPC Server that handles client connections
pub struct DaemonServer {
    db: Arc<Database>,
    config: Arc<AppConfig>,
    socket_path: PathBuf,
    start_time: Instant,
    /// Semaphore to limit concurrent request processing
    request_semaphore: Arc<Semaphore>,
}

impl DaemonServer {
    pub fn new(db: Arc<Database>, config: Arc<AppConfig>) -> Self {
        let socket_path = config.socket_path();
        Self {
            db,
            config,
            socket_path,
            start_time: Instant::now(),
            request_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        }
    }

    /// Run the IPC server
    pub async fn run(&self, mut shutdown_rx: watch::Receiver<bool>) -> Result<()> {
        // Remove old socket file if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        // Ensure parent directory exists
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("IPC server listening on: {}", self.socket_path.display());

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let db = self.db.clone();
                            let config = self.config.clone();
                            let start_time = self.start_time;
                            let semaphore = self.request_semaphore.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, db, config, start_time, semaphore).await {
                                    warn!("Error handling connection: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("IPC server shutting down");
                        break;
                    }
                }
            }
        }

        // Cleanup socket file
        let _ = std::fs::remove_file(&self.socket_path);
        Ok(())
    }
}

async fn handle_connection(
    stream: UnixStream,
    db: Arc<Database>,
    config: Arc<AppConfig>,
    start_time: Instant,
    semaphore: Arc<Semaphore>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break; // Connection closed
        }

        // Acquire semaphore permit to limit concurrent request processing
        // This prevents connection pool exhaustion under high load
        let _permit = semaphore.acquire().await.map_err(|e| {
            crate::Error::Other(format!("Failed to acquire semaphore: {}", e))
        })?;

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => {
                debug!("Received request: {} (id: {})", request.method, request.id);
                handle_request(request, &db, &config, start_time).await
            }
            Err(e) => {
                warn!("Failed to parse request: {}", e);
                Response::error(Uuid::nil(), ERR_PARSE, format!("Parse error: {}", e))
            }
        };

        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

async fn handle_request(
    request: Request,
    db: &Database,
    config: &AppConfig,
    start_time: Instant,
) -> Response {
    let id = request.id;

    match request.method.as_str() {
        methods::PING => Response::success(id, serde_json::json!({"ok": true})),

        methods::STATUS => {
            let uptime = start_time.elapsed().as_secs();
            Response::success(
                id,
                serde_json::json!({
                    "running": true,
                    "uptime_secs": uptime,
                    "scheduler_running": true
                }),
            )
        }

        methods::ARTICLE_LIST => {
            match serde_json::from_value::<ArticleListParams>(request.params) {
                Ok(params) => {
                    let repo = ArticleRepository::new(db);
                    let result = if let Some(feed_id) = params.feed_id {
                        repo.list_by_feed(feed_id, params.unread_only).await
                    } else {
                        // List all unread articles
                        repo.list_unread(1000).await
                    };

                    match result {
                        Ok(articles) => Response::success(
                            id,
                            serde_json::json!({ "articles": articles }),
                        ),
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::ARTICLE_GET => {
            match serde_json::from_value::<ArticleIdParams>(request.params) {
                Ok(params) => {
                    let repo = ArticleRepository::new(db);
                    match repo.find_by_id(params.id).await {
                        Ok(article) => Response::success(
                            id,
                            serde_json::json!({ "article": article }),
                        ),
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::ARTICLE_MARK_READ => {
            match serde_json::from_value::<ArticleIdParams>(request.params) {
                Ok(params) => {
                    let repo = ArticleRepository::new(db);
                    // Get article to find feed_id for behavior tracking
                    let feed_id = match repo.find_by_id(params.id).await {
                        Ok(Some(article)) => Some(article.feed_id),
                        _ => None,
                    };
                    match repo.mark_read(params.id).await {
                        Ok(()) => {
                            // Record behavior event for user preference learning
                            if let Some(feed_id) = feed_id {
                                let tracker = BehaviorTracker::new(db);
                                if let Err(e) = tracker
                                    .record_event(
                                        Some(params.id),
                                        Some(feed_id),
                                        BehaviorEventType::Click,
                                        None,
                                        None,
                                    )
                                    .await
                                {
                                    debug!("Failed to record behavior event: {}", e);
                                }
                            }
                            Response::ok(id)
                        }
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::ARTICLE_MARK_UNREAD => {
            match serde_json::from_value::<ArticleIdParams>(request.params) {
                Ok(params) => {
                    let repo = ArticleRepository::new(db);
                    match repo.mark_unread(params.id).await {
                        Ok(()) => Response::ok(id),
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::ARTICLE_TOGGLE_SAVED => {
            match serde_json::from_value::<ArticleIdParams>(request.params) {
                Ok(params) => {
                    let repo = ArticleRepository::new(db);
                    // Get article to find feed_id for behavior tracking
                    let feed_id = match repo.find_by_id(params.id).await {
                        Ok(Some(article)) => Some(article.feed_id),
                        _ => None,
                    };
                    match repo.toggle_saved(params.id).await {
                        Ok(is_saved) => {
                            // Record save event if article was saved (not unsaved)
                            if is_saved {
                                if let Some(feed_id) = feed_id {
                                    let tracker = BehaviorTracker::new(db);
                                    if let Err(e) = tracker
                                        .record_event(
                                            Some(params.id),
                                            Some(feed_id),
                                            BehaviorEventType::Save,
                                            None,
                                            None,
                                        )
                                        .await
                                    {
                                        debug!("Failed to record save event: {}", e);
                                    }
                                }
                            }
                            Response::success(
                                id,
                                serde_json::json!({ "is_saved": is_saved }),
                            )
                        }
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::ARTICLE_SEARCH => {
            match serde_json::from_value::<ArticleSearchParams>(request.params) {
                Ok(params) => {
                    let repo = ArticleRepository::new(db);
                    match repo.search(&params.query, params.feed_id).await {
                        Ok(articles) => Response::success(
                            id,
                            serde_json::json!({ "articles": articles }),
                        ),
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::FEED_LIST => {
            let repo = FeedRepository::new(db);
            match repo.list_all().await {
                Ok(feeds) => Response::success(id, serde_json::json!({ "feeds": feeds })),
                Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
            }
        }

        methods::FEED_ADD => {
            match serde_json::from_value::<FeedAddParams>(request.params) {
                Ok(params) => {
                    let repo = FeedRepository::new(db);
                    let new_feed = NewFeed {
                        url: params.url,
                        local_name: params.name,
                    };
                    match repo.create(&new_feed).await {
                        Ok(feed) => Response::success(id, serde_json::json!({ "feed": feed })),
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::FEED_DELETE => {
            match serde_json::from_value::<ArticleIdParams>(request.params) {
                Ok(params) => {
                    let repo = FeedRepository::new(db);
                    match repo.delete(params.id).await {
                        Ok(deleted) => Response::success(
                            id,
                            serde_json::json!({ "deleted": deleted }),
                        ),
                        Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        methods::FEED_REFRESH => {
            match serde_json::from_value::<FeedIdParams>(request.params) {
                Ok(params) => {
                    if params.id.is_some() {
                        // Single feed refresh not implemented yet
                        Response::error(id, ERR_INTERNAL, "Single feed refresh not implemented")
                    } else {
                        // Refresh all feeds
                        match tasks::refresh_all_feeds(db, config).await {
                            Ok(count) => Response::success(
                                id,
                                serde_json::json!({ "new_articles": count }),
                            ),
                            Err(e) => Response::error(id, ERR_INTERNAL, e.to_string()),
                        }
                    }
                }
                Err(e) => Response::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }

        _ => Response::error(id, ERR_METHOD_NOT_FOUND, "Method not found"),
    }
}
