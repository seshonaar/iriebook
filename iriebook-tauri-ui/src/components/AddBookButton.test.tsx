import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AddBookButton } from "./AddBookButton";
import { commands } from "../bindings";

const mockedCommands = vi.mocked(commands);

function renderAddBookButton() {
  render(<AddBookButton workspaceRoot="/books" onBookAdded={vi.fn()} />);
}

describe("AddBookButton", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    vi.useRealTimers();
    cleanup();
  });

  it("starts the Google auth wizard before opening the doc picker when not authenticated", async () => {
    mockedCommands.googleCheckAuth.mockResolvedValueOnce({
      status: "ok",
      data: false,
    });
    mockedCommands.googleAuthStart.mockResolvedValueOnce({
      status: "error",
      error: "Authentication cancelled",
    });

    renderAddBookButton();

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await user.click(screen.getByRole("button", { name: /books\.list\.addBookFrom/i }));
    await user.click(screen.getByText("books.list.addFromGoogleDocs"));

    await waitFor(() => {
      expect(screen.getByTestId("google-auth-dialog")).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
    });

    expect(mockedCommands.googleListDocs).not.toHaveBeenCalled();
  });

  it("opens the doc picker after successful authentication", async () => {
    mockedCommands.googleCheckAuth.mockResolvedValueOnce({
      status: "ok",
      data: false,
    });
    mockedCommands.googleAuthStart.mockResolvedValueOnce({
      status: "ok",
      data: null,
    });

    renderAddBookButton();

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await user.click(screen.getByRole("button", { name: /books\.list\.addBookFrom/i }));
    await user.click(screen.getByText("books.list.addFromGoogleDocs"));

    await waitFor(() => {
      expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(screen.getByTestId("google-doc-picker-dialog")).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(mockedCommands.googleListDocs).toHaveBeenCalledTimes(1);
    });
  });
});
