use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::watch;
use tracing::{info, warn};

use kenseader_core::{
    ai::Summarizer,
    scheduler::SchedulerService,
    storage::Database,
    AppConfig,
};

/// Get the PID file path
fn pid_file_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(|| dirs::data_local_dir())
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("kenseader")
        .join("daemon.pid")
}

/// Check if daemon is running
fn is_daemon_running() -> Option<u32> {
    let pid_path = pid_file_path();
    if !pid_path.exists() {
        return None;
    }

    let mut file = fs::File::open(&pid_path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    let pid: u32 = contents.trim().parse().ok()?;

    // Check if process is still running
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .ok()?;
        if output.status.success() {
            return Some(pid);
        }
    }

    #[cfg(windows)]
    {
        // On Windows, just check if PID file exists (simplified)
        return Some(pid);
    }

    // Process not running, clean up stale PID file
    let _ = fs::remove_file(&pid_path);
    None
}

/// Write PID file
fn write_pid_file() -> Result<()> {
    let pid_path = pid_file_path();
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(&pid_path)?;
    writeln!(file, "{}", std::process::id())?;
    Ok(())
}

/// Remove PID file
fn remove_pid_file() {
    let _ = fs::remove_file(pid_file_path());
}

/// Start the daemon
pub async fn start(db: Arc<Database>, config: Arc<AppConfig>) -> Result<()> {
    // Check if already running
    if let Some(pid) = is_daemon_running() {
        println!("Daemon is already running (PID: {})", pid);
        return Ok(());
    }

    println!("Starting kenseader daemon...");

    // Write PID file
    write_pid_file()?;

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Setup signal handlers for graceful shutdown
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Received shutdown signal");
        let _ = shutdown_tx_clone.send(true);
    });

    // Create summarizer if AI is enabled
    let summarizer = if config.ai.enabled {
        match Summarizer::new(&config) {
            Ok(s) => {
                info!("AI summarization enabled (provider: {})", config.ai.provider);
                Some(Arc::new(s))
            }
            Err(e) => {
                warn!("Failed to initialize AI summarizer: {}", e);
                None
            }
        }
    } else {
        info!("AI summarization disabled");
        None
    };

    // Build scheduler service
    let scheduler = {
        let mut svc = SchedulerService::new(db.clone(), config.clone());
        if let Some(ref s) = summarizer {
            svc = svc.with_summarizer(s.clone());
        }
        svc
    };

    println!(
        "Daemon started (PID: {}). Press Ctrl+C or run 'kenseader daemon stop' to stop.",
        std::process::id()
    );
    println!("  Refresh interval: {} seconds", config.sync.refresh_interval_secs);
    println!("  Cleanup interval: {} seconds", config.sync.cleanup_interval_secs);
    println!("  Summarize interval: {} seconds", config.sync.summarize_interval_secs);

    // Run scheduler (blocks until shutdown)
    scheduler.run(shutdown_rx).await;

    // Cleanup
    remove_pid_file();
    println!("Daemon stopped.");

    Ok(())
}

/// Stop the daemon
pub async fn stop() -> Result<()> {
    match is_daemon_running() {
        Some(pid) => {
            println!("Stopping daemon (PID: {})...", pid);

            #[cfg(unix)]
            {
                use std::process::Command;
                let output = Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output()?;

                if output.status.success() {
                    // Wait a moment for graceful shutdown
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Check if still running
                    if is_daemon_running().is_none() {
                        println!("Daemon stopped successfully.");
                    } else {
                        // Force kill
                        let _ = Command::new("kill")
                            .arg("-9")
                            .arg(pid.to_string())
                            .output();
                        remove_pid_file();
                        println!("Daemon forcefully terminated.");
                    }
                } else {
                    println!("Failed to stop daemon. You may need to kill it manually: kill {}", pid);
                }
            }

            #[cfg(windows)]
            {
                println!("Please stop the daemon manually on Windows (PID: {})", pid);
            }
        }
        None => {
            println!("Daemon is not running.");
        }
    }

    Ok(())
}

/// Show daemon status
pub async fn status() -> Result<()> {
    match is_daemon_running() {
        Some(pid) => {
            println!("Daemon is running (PID: {})", pid);
            println!("PID file: {}", pid_file_path().display());
        }
        None => {
            println!("Daemon is not running.");
        }
    }

    Ok(())
}
