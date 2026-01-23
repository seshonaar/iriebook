import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, cleanup } from "@testing-library/react";
import { useSession } from "./useSession";
import { AppProvider, useAppContext } from "../contexts/AppContext";
import { commands } from "../bindings";
import type { BookInfo, SessionData } from "../bindings";
import React from "react";

// Get the mocked commands
const mockedCommands = vi.mocked(commands);

// Wrapper component for hooks that need AppContext
function createWrapper() {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <AppProvider>{children}</AppProvider>;
  };
}

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

describe("useSession", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock saveSession to prevent the debounced effect from causing issues
    mockedCommands.saveSession.mockResolvedValue({
      status: "ok",
      data: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("should call loadSession on mount", async () => {
    // Return null session to avoid triggering book scan
    mockedCommands.loadSession.mockResolvedValueOnce({
      status: "ok",
      data: null,
    });

    const { unmount } = renderHook(() => useSession(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(mockedCommands.loadSession).toHaveBeenCalledTimes(1);
    });

    unmount();
  });

  it("should handle null session gracefully", async () => {
    mockedCommands.loadSession.mockResolvedValueOnce({
      status: "ok",
      data: null,
    });

    const { result, unmount } = renderHook(
      () => {
        useSession();
        return useAppContext();
      },
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(result.current.state.loading).toBe(false);
    });

    // State should remain in initial state
    expect(result.current.state.session).toBeNull();
    expect(result.current.state.selectedFolder).toBeNull();

    unmount();
  });

  it("should scan books when session has folder_path", async () => {
    const mockSession: SessionData = {
      folder_path: "/test/folder" as any,
      selected_book_paths: ["/test/folder/book1" as any],
      current_book_mode: false,
    };

    const mockBooks: BookInfo[] = [
      createMockBook({
        path: "/test/folder/book1" as any,
        display_name: "Book 1",
      }),
      createMockBook({
        path: "/test/folder/book2" as any,
        display_name: "Book 2",
      }),
    ];

    mockedCommands.loadSession.mockResolvedValueOnce({
      status: "ok",
      data: mockSession,
    });
    mockedCommands.scanBooks.mockResolvedValueOnce({
      status: "ok",
      data: mockBooks,
    });

    const { result, unmount } = renderHook(
      () => {
        useSession();
        return useAppContext();
      },
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(mockedCommands.scanBooks).toHaveBeenCalledWith("/test/folder");
    });

    await waitFor(() => {
      expect(result.current.state.books).toHaveLength(2);
    });

    unmount();
  });

  it("should restore book selection state from session", async () => {
    const mockSession: SessionData = {
      folder_path: "/test/folder" as any,
      selected_book_paths: ["/test/folder/book1" as any],
      current_book_mode: true,
    };

    const mockBooks: BookInfo[] = [
      createMockBook({
        path: "/test/folder/book1" as any,
        display_name: "Book 1",
      }),
      createMockBook({
        path: "/test/folder/book2" as any,
        display_name: "Book 2",
      }),
    ];

    mockedCommands.loadSession.mockResolvedValueOnce({
      status: "ok",
      data: mockSession,
    });
    mockedCommands.scanBooks.mockResolvedValueOnce({
      status: "ok",
      data: mockBooks,
    });

    const { result, unmount } = renderHook(
      () => {
        useSession();
        return useAppContext();
      },
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      const book1 = result.current.state.books.find(
        (b) => b.path === "/test/folder/book1"
      );
      expect(book1?.selected).toBe(true);
    });

    // Book2 should not be selected
    const book2 = result.current.state.books.find(
      (b) => b.path === "/test/folder/book2"
    );
    expect(book2?.selected).toBe(false);

    unmount();
  });

  it("should handle loadSession error gracefully", async () => {
    mockedCommands.loadSession.mockResolvedValueOnce({
      status: "error",
      error: "Failed to load session",
    });

    const { result, unmount } = renderHook(
      () => {
        useSession();
        return useAppContext();
      },
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(result.current.state.error).toContain(
        "errors.operations.loadSession"
      );
    });

    // Should set loading to false after error
    expect(result.current.state.loading).toBe(false);

    unmount();
  });
});
