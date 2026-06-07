use crate::config::Config;
use crate::storage::{ConnectionInfo, DatabaseStats};
use crate::types::{Module, Task};

#[derive(Debug, Clone)]
pub enum Event {
    TaskUpdate(Task),
    ModuleUpdate(Module),
    StatusUpdate(crate::ui::app::AppStatus),
    ProgressUpdate(f32),
    LogMessage(String),
    CodeUpdate(String),
    FileUpdate(String),
    ReinitializeCoordinator(Config),
    // Storage events
    StorageConnected(ConnectionInfo),
    StorageDisconnected,
    StorageError(String),
    StorageTestResult(bool, Option<String>),
    DatabaseStatsUpdate(DatabaseStats),
    AdminQueryResult(Result<serde_json::Value, String>),
}
