import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, cleanup } from "@testing-library/react";
import { useProcessingEvents } from "./useProcessingEvents";
import { useCoverImage } from "./useCoverImage";
import { getGlobalCoverCache, clearGlobalCoverCache } from "./coverCache";
import { AppProvider } from "../contexts/AppContext";
import { commands, events } from "../bindings";
import React from "react";

const mockedCommands = vi.mocked(commands);
const mockedEvents = vi.mocked(events);

function createWrapper() {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <AppProvider>{children}</AppProvider>;
  };
}

describe("useProcessingEvents - Cover Cache Invalidation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearGlobalCoverCache();
  });

  afterEach(() => {
    cleanup();
    clearGlobalCoverCache();
  });

  it("clears global cover cache when BookListChangedEvent is received", async () => {
    // Set up mock to capture event callback
    let eventCallback: any;
    const originalListen = mockedEvents.bookListChangedEvent.listen;
    mockedEvents.bookListChangedEvent.listen = vi.fn((callback: any) => {
      eventCallback = callback;
      return Promise.resolve(() => {});
    }) as any;

    // Mock scanBooks to avoid network calls
    mockedCommands.scanBooks.mockResolvedValueOnce({
      status: "ok",
      data: [],
    });

    const wrapper = createWrapper();

    // Render both hooks
    const { result: coverResult } = renderHook(() => useCoverImage(), {
      wrapper,
    });
    renderHook(() => useProcessingEvents(), {
      wrapper,
    });

    // Load a cover image (this caches it globally)
    const mockDataUrl = "data:image/png;base64,test_cover";
    mockedCommands.loadCoverImage.mockResolvedValueOnce({
      status: "ok",
      data: { data_url: mockDataUrl, width: 100, height: 150 },
    });

    await act(async () => {
      await coverResult.current.loadCover("/books/book1/cover.jpg");
    });

    // Verify cover is in global cache
    expect(getGlobalCoverCache("/books/book1/cover.jpg")).toBe(mockDataUrl);

    // Trigger BookListChangedEvent
    await act(async () => {
      if (eventCallback) {
        eventCallback({ payload: {} });
      }
    });

    // Assertion: Global cover cache should be cleared after BookListChangedEvent
    expect(getGlobalCoverCache("/books/book1/cover.jpg")).toBeNull();
  });
});
