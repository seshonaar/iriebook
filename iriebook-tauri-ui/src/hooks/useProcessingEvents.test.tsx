import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, cleanup } from "@testing-library/react";
import { useProcessingEvents } from "./useProcessingEvents";
import { useAppContext } from "../contexts/AppContext";
import { setCoverStatus } from "../contexts/actions";
import { AppProvider } from "../contexts/AppContext";
import { commands, events } from "../bindings";
import React from "react";

const mockedCommands = vi.mocked(commands);
const mockedEvents = vi.mocked(events);

describe("useProcessingEvents - Cover Cache Invalidation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it("clears context cover status when BookListChangedEvent is received", async () => {
    // Set up mock to capture event callback
    let bookListChangedCallback: any;
    mockedEvents.bookListChangedEvent.listen = vi.fn((callback: any) => {
      bookListChangedCallback = callback;
      return Promise.resolve(() => {});
    }) as any;

    // Mock the cover reload event listener
    mockedEvents.coverReloadEvent = {
      listen: vi.fn(() => Promise.resolve(() => {})),
    } as any;

    // Mock scanBooks to avoid network calls
    mockedCommands.scanBooks.mockResolvedValueOnce({
      status: "ok",
      data: [],
    });

    // Combined hook that uses both context and events
    function useCombinedHook() {
      const context = useAppContext();
      useProcessingEvents();
      return context;
    }

    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <AppProvider>{children}</AppProvider>
    );

    const { result } = renderHook(() => useCombinedHook(), { wrapper });

    // Set a cover status in context
    act(() => {
      result.current.dispatch(
        setCoverStatus("/books/book1/cover.jpg", {
          type: "ready",
          data_url: "data:image/png;base64,test",
          width: 100,
          height: 150,
        })
      );
    });

    // Verify cover is in context
    expect(result.current.state.coverStatus["/books/book1/cover.jpg"]).toBeDefined();

    // Trigger BookListChangedEvent
    await act(async () => {
      if (bookListChangedCallback) {
        await bookListChangedCallback({ payload: {} });
      }
    });

    // Assertion: Cover status should be cleared after BookListChangedEvent
    expect(result.current.state.coverStatus["/books/book1/cover.jpg"]).toBeUndefined();
  });
});
