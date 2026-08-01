import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { fireEvent, render, waitFor, cleanup, screen } from "@testing-library/react";
import { GitHistoryPanel } from "./GitHistoryPanel";
import { AppProvider, useAppContext } from "../contexts/AppContext";
import { setFolder, setGitSyncStatus } from "../contexts/actions";
import { commands } from "../bindings";
import React, { useEffect } from "react";

const mockedCommands = vi.mocked(commands);

function GitHistoryPanelHarness() {
  const { dispatch } = useAppContext();

  useEffect(() => {
    dispatch(setFolder("/tmp/book-repo"));
    dispatch(setGitSyncStatus({ status: "Clean" }));
  }, [dispatch]);

  return <GitHistoryPanel />;
}

describe("GitHistoryPanel", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.setSystemTime(new Date("2025-04-15T12:00:00"));
    vi.resetAllMocks();
    mockedCommands.gitGetLog.mockResolvedValue({ status: "ok", data: [] });
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("loads commit history from the last three months by default", async () => {
    render(
      <AppProvider>
        <GitHistoryPanelHarness />
      </AppProvider>
    );

    const expectedSinceTimestamp = Math.floor(
      new Date("2025-01-15T00:00:00").getTime() / 1000
    );

    await waitFor(() => {
      expect(mockedCommands.gitGetLog).toHaveBeenCalledWith(
        "/tmp/book-repo",
        1000,
        expectedSinceTimestamp
      );
    });

    expect(screen.getByLabelText("git.history.range.label")).toHaveValue("threeMonths");
  });

  it("loads all commit history when beginning is selected", async () => {
    render(
      <AppProvider>
        <GitHistoryPanelHarness />
      </AppProvider>
    );

    await waitFor(() => {
      expect(mockedCommands.gitGetLog).toHaveBeenCalled();
    });

    fireEvent.change(screen.getByLabelText("git.history.range.label"), {
      target: { value: "beginning" },
    });

    await waitFor(() => {
      expect(mockedCommands.gitGetLog).toHaveBeenLastCalledWith(
        "/tmp/book-repo",
        1000,
        null
      );
    });
  });
});
