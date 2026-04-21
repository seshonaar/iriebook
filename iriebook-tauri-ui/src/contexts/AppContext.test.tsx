import { describe, it, expect } from "vitest";
import { appReducer, initialState, type AppState } from "./AppContext";
import {
  setSession,
  setBooks,
  toggleBook,
  toggleAllBooks,
  setProcessing,
  setProcessingProgress,
  openDiffTab,
  closeDiffTab,
  addLogEntry,
  deviceFlowStarted,
  deviceFlowCompleted,
  setGitAuthStatus,
  gitOperationStarted,
  gitOperationCompleted,
  setActiveTab,
} from "./actions";
import type { BookInfo, DiffComparison, SessionData } from "../bindings";

// Helper to create mock book info
function createMockBook(overrides: Partial<BookInfo> = {}): BookInfo {
  return {
    path: "/path/to/book" as any,
    display_name: "Test Book",
    selected: false,
    cover_image_path: null,
    metadata: null,
    google_docs_sync_info: null,
    git_changed_files: [],
    ...overrides,
  };
}

// Helper to create mock diff comparison
function createMockDiffComparison(): DiffComparison {
  return {
    left_display_name: "Original",
    right_display_name: "Modified",
    diff: {
      segments: [],
      stats: { added: 0, removed: 0, unchanged: 0 },
    },
  };
}

describe("appReducer", () => {
  describe("SET_SESSION", () => {
    it("should set session and selectedFolder from session", () => {
      const session: SessionData = {
        folder_path: "/test/folder" as any,
        selected_book_paths: [],
        current_book_mode: true,
        publication_options: initialState.publicationOptions,
      };

      const newState = appReducer(initialState, setSession(session));

      expect(newState.session).toBe(session);
      expect(newState.selectedFolder).toBe("/test/folder");
    });

    it("should clear selectedFolder when session is null", () => {
      const stateWithSession: AppState = {
        ...initialState,
        session: {
          folder_path: "/old/folder" as any,
          selected_book_paths: [],
          current_book_mode: true,
          publication_options: initialState.publicationOptions,
        },
        selectedFolder: "/old/folder",
      };

      const newState = appReducer(stateWithSession, setSession(null));

      expect(newState.session).toBeNull();
      expect(newState.selectedFolder).toBeNull();
    });
  });

  describe("SET_BOOKS", () => {
    it("should set books with sorting applied", () => {
      const books: BookInfo[] = [
        createMockBook({
          path: "/path/b" as any,
          display_name: "Book B",
          metadata: { title: "Book B", author: "Zeta Author" } as any,
        }),
        createMockBook({
          path: "/path/a" as any,
          display_name: "Book A",
          metadata: { title: "Book A", author: "Alpha Author" } as any,
        }),
      ];

      const newState = appReducer(initialState, setBooks(books));

      // Should be sorted by author (Alpha before Zeta)
      expect(newState.books).toHaveLength(2);
      expect(newState.books[0].metadata?.author).toBe("Alpha Author");
      expect(newState.books[1].metadata?.author).toBe("Zeta Author");
    });

    it("should handle empty books array", () => {
      const newState = appReducer(initialState, setBooks([]));
      expect(newState.books).toEqual([]);
    });
  });

  describe("TOGGLE_BOOK", () => {
    it("should toggle book selected state from false to true", () => {
      const stateWithBooks: AppState = {
        ...initialState,
        books: [createMockBook({ selected: false })],
      };

      const newState = appReducer(stateWithBooks, toggleBook(0));

      expect(newState.books[0].selected).toBe(true);
    });

    it("should toggle book selected state from true to false", () => {
      const stateWithBooks: AppState = {
        ...initialState,
        books: [createMockBook({ selected: true })],
      };

      const newState = appReducer(stateWithBooks, toggleBook(0));

      expect(newState.books[0].selected).toBe(false);
    });

    it("should not modify state when index is out of bounds", () => {
      const stateWithBooks: AppState = {
        ...initialState,
        books: [createMockBook()],
      };

      const newState = appReducer(stateWithBooks, toggleBook(5));

      expect(newState.books).toEqual(stateWithBooks.books);
    });

    it("should only toggle the specified book", () => {
      const stateWithBooks: AppState = {
        ...initialState,
        books: [
          createMockBook({ selected: false, path: "/a" as any }),
          createMockBook({ selected: false, path: "/b" as any }),
          createMockBook({ selected: false, path: "/c" as any }),
        ],
      };

      const newState = appReducer(stateWithBooks, toggleBook(1));

      expect(newState.books[0].selected).toBe(false);
      expect(newState.books[1].selected).toBe(true);
      expect(newState.books[2].selected).toBe(false);
    });
  });

  describe("TOGGLE_ALL_BOOKS", () => {
    it("should select all books when payload is true", () => {
      const stateWithBooks: AppState = {
        ...initialState,
        books: [
          createMockBook({ selected: false }),
          createMockBook({ selected: true }),
          createMockBook({ selected: false }),
        ],
      };

      const newState = appReducer(stateWithBooks, toggleAllBooks(true));

      expect(newState.books.every((b) => b.selected)).toBe(true);
    });

    it("should deselect all books when payload is false", () => {
      const stateWithBooks: AppState = {
        ...initialState,
        books: [
          createMockBook({ selected: true }),
          createMockBook({ selected: true }),
        ],
      };

      const newState = appReducer(stateWithBooks, toggleAllBooks(false));

      expect(newState.books.every((b) => !b.selected)).toBe(true);
    });
  });

  describe("SET_PROCESSING", () => {
    it("should set isProcessing to true", () => {
      const newState = appReducer(initialState, setProcessing(true));
      expect(newState.isProcessing).toBe(true);
    });

    it("should set isProcessing to false", () => {
      const processingState: AppState = { ...initialState, isProcessing: true };
      const newState = appReducer(processingState, setProcessing(false));
      expect(newState.isProcessing).toBe(false);
    });
  });

  describe("SET_PROCESSING_PROGRESS", () => {
    it("should set processing progress", () => {
      const progress = {
        currentBookIndex: 2,
        currentBookName: "Test Book",
      };

      const newState = appReducer(initialState, setProcessingProgress(progress));

      expect(newState.processingProgress).toEqual(progress);
    });
  });

  describe("OPEN_DIFF_TAB", () => {
    it("should create a new diff tab", () => {
      const diffData = createMockDiffComparison();
      const action = openDiffTab({
        commitHash: "abc123",
        filePath: "manuscript.md",
        title: "manuscript.md @ abc123",
        diffData,
      });

      const newState = appReducer(initialState, action);

      expect(newState.openDiffTabs).toHaveLength(1);
      expect(newState.openDiffTabs[0].id).toBe("abc123-manuscript.md");
      expect(newState.openDiffTabs[0].title).toBe("manuscript.md @ abc123");
      expect(newState.activeDiffTabId).toBe("abc123-manuscript.md");
      expect(newState.activeTab).toBe("abc123-manuscript.md");
    });

    it("should activate existing tab instead of creating duplicate", () => {
      const diffData = createMockDiffComparison();
      const stateWithTab: AppState = {
        ...initialState,
        openDiffTabs: [
          {
            id: "abc123-manuscript.md",
            type: "diff",
            title: "manuscript.md @ abc123",
            commitHash: "abc123",
            filePath: "manuscript.md",
            diffData,
          },
        ],
        activeDiffTabId: null,
        activeTab: "books",
      };

      const action = openDiffTab({
        commitHash: "abc123",
        filePath: "manuscript.md",
        title: "manuscript.md @ abc123",
        diffData,
      });

      const newState = appReducer(stateWithTab, action);

      // Should not create duplicate
      expect(newState.openDiffTabs).toHaveLength(1);
      // Should activate existing tab
      expect(newState.activeDiffTabId).toBe("abc123-manuscript.md");
      expect(newState.activeTab).toBe("abc123-manuscript.md");
    });
  });

  describe("CLOSE_DIFF_TAB", () => {
    it("should close the specified tab", () => {
      const diffData = createMockDiffComparison();
      const stateWithTabs: AppState = {
        ...initialState,
        openDiffTabs: [
          {
            id: "tab1",
            type: "diff",
            title: "Tab 1",
            commitHash: "abc",
            filePath: "file1.md",
            diffData,
          },
          {
            id: "tab2",
            type: "diff",
            title: "Tab 2",
            commitHash: "def",
            filePath: "file2.md",
            diffData,
          },
        ],
        activeDiffTabId: "tab2",
        activeTab: "tab2",
        lastActiveStaticTab: "books",
      };

      const newState = appReducer(stateWithTabs, closeDiffTab("tab1"));

      expect(newState.openDiffTabs).toHaveLength(1);
      expect(newState.openDiffTabs[0].id).toBe("tab2");
    });

    it("should switch to last active static tab when closing the active tab and no tabs remain", () => {
      const diffData = createMockDiffComparison();
      const stateWithTab: AppState = {
        ...initialState,
        openDiffTabs: [
          {
            id: "tab1",
            type: "diff",
            title: "Tab 1",
            commitHash: "abc",
            filePath: "file1.md",
            diffData,
          },
        ],
        activeDiffTabId: "tab1",
        activeTab: "tab1",
        lastActiveStaticTab: "history",
      };

      const newState = appReducer(stateWithTab, closeDiffTab("tab1"));

      expect(newState.openDiffTabs).toHaveLength(0);
      expect(newState.activeDiffTabId).toBeNull();
      expect(newState.activeTab).toBe("history");
    });

    it("should switch to previous diff tab when closing active tab and other tabs exist", () => {
      const diffData = createMockDiffComparison();
      const stateWithTabs: AppState = {
        ...initialState,
        openDiffTabs: [
          {
            id: "tab1",
            type: "diff",
            title: "Tab 1",
            commitHash: "abc",
            filePath: "file1.md",
            diffData,
          },
          {
            id: "tab2",
            type: "diff",
            title: "Tab 2",
            commitHash: "def",
            filePath: "file2.md",
            diffData,
          },
        ],
        activeDiffTabId: "tab2",
        activeTab: "tab2",
        lastActiveStaticTab: "books",
      };

      const newState = appReducer(stateWithTabs, closeDiffTab("tab2"));

      expect(newState.openDiffTabs).toHaveLength(1);
      expect(newState.activeDiffTabId).toBe("tab1");
      expect(newState.activeTab).toBe("tab1");
    });
  });

  describe("ADD_LOG_ENTRY", () => {
    it("should add log entry with timestamp", () => {
      const entry = {
        message: "Test message",
        type: "info" as const,
        outputPath: "/path/to/output",
      };

      const newState = appReducer(initialState, addLogEntry(entry));

      expect(newState.resultsLog).toHaveLength(1);
      expect(newState.resultsLog[0].message).toBe("Test message");
      expect(newState.resultsLog[0].type).toBe("info");
      expect(newState.resultsLog[0].outputPath).toBe("/path/to/output");
      expect(newState.resultsLog[0].timestamp).toBeInstanceOf(Date);
    });

    it("should append to existing log entries", () => {
      const stateWithLog: AppState = {
        ...initialState,
        resultsLog: [
          { message: "First", type: "info", timestamp: new Date() },
        ],
      };

      const newState = appReducer(
        stateWithLog,
        addLogEntry({ message: "Second", type: "success" })
      );

      expect(newState.resultsLog).toHaveLength(2);
      expect(newState.resultsLog[1].message).toBe("Second");
    });
  });

  describe("DEVICE_FLOW_STARTED", () => {
    it("should set device flow info when starting GitHub device flow", () => {
      const deviceFlowInfo = {
        deviceCode: "abc123",
        userCode: "WDJB-MJHT",
        verificationUri: "https://github.com/login/device",
        expiresIn: 900,
      };

      const newState = appReducer(
        initialState,
        deviceFlowStarted(deviceFlowInfo)
      );

      expect(newState.deviceFlowInfo).toEqual(deviceFlowInfo);
    });

    it("should replace existing device flow info", () => {
      const stateWithExistingFlow: AppState = {
        ...initialState,
        deviceFlowInfo: {
          deviceCode: "old",
          userCode: "OLD-CODE",
          verificationUri: "https://github.com/login/device",
          expiresIn: 100,
        },
      };

      const newFlowInfo = {
        deviceCode: "new",
        userCode: "NEW-CODE",
        verificationUri: "https://github.com/login/device",
        expiresIn: 900,
      };

      const newState = appReducer(
        stateWithExistingFlow,
        deviceFlowStarted(newFlowInfo)
      );

      expect(newState.deviceFlowInfo).toEqual(newFlowInfo);
    });
  });

  describe("DEVICE_FLOW_COMPLETED", () => {
    it("should clear device flow info and set authenticated on success", () => {
      const stateWithDeviceFlow: AppState = {
        ...initialState,
        deviceFlowInfo: {
          deviceCode: "abc",
          userCode: "XYZ-123",
          verificationUri: "https://github.com/login/device",
          expiresIn: 900,
        },
        gitAuthStatus: { status: "NotAuthenticated" },
      };

      const newState = appReducer(
        stateWithDeviceFlow,
        deviceFlowCompleted({ success: true })
      );

      expect(newState.deviceFlowInfo).toBeNull();
      expect(newState.gitAuthStatus).toEqual({ status: "Authenticated" });
    });

    it("should clear device flow info and keep not authenticated on failure", () => {
      const stateWithDeviceFlow: AppState = {
        ...initialState,
        deviceFlowInfo: {
          deviceCode: "abc",
          userCode: "XYZ-123",
          verificationUri: "https://github.com/login/device",
          expiresIn: 900,
        },
        gitAuthStatus: { status: "NotAuthenticated" },
      };

      const newState = appReducer(
        stateWithDeviceFlow,
        deviceFlowCompleted({ success: false })
      );

      expect(newState.deviceFlowInfo).toBeNull();
      expect(newState.gitAuthStatus).toEqual({ status: "NotAuthenticated" });
    });
  });

  describe("SET_GIT_AUTH_STATUS", () => {
    it("should set git auth status to Authenticated", () => {
      const newState = appReducer(
        initialState,
        setGitAuthStatus({ status: "Authenticated" })
      );

      expect(newState.gitAuthStatus).toEqual({ status: "Authenticated" });
    });

    it("should set git auth status to NotAuthenticated", () => {
      const authenticatedState: AppState = {
        ...initialState,
        gitAuthStatus: { status: "Authenticated" },
      };

      const newState = appReducer(
        authenticatedState,
        setGitAuthStatus({ status: "NotAuthenticated" })
      );

      expect(newState.gitAuthStatus).toEqual({ status: "NotAuthenticated" });
    });

    it("should set git auth status to TokenExpired", () => {
      const authenticatedState: AppState = {
        ...initialState,
        gitAuthStatus: { status: "Authenticated" },
      };

      const newState = appReducer(
        authenticatedState,
        setGitAuthStatus({ status: "TokenExpired" })
      );

      expect(newState.gitAuthStatus).toEqual({ status: "TokenExpired" });
    });
  });

  describe("GIT_OPERATION_STARTED", () => {
    it("should set gitOperationInProgress to true", () => {
      const newState = appReducer(initialState, gitOperationStarted());

      expect(newState.gitOperationInProgress).toBe(true);
    });
  });

  describe("GIT_OPERATION_COMPLETED", () => {
    it("should set gitOperationInProgress to false on success", () => {
      const inProgressState: AppState = {
        ...initialState,
        gitOperationInProgress: true,
      };

      const newState = appReducer(
        inProgressState,
        gitOperationCompleted({ success: true, message: "Sync completed" })
      );

      expect(newState.gitOperationInProgress).toBe(false);
    });

    it("should set gitOperationInProgress to false on failure", () => {
      const inProgressState: AppState = {
        ...initialState,
        gitOperationInProgress: true,
      };

      const newState = appReducer(
        inProgressState,
        gitOperationCompleted({ success: false, message: "Sync failed" })
      );

      expect(newState.gitOperationInProgress).toBe(false);
    });
  });

  describe("SET_ACTIVE_TAB", () => {
    it("should update lastActiveStaticTab when switching to static tab", () => {
      const state: AppState = {
        ...initialState,
        activeTab: "books",
        lastActiveStaticTab: "books",
      };

      const newState = appReducer(state, setActiveTab("history"));

      expect(newState.activeTab).toBe("history");
      expect(newState.lastActiveStaticTab).toBe("history");
    });

    it("should not update lastActiveStaticTab when switching to diff tab", () => {
      const state: AppState = {
        ...initialState,
        activeTab: "books",
        lastActiveStaticTab: "books",
      };

      const newState = appReducer(state, setActiveTab("abc123-file.md"));

      expect(newState.activeTab).toBe("abc123-file.md");
      expect(newState.lastActiveStaticTab).toBe("books"); // unchanged
    });
  });
});
