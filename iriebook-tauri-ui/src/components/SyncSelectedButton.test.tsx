import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as React from "react";
import { SyncSelectedButton } from "./SyncSelectedButton";
import { AppProvider, useAppContext } from "../contexts/AppContext";
import { setBooks, setCurrentBookMode } from "../contexts/actions";
import { commands } from "../bindings";
import type { BookInfo } from "../bindings";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const mockedCommands = vi.mocked(commands);

function createLinkedBook(name: string): BookInfo {
  return {
    path: `/books/${name}.md` as any,
    display_name: name,
    selected: true,
    cover_image_path: null,
    metadata: null,
    google_docs_sync_info: {
      "google-doc-id": `doc-${name}`,
      "sync-status": "synced",
    },
    git_changed_files: [],
  };
}

// Wrapper that seeds the AppContext with books in multi-select mode
function TestHarness({ books }: { books: BookInfo[] }) {
  const { dispatch } = useAppContext();
  React.useEffect(() => {
    dispatch(setCurrentBookMode(false));
    dispatch(setBooks(books));
  }, [books, dispatch]);
  return null;
}

function renderSyncSelected(books: BookInfo[]) {
  const onBookUpdated = vi.fn();
  render(
    <AppProvider>
      <TestHarness books={books} />
      <SyncSelectedButton books={books} onBookUpdated={onBookUpdated} />
    </AppProvider>
  );
  return { onBookUpdated };
}

describe("SyncSelectedButton", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    vi.useRealTimers();
    cleanup();
  });

  describe("batch sync auth check", () => {
    it("triggers auth wizard instead of batch sync when not authenticated", async () => {
      // Reproduce the bug: token lost after system change, books still linked.
      // googleCheckAuth returns false => must show the auth wizard, not fail silently.
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      // Simulate the user not completing auth yet (cancelled) so the wizard
      // gates the sync: auth must be attempted before any sync runs.
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "error",
        error: "Authentication cancelled",
      });

      const linkedBooks = [createLinkedBook("Alpha"), createLinkedBook("Beta")];
      renderSyncSelected(linkedBooks);

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

      // Wait for books to be seeded into context, then click the sync button
      const syncButton = await screen.findByTestId("sync-selected-button");
      await user.click(syncButton);

      // Auth wizard dialog should appear
      await waitFor(() => {
        expect(screen.getByTestId("google-auth-dialog")).toBeInTheDocument();
      });

      // The auth flow must have been triggered (the bug never called it)
      await waitFor(() => {
        expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
      });

      // ...and the batch sync must NOT run until auth completes
      expect(mockedCommands.googleSyncSelected).not.toHaveBeenCalled();
    });

    it("proceeds with batch sync when already authenticated", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });
      mockedCommands.googleSyncSelected.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      const linkedBooks = [createLinkedBook("Alpha"), createLinkedBook("Beta")];
      renderSyncSelected(linkedBooks);

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

      const syncButton = await screen.findByTestId("sync-selected-button");
      await user.click(syncButton);

      await waitFor(() => {
        expect(mockedCommands.googleSyncSelected).toHaveBeenCalledTimes(1);
      });

      // Auth wizard must not appear when already authenticated
      expect(screen.queryByTestId("google-auth-dialog")).not.toBeInTheDocument();
      expect(mockedCommands.googleAuthStart).not.toHaveBeenCalled();
    });

    it("triggers auth wizard when auth check throws", async () => {
      // Defensive: if the auth check itself errors, fall back to the wizard.
      mockedCommands.googleCheckAuth.mockRejectedValueOnce(new Error("network"));
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "error",
        error: "Authentication cancelled",
      });

      const linkedBooks = [createLinkedBook("Alpha")];
      renderSyncSelected(linkedBooks);

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

      const syncButton = await screen.findByTestId("sync-selected-button");
      await user.click(syncButton);

      await waitFor(() => {
        expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
      });
      expect(mockedCommands.googleSyncSelected).not.toHaveBeenCalled();
    });

    it("resumes batch sync after auth completes successfully", async () => {
      // The full happy path: token expired -> wizard -> user authenticates -> sync runs.
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });
      mockedCommands.googleSyncSelected.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      const linkedBooks = [createLinkedBook("Alpha")];
      renderSyncSelected(linkedBooks);

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

      const syncButton = await screen.findByTestId("sync-selected-button");
      await user.click(syncButton);

      // Auth runs first, then sync resumes after success
      await waitFor(() => {
        expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
      });
      await waitFor(() => {
        expect(mockedCommands.googleSyncSelected).toHaveBeenCalledTimes(1);
      });
    });
  });

  // Keep the act() import meaningful for future async state assertions
  it("renders without crashing", async () => {
    mockedCommands.googleCheckAuth.mockResolvedValue({
      status: "ok",
      data: true,
    });
    renderSyncSelected([createLinkedBook("Solo")]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(await screen.findByTestId("sync-selected-button")).toBeInTheDocument();
  });
});
