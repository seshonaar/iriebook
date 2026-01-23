import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useCoverImage } from "./useCoverImage";
import React from "react";
import { AppProvider } from "../contexts/AppContext";

// Wrapper to provide context
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <AppProvider>{children}</AppProvider>
);

describe("useCoverImage", () => {
  it("should provide clearCache function", () => {
    const { result } = renderHook(() => useCoverImage(), { wrapper });
    expect(typeof result.current.clearCache).toBe("function");
  });

  it("should provide invalidateCover function", () => {
    const { result } = renderHook(() => useCoverImage(), { wrapper });
    expect(typeof result.current.invalidateCover).toBe("function");
  });

  it("should not throw when calling clearCache", () => {
    const { result } = renderHook(() => useCoverImage(), { wrapper });
    expect(() => {
      act(() => {
        result.current.clearCache();
      });
    }).not.toThrow();
  });

  it("should not throw when calling invalidateCover", () => {
    const { result } = renderHook(() => useCoverImage(), { wrapper });
    expect(() => {
      act(() => {
        result.current.invalidateCover("/path/to/cover.png");
      });
    }).not.toThrow();
  });
});
