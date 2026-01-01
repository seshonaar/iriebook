import React, { createContext, useContext, useReducer, ReactNode } from "react";
import type {
  SessionData,
  BookInfo,
  GitSyncStatus,
  GitCommit,
  DeviceFlowInfo,
  DiffComparison,
} from "../bindings";
import type { LogEntryType } from "../bindings/LogEntryType";
import type { ProcessingProgress } from "../bindings/ProcessingProgress";
import type { GitAuthStatus } from "../bindings/GitAuthStatus";
import { sortBooks } from "../lib/utils";
import type { AppAction } from "./actions";

export type { AppAction } from "./actions";

// Log entry for results panel
export interface LogEntry {
  message: string;
  type: LogEntryType;
  timestamp: Date;
  outputPath?: string;
}

// Diff tab for viewing commit changes
export interface DiffTab {
  id: string; // unique ID (e.g., `${commitHash}-${filePath}`)
  type: "diff";
  title: string; // e.g., "manuscript.md @ abc1234"
  commitHash: string;
  filePath: string;
  diffData: DiffComparison; // Always loaded (comes from backend)
}

// State interface
export interface AppState {
  // Session & Books
  session: SessionData | null;
  books: BookInfo[];
  selectedFolder: string | null;

  // Current Book Mode (default ON) - when enabled, actions apply to viewed book only
  currentBookMode: boolean;

  // Processing state (Phase 2)
  isProcessing: boolean;
  processingMode: "publish" | "analyze" | null;
  processingProgress: ProcessingProgress | null;
  resultsLog: LogEntry[];

  // UI state
  loading: boolean;
  error: string | null;

  // Book viewer (Phase 3)
  viewedBookIndex: number | null;
  
  // Git state (Phase 4)
  gitSyncStatus: GitSyncStatus;
  gitAuthStatus: GitAuthStatus;
  commitHistory: GitCommit[];
  gitOperationInProgress: boolean;
  gitOperationMessage: string | null;
  deviceFlowInfo: DeviceFlowInfo | null;

  // Diff tabs (Phase 5)
  openDiffTabs: DiffTab[];
  activeDiffTabId: string | null;
  activeTab: string; // "books", "history", "analysis", or a diff tab ID
  lastActiveStaticTab: string; // "books", "history", "analysis" - to restore when closing diff tabs
}

// Initial state
const initialState: AppState = {
  session: null,
  books: [],
  selectedFolder: null,
  currentBookMode: true, // Default ON - actions apply to currently viewed book
  isProcessing: false,
  processingMode: null,
  processingProgress: null,
  resultsLog: [],
  loading: false,
  error: null,
  viewedBookIndex: null,
  gitSyncStatus: { status: "Uninitialized" },
  gitAuthStatus: { status: "NotAuthenticated" },
  commitHistory: [],
  gitOperationInProgress: false,
  gitOperationMessage: null,
  deviceFlowInfo: null,
  openDiffTabs: [],
  activeDiffTabId: null,
  activeTab: "books",
  lastActiveStaticTab: "books",
};

// Reducer
function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "SET_SESSION":
      return {
        ...state,
        session: action.payload,
        selectedFolder: action.payload?.folder_path || null,
      };

    case "SET_BOOKS":
      return { ...state, books: sortBooks(action.payload) };

    case "SET_FOLDER":
      return { ...state, selectedFolder: action.payload };

    case "TOGGLE_BOOK": {
      const newBooks = [...state.books];
      if (newBooks[action.payload]) {
        newBooks[action.payload] = {
          ...newBooks[action.payload],
          selected: !newBooks[action.payload].selected,
        };
      }
      return { ...state, books: newBooks };
    }

    case "TOGGLE_ALL_BOOKS": {
      const newBooks = state.books.map((book) => ({
        ...book,
        selected: action.payload,
      }));
      return { ...state, books: newBooks };
    }

    case "SET_CURRENT_BOOK_MODE":
      return { ...state, currentBookMode: action.payload };

    case "SET_LOADING":
      return { ...state, loading: action.payload };

    case "SET_ERROR":
      return { ...state, error: action.payload };

    case "SET_VIEWED_BOOK":
      return { ...state, viewedBookIndex: action.payload };

    case "SET_PROCESSING":
      return { ...state, isProcessing: action.payload };

    case "SET_PROCESSING_MODE":
      return { ...state, processingMode: action.payload };

    case "SET_PROCESSING_PROGRESS":
      return { ...state, processingProgress: action.payload };

    case "CLEAR_PROCESSING_PROGRESS":
      return { ...state, processingProgress: null };

    case "ADD_LOG_ENTRY":
      return {
        ...state,
        resultsLog: [
          ...state.resultsLog,
          { ...action.payload, timestamp: new Date() },
        ],
      };

    case "CLEAR_LOG":
      return { ...state, resultsLog: [] };

    // Git operations
    case "SET_GIT_SYNC_STATUS":
      return { ...state, gitSyncStatus: action.payload };

    case "SET_GIT_AUTH_STATUS":
      return { ...state, gitAuthStatus: action.payload };

    case "SET_COMMIT_HISTORY":
      return { ...state, commitHistory: action.payload };

    case "GIT_OPERATION_STARTED":
      return {
        ...state,
        gitOperationInProgress: true,
        gitOperationMessage: null,
      };

    case "GIT_OPERATION_COMPLETED":
      return {
        ...state,
        gitOperationInProgress: false,
        gitOperationMessage: action.payload.message,
      };

    case "DEVICE_FLOW_STARTED":
      return { ...state, deviceFlowInfo: action.payload };

    case "DEVICE_FLOW_COMPLETED":
      return {
        ...state,
        deviceFlowInfo: null,
        gitAuthStatus: action.payload.success
          ? { status: "Authenticated" }
          : { status: "NotAuthenticated" },
      };

    case "OPEN_DIFF_TAB": {
      const { commitHash, filePath, title, diffData } = action.payload;
      const tabId = `${commitHash}-${filePath}`;

      // Check if tab already exists
      if (state.openDiffTabs.some(tab => tab.id === tabId)) {
        // Just activate existing tab
        return { ...state, activeDiffTabId: tabId, activeTab: tabId };
      }

      // Create new tab
      const newTab: DiffTab = {
        id: tabId,
        type: "diff",
        title,
        commitHash,
        filePath,
        diffData,
      };

      return {
        ...state,
        openDiffTabs: [...state.openDiffTabs, newTab],
        activeDiffTabId: tabId,
        activeTab: tabId,
      };
    }

    case "CLOSE_DIFF_TAB": {
      const tabId = action.payload;
      const newTabs = state.openDiffTabs.filter(tab => tab.id !== tabId);

      // If closing the active tab, switch to another tab
      let newActiveId = state.activeDiffTabId;
      let newActiveTab = state.activeTab;
      if (state.activeDiffTabId === tabId) {
        newActiveId = newTabs.length > 0 ? newTabs[newTabs.length - 1].id : null;
        newActiveTab = newActiveId || state.lastActiveStaticTab;
      }

      return {
        ...state,
        openDiffTabs: newTabs,
        activeDiffTabId: newActiveId,
        activeTab: newActiveTab,
      };
    }

    case "SET_ACTIVE_TAB": {
      const isStatic = ["books", "history", "analysis"].includes(action.payload);
      return {
        ...state,
        activeTab: action.payload,
        lastActiveStaticTab: isStatic ? action.payload : state.lastActiveStaticTab
      };
    }

    default:
      return state;
  }
}

// Context
interface AppContextValue {
  state: AppState;
  dispatch: React.Dispatch<AppAction>;
}

const AppContext = createContext<AppContextValue | undefined>(undefined);

// Provider
export function AppProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(appReducer, initialState);

  return (
    <AppContext.Provider value={{ state, dispatch }}>
      {children}
    </AppContext.Provider>
  );
}

// Hook to use the context
export function useAppContext() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error("useAppContext must be used within AppProvider");
  }
  return context;
}
