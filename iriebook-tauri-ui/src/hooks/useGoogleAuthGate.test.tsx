import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useGoogleAuthGate } from "./useGoogleAuthGate";
import { commands } from "../bindings";

const mockedCommands = vi.mocked(commands);

describe("useGoogleAuthGate", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("ensureAuthenticated", () => {
    it("runs onSuccess immediately when already authenticated", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });

      const onSuccess = vi.fn();
      const { result } = renderHook(() => useGoogleAuthGate());

      await act(async () => {
        await result.current.ensureAuthenticated(onSuccess);
      });

      expect(onSuccess).toHaveBeenCalledTimes(1);
      // Wizard must never appear when already authenticated
      expect(result.current.showAuthFlow).toBe(false);
      expect(mockedCommands.googleAuthStart).not.toHaveBeenCalled();
    });

    it("shows wizard and runs onSuccess after successful auth when not authenticated", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      const onSuccess = vi.fn();
      const { result } = renderHook(() => useGoogleAuthGate());

      await act(async () => {
        await result.current.ensureAuthenticated(onSuccess);
        // Flush the 100ms DOM-settle delay inside startAuthFlow
        await vi.advanceTimersByTimeAsync(150);
      });

      // Wizard opened, auth attempted, then onSuccess resumed
      expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
      expect(onSuccess).toHaveBeenCalledTimes(1);
      expect(result.current.showAuthFlow).toBe(false);
    });

    it("falls back to the wizard when the auth check throws", async () => {
      mockedCommands.googleCheckAuth.mockRejectedValueOnce(new Error("network"));
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "error",
        error: "Authentication cancelled",
      });

      const onSuccess = vi.fn();
      const { result } = renderHook(() => useGoogleAuthGate());

      await act(async () => {
        await result.current.ensureAuthenticated(onSuccess);
        await vi.advanceTimersByTimeAsync(150);
      });

      expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
      // Auth was cancelled, so onSuccess must not run
      expect(onSuccess).not.toHaveBeenCalled();
    });

    it("does not run onSuccess when auth is cancelled mid-flow", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "error",
        error: "Authentication cancelled",
      });

      const onSuccess = vi.fn();
      const { result } = renderHook(() => useGoogleAuthGate());

      await act(async () => {
        await result.current.ensureAuthenticated(onSuccess);
        await vi.advanceTimersByTimeAsync(150);
      });

      expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
      expect(onSuccess).not.toHaveBeenCalled();
      expect(result.current.showAuthFlow).toBe(false);
    });
  });

  describe("startAuthFlow", () => {
    it("surfaces the error message when auth fails (non-cancelled)", async () => {
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "error",
        error: "Server is down",
      });

      const { result } = renderHook(() => useGoogleAuthGate());

      await act(async () => {
        await result.current.startAuthFlow();
        await vi.advanceTimersByTimeAsync(150);
      });

      expect(result.current.authError).toContain("Server is down");
      expect(result.current.isAuthenticating).toBe(false);
    });

    it("runs onSuccess after a successful explicit start", async () => {
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      const onSuccess = vi.fn();
      const { result } = renderHook(() => useGoogleAuthGate());

      await act(async () => {
        await result.current.startAuthFlow(onSuccess);
        await vi.advanceTimersByTimeAsync(150);
      });

      expect(onSuccess).toHaveBeenCalledTimes(1);
    });
  });

  describe("cancelAuthFlow", () => {
    it("cancels the backend flow and resets state", async () => {
      mockedCommands.googleAuthStart.mockImplementation(
        () => new Promise(() => {}) // never resolves until cancelled
      );

      const { result } = renderHook(() => useGoogleAuthGate());

      // Kick off an auth flow, then cancel it before it settles
      act(() => {
        result.current.startAuthFlow();
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(150);
      });

      expect(result.current.isAuthenticating).toBe(true);

      await act(async () => {
        await result.current.cancelAuthFlow();
      });

      expect(mockedCommands.googleAuthCancel).toHaveBeenCalledTimes(1);
      expect(result.current.showAuthFlow).toBe(false);
      expect(result.current.isAuthenticating).toBe(false);
      expect(result.current.authError).toBeNull();
    });
  });

  describe("initial state", () => {
    it("starts closed and idle", () => {
      const { result } = renderHook(() => useGoogleAuthGate());

      expect(result.current.showAuthFlow).toBe(false);
      expect(result.current.isAuthenticating).toBe(false);
      expect(result.current.authError).toBeNull();
    });
  });

  // Keep waitFor referenced for future async-state assertions
  it("exposes stable callbacks across re-renders", async () => {
    const { result, rerender } = renderHook(() => useGoogleAuthGate());
    const first = result.current.ensureAuthenticated;
    rerender();
    await waitFor(() => {
      expect(result.current.ensureAuthenticated).toBe(first);
    });
  });
});
