use serde::{Deserialize, Serialize};
use specta::Type;
use crate::session::SessionData;
use crate::ui_state::BookInfo;
use crate::git_state::{GitSyncStatus, GitAuthStatus, DeviceFlowInfo, GitCommit};

/// Progress state for batch processing
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingProgress {
    #[specta(type = u32)]
    pub current_book_index: usize,
    pub current_book_name: String,
}

/// Type of log entry
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum LogEntryType {
    Info,
    Success,
    Error,
}

/// Log entry data without timestamp
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LogEntryPayload {
    pub message: String,
    #[serde(rename = "type")]
    pub type_: LogEntryType,
}

/// Application actions for state management (Redux/Reducer style)
///
/// These actions match the frontend reducer actions to ensure type safety
/// between backend events and frontend state updates.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "payload")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppAction {
    SetSession(Option<SessionData>),
    SetBooks(Vec<BookInfo>),
    SetFolder(Option<String>),
    SetPublishEnabled(bool),
    SetWordStatsEnabled(bool),
    ToggleBook(usize),
    ToggleAllBooks(bool),
    SetLoading(bool),
    SetError(Option<String>),
    SetViewedBook(Option<usize>),
    SetProcessing(bool),
    SetProcessingProgress(ProcessingProgress),
    ClearProcessingProgress,
    AddLogEntry(LogEntryPayload),
    ClearLog,

    // Git operations
    SetGitSyncStatus(GitSyncStatus),
    SetGitAuthStatus(GitAuthStatus),
    SetCommitHistory(Vec<GitCommit>),
    GitOperationStarted,
    GitOperationCompleted { success: bool, message: String },

    // Device flow
    DeviceFlowStarted(DeviceFlowInfo),
    DeviceFlowCompleted { success: bool },
}
