use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::watch;
use tracing::{info, warn};

use kenseader_core::{
    ai::Summarizer,
    ipc::DaemonServer,
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

/// Get the path to the file that stores the last used data_dir
fn last_data_dir_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("kenseader")
        .join("last_data_dir")
}

/// Read the last used data_dir from record file
fn read_last_data_dir() -> Option<PathBuf> {
    let path = last_data_dir_path();
    fs::read_to_string(&path)
        .ok()
        .map(|s| PathBuf::from(s.trim()))
}

/// Write the current data_dir to record file
fn write_last_data_dir(data_dir: &Path) -> Result<()> {
    let path = last_data_dir_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, data_dir.to_string_lossy().as_bytes())?;
    Ok(())
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Migrate data from old data_dir to new data_dir
/// Returns Ok(true) if migration was performed, Ok(false) if skipped due to existing data
fn migrate_data_dir(old_dir: &Path, new_dir: &Path) -> Result<bool> {
    // Files/directories to migrate
    let items_to_migrate = ["kenseader.db", "image_cache"];

    // Check if new path already has data - skip migration if so
    let db_in_new = new_dir.join("kenseader.db");
    if db_in_new.exists() {
        return Ok(false);
    }

    // Create new directory if needed
    fs::create_dir_all(new_dir)?;

    // Migrate each item
    for item in &items_to_migrate {
        let old_path = old_dir.join(item);
        let new_path = new_dir.join(item);

        if old_path.exists() {
            if old_path.is_dir() {
                // Copy directory recursively
                copy_dir_all(&old_path, &new_path)?;
            } else {
                // Copy file
                fs::copy(&old_path, &new_path)?;
            }
            info!("Migrated {} -> {}", old_path.display(), new_path.display());
        }
    }

    Ok(true)
}

/// Check and perform data migration if needed.
/// This must be called BEFORE Database::new() to ensure proper migration.
pub fn maybe_migrate_data(config: &AppConfig) -> Result<()> {
    let current_data_dir = config.data_dir();
    let default_data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kenseader");

    if let Some(last_data_dir) = read_last_data_dir() {
        // Case 1: We have a record of the last used data_dir
        if last_data_dir != current_data_dir {
            println!("Data directory changed:");
            println!("  Old: {}", last_data_dir.display());
            println!("  New: {}", current_data_dir.display());

            // Check if old directory has data to migrate
            let old_db = last_data_dir.join("kenseader.db");
            if old_db.exists() {
                println!("Migrating data from old directory...");
                if migrate_data_dir(&last_data_dir, &current_data_dir)? {
                    println!("Migration completed successfully.");
                } else {
                    println!("Using existing data at new location.");
                    println!(
                        "Note: Old data at {} was not migrated.",
                        last_data_dir.display()
                    );
                }
            }
        }
    } else {
        // Case 2: No record exists (first time using this feature)
        // Check if default data dir has data and config points elsewhere
        let default_db = default_data_dir.join("kenseader.db");
        if default_db.exists() && default_data_dir != current_data_dir {
            println!("First time migration detected:");
            println!("  Default data location: {}", default_data_dir.display());
            println!("  Configured location: {}", current_data_dir.display());
            println!("Migrating data from default directory...");
            if migrate_data_dir(&default_data_dir, &current_data_dir)? {
                println!("Migration completed successfully.");
            } else {
                println!("Using existing data at configured location.");
                println!(
                    "Note: Data at default location {} was not migrated.",
                    default_data_dir.display()
                );
            }
        }
    }

    // Record current data_dir for future comparison
    write_last_data_dir(&current_data_dir)?;

    Ok(())
}

/// Start the daemon
///
/// When `foreground` is true, the daemon runs in foreground mode (for launchd/systemd/brew services).
/// In foreground mode, PID file management is skipped since the service manager handles process lifecycle.
pub async fn start(db: Arc<Database>, config: Arc<AppConfig>, foreground: bool) -> Result<()> {
    if !foreground {
        // Check if already running (only in background mode)
        if let Some(pid) = is_daemon_running() {
            println!("Daemon is already running (PID: {})", pid);
            return Ok(());
        }
    }

    println!("Starting kenseader daemon{}...", if foreground { " (foreground mode)" } else { "" });

    // Write PID file (only in background mode)
    if !foreground {
        write_pid_file()?;
    }

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

    // Create IPC server
    let ipc_server = DaemonServer::new(db.clone(), config.clone());

    println!(
        "Daemon started (PID: {}). Press Ctrl+C or run 'kenseader daemon stop' to stop.",
        std::process::id()
    );
    println!("  Refresh interval: {} seconds", config.sync.refresh_interval_secs);
    println!("  Cleanup interval: {} seconds", config.sync.cleanup_interval_secs);
    println!("  Summarize interval: {} seconds", config.sync.summarize_interval_secs);
    println!("  IPC socket: {}", config.socket_path().display());

    // Run scheduler and IPC server in parallel
    let scheduler_shutdown_rx = shutdown_rx.clone();
    let ipc_shutdown_rx = shutdown_rx;

    tokio::select! {
        _ = scheduler.run(scheduler_shutdown_rx) => {
            info!("Scheduler stopped");
        }
        result = ipc_server.run(ipc_shutdown_rx) => {
            if let Err(e) = result {
                warn!("IPC server error: {}", e);
            }
        }
    }

    // Cleanup
    if !foreground {
        remove_pid_file();
    }
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
