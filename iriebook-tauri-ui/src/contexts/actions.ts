import type {
  SessionData,
  BookInfo,
  PublicationOptions,
  GitSyncStatus,
  GitCommit,
  DeviceFlowInfo,
  DiffComparison,
  CoverStatus,
} from "../bindings";
import type { LogEntryType } from "../bindings/LogEntryType";
import type { ProcessingProgress } from "../bindings/ProcessingProgress";
import type { GitAuthStatus } from "../bindings/GitAuthStatus";

// ============================================================================
// Session & Books
// ============================================================================

export const setSession = (session: SessionData | null) => ({
  type: "SET_SESSION" as const,
  payload: session,
});

export const setBooks = (books: BookInfo[]) => ({
  type: "SET_BOOKS" as const,
  payload: books,
});

export const setFolder = (folder: string | null) => ({
  type: "SET_FOLDER" as const,
  payload: folder,
});

export const setViewedBook = (index: number | null) => ({
  type: "SET_VIEWED_BOOK" as const,
  payload: index,
});

export const toggleBook = (index: number) => ({
  type: "TOGGLE_BOOK" as const,
  payload: index,
});

export const toggleAllBooks = (selected: boolean) => ({
  type: "TOGGLE_ALL_BOOKS" as const,
  payload: selected,
});

export const setCurrentBookMode = (enabled: boolean) => ({
  type: "SET_CURRENT_BOOK_MODE" as const,
  payload: enabled,
});

export const setPublicationOptions = (options: PublicationOptions) => ({
  type: "SET_PUBLICATION_OPTIONS" as const,
  payload: options,
});

// ============================================================================
// UI State
// ============================================================================

export const setLoading = (loading: boolean) => ({
  type: "SET_LOADING" as const,
  payload: loading,
});

export const setError = (error: string | null) => ({
  type: "SET_ERROR" as const,
  payload: error,
});

export const setActiveTab = (tab: string) => ({
  type: "SET_ACTIVE_TAB" as const,
  payload: tab,
});

// ============================================================================
// Processing
// ============================================================================

export const setProcessing = (processing: boolean) => ({
  type: "SET_PROCESSING" as const,
  payload: processing,
});

export const setProcessingMode = (mode: "publish" | "analyze" | null) => ({
  type: "SET_PROCESSING_MODE" as const,
  payload: mode,
});

export const setProcessingProgress = (progress: ProcessingProgress) => ({
  type: "SET_PROCESSING_PROGRESS" as const,
  payload: progress,
});

export const clearProcessingProgress = () => ({
  type: "CLEAR_PROCESSING_PROGRESS" as const,
});

export const addLogEntry = (entry: {
  message: string;
  type: LogEntryType;
  outputPath?: string;
}) => ({
  type: "ADD_LOG_ENTRY" as const,
  payload: entry,
});

export const clearLog = () => ({
  type: "CLEAR_LOG" as const,
});

// ============================================================================
// Git Operations
// ============================================================================

export const setGitSyncStatus = (status: GitSyncStatus) => ({
  type: "SET_GIT_SYNC_STATUS" as const,
  payload: status,
});

export const setGitAuthStatus = (status: GitAuthStatus) => ({
  type: "SET_GIT_AUTH_STATUS" as const,
  payload: status,
});

export const setCommitHistory = (commits: GitCommit[]) => ({
  type: "SET_COMMIT_HISTORY" as const,
  payload: commits,
});

export const gitOperationStarted = () => ({
  type: "GIT_OPERATION_STARTED" as const,
});

export const gitOperationCompleted = (result: {
  success: boolean;
  message: string;
}) => ({
  type: "GIT_OPERATION_COMPLETED" as const,
  payload: result,
});

export const deviceFlowStarted = (info: DeviceFlowInfo) => ({
  type: "DEVICE_FLOW_STARTED" as const,
  payload: info,
});

export const deviceFlowCompleted = (result: { success: boolean }) => ({
  type: "DEVICE_FLOW_COMPLETED" as const,
  payload: result,
});

// ============================================================================
// Diff Tabs
// ============================================================================

export const openDiffTab = (tab: {
  commitHash: string;
  filePath: string;
  title: string;
  diffData: DiffComparison;
}) => ({
  type: "OPEN_DIFF_TAB" as const,
  payload: tab,
});

export const closeDiffTab = (tabId: string) => ({
  type: "CLOSE_DIFF_TAB" as const,
  payload: tabId,
});

// ============================================================================
// Cover Loading
// ============================================================================

export const setCoverStatus = (path: string, status: CoverStatus) => ({
  type: "SET_COVER_STATUS" as const,
  payload: { path, status },
});

export const clearCoverStatus = () => ({
  type: "CLEAR_COVER_STATUS" as const,
});

// ============================================================================
// Union type derived from all action creators
// ============================================================================

export type AppAction =
  | ReturnType<typeof setSession>
  | ReturnType<typeof setBooks>
  | ReturnType<typeof setFolder>
  | ReturnType<typeof setViewedBook>
  | ReturnType<typeof toggleBook>
  | ReturnType<typeof toggleAllBooks>
  | ReturnType<typeof setCurrentBookMode>
  | ReturnType<typeof setPublicationOptions>
  | ReturnType<typeof setLoading>
  | ReturnType<typeof setError>
  | ReturnType<typeof setActiveTab>
  | ReturnType<typeof setProcessing>
  | ReturnType<typeof setProcessingMode>
  | ReturnType<typeof setProcessingProgress>
  | ReturnType<typeof clearProcessingProgress>
  | ReturnType<typeof addLogEntry>
  | ReturnType<typeof clearLog>
  | ReturnType<typeof setGitSyncStatus>
  | ReturnType<typeof setGitAuthStatus>
  | ReturnType<typeof setCommitHistory>
  | ReturnType<typeof gitOperationStarted>
  | ReturnType<typeof gitOperationCompleted>
  | ReturnType<typeof deviceFlowStarted>
  | ReturnType<typeof deviceFlowCompleted>
  | ReturnType<typeof openDiffTab>
  | ReturnType<typeof closeDiffTab>
  | ReturnType<typeof setCoverStatus>
  | ReturnType<typeof clearCoverStatus>;
