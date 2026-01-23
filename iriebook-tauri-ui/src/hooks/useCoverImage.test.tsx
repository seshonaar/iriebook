import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useCoverImage } from "./useCoverImage";
import { commands } from "../bindings";

// Get the mocked commands
const mockedCommands = vi.mocked(commands);

describe("useCoverImage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return null from getCachedCover when cover is not in cache", () => {
    const { result } = renderHook(() => useCoverImage());

    const cached = result.current.getCachedCover("/path/to/cover.png");

    expect(cached).toBeNull();
  });

  it("should load and cache cover image", async () => {
    const mockDataUrl = "data:image/png;base64,testdata123";
    mockedCommands.loadCoverImage.mockResolvedValueOnce({
      status: "ok",
      data: { data_url: mockDataUrl, width: 100, height: 150 },
    });

    const { result } = renderHook(() => useCoverImage());

    // Load the cover
    let loadedUrl: string | null = null;
    await act(async () => {
      loadedUrl = await result.current.loadCover("/path/to/cover.png");
    });

    expect(loadedUrl).toBe(mockDataUrl);
    expect(mockedCommands.loadCoverImage).toHaveBeenCalledWith(
      "/path/to/cover.png"
    );

    // Check it's now in cache
    const cached = result.current.getCachedCover("/path/to/cover.png");
    expect(cached).toBe(mockDataUrl);
  });

  it("should return cached cover on second call without fetching again", async () => {
    const mockDataUrl = "data:image/png;base64,cached123";
    mockedCommands.loadCoverImage.mockResolvedValueOnce({
      status: "ok",
      data: { data_url: mockDataUrl, width: 100, height: 150 },
    });

    const { result } = renderHook(() => useCoverImage());

    // First load
    await act(async () => {
      await result.current.loadCover("/path/to/cover.png");
    });

    // Reset mock to verify it's not called again
    mockedCommands.loadCoverImage.mockClear();

    // Second load should use cache
    let secondResult: string | null = null;
    await act(async () => {
      secondResult = await result.current.loadCover("/path/to/cover.png");
    });

    expect(secondResult).toBe(mockDataUrl);
    expect(mockedCommands.loadCoverImage).not.toHaveBeenCalled();
  });

  it("should handle loading errors gracefully", async () => {
    mockedCommands.loadCoverImage.mockResolvedValueOnce({
      status: "error",
      error: "Failed to load image",
    });

    const { result } = renderHook(() => useCoverImage());

    let loadedUrl: string | null = "initial";
    await act(async () => {
      loadedUrl = await result.current.loadCover("/path/to/broken.png");
    });

    expect(loadedUrl).toBeNull();
    // Should not be cached
    expect(result.current.getCachedCover("/path/to/broken.png")).toBeNull();
  });

  it("should return null while cover is loading to prevent duplicate requests", async () => {
    // Create a promise we can control
    let resolveLoad: (value: any) => void;
    const loadPromise = new Promise((resolve) => {
      resolveLoad = resolve;
    });
    mockedCommands.loadCoverImage.mockReturnValueOnce(loadPromise as any);

    const { result } = renderHook(() => useCoverImage());

    // Start loading
    act(() => {
      result.current.loadCover("/path/to/cover.png");
    });

    // Immediately check loading state
    expect(result.current.isLoadingCover("/path/to/cover.png")).toBe(true);

    // Try to load again while still loading - should return null and not start another request
    let secondResult: string | null = "initial";
    await act(async () => {
      secondResult = await result.current.loadCover("/path/to/cover.png");
    });
    expect(secondResult).toBeNull();

    // Should only have called once
    expect(mockedCommands.loadCoverImage).toHaveBeenCalledTimes(1);

    // Complete the load
    await act(async () => {
      resolveLoad!({
        status: "ok",
        data: { data_url: "data:image/png;base64,done", width: 100, height: 150 },
      });
    });

    await waitFor(() => {
      expect(result.current.isLoadingCover("/path/to/cover.png")).toBe(false);
    });
  });

  it("should invalidateCover remove from cache", async () => {
    const mockDataUrl = "data:image/png;base64,toberemoved";
    mockedCommands.loadCoverImage.mockResolvedValueOnce({
      status: "ok",
      data: { data_url: mockDataUrl, width: 100, height: 150 },
    });

    const { result } = renderHook(() => useCoverImage());

    // Load the cover
    await act(async () => {
      await result.current.loadCover("/path/to/cover.png");
    });

    // Verify it's cached
    expect(result.current.getCachedCover("/path/to/cover.png")).toBe(
      mockDataUrl
    );

    // Invalidate it
    act(() => {
      result.current.invalidateCover("/path/to/cover.png");
    });

    // Should no longer be in cache
    expect(result.current.getCachedCover("/path/to/cover.png")).toBeNull();
  });

  it("should clearCache remove all cached covers", async () => {
    mockedCommands.loadCoverImage
      .mockResolvedValueOnce({
        status: "ok",
        data: { data_url: "data:image/png;base64,cover1", width: 100, height: 150 },
      })
      .mockResolvedValueOnce({
        status: "ok",
        data: { data_url: "data:image/png;base64,cover2", width: 100, height: 150 },
      });

    const { result } = renderHook(() => useCoverImage());

    // Load multiple covers
    await act(async () => {
      await result.current.loadCover("/path/to/cover1.png");
      await result.current.loadCover("/path/to/cover2.png");
    });

    // Verify they're cached
    expect(result.current.getCachedCover("/path/to/cover1.png")).toBe(
      "data:image/png;base64,cover1"
    );
    expect(result.current.getCachedCover("/path/to/cover2.png")).toBe(
      "data:image/png;base64,cover2"
    );

    // Clear all
    act(() => {
      result.current.clearCache();
    });

    // Should all be gone
    expect(result.current.getCachedCover("/path/to/cover1.png")).toBeNull();
    expect(result.current.getCachedCover("/path/to/cover2.png")).toBeNull();
  });
});
