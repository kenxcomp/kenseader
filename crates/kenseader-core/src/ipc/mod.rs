//! IPC module for daemon-client communication
//!
//! This module provides Unix socket based IPC for separating the TUI frontend
//! from the backend daemon service.

mod client;
mod protocol;
mod server;

pub use client::{is_daemon_running, DaemonClient};
pub use protocol::*;
pub use server::DaemonServer;
