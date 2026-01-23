import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { GoogleAuthPanel } from "./GoogleAuthPanel";
import { commands } from "../bindings";

// Get the mocked commands
const mockedCommands = vi.mocked(commands);

describe("GoogleAuthPanel", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe("Initial Auth Check", () => {
    it("should check auth status on mount", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(mockedCommands.googleCheckAuth).toHaveBeenCalledTimes(1);
      });
    });

    it("should show connect button when not authenticated", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.connect")).toBeInTheDocument();
      });
    });

    it("should show connected state when authenticated", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.connected")).toBeInTheDocument();
      });
    });

    it("should show disconnect button when authenticated", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.disconnect")).toBeInTheDocument();
      });
    });
  });

  describe("Google OAuth Authentication", () => {
    it("should start auth flow when connect button is clicked", async () => {
      const user = userEvent.setup();

      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      // Auth flow that succeeds
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("google.auth.connect");
      await user.click(connectButton);

      await waitFor(() => {
        expect(mockedCommands.googleAuthStart).toHaveBeenCalledTimes(1);
      });
    });

    it("should show loading state during authentication", async () => {
      const user = userEvent.setup();

      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      // Auth flow that never completes (to test loading state)
      mockedCommands.googleAuthStart.mockImplementation(
        () => new Promise(() => {})
      );

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("google.auth.connect");
      await user.click(connectButton);

      // Should show loading UI
      await waitFor(() => {
        expect(screen.getByText("Browser opened...")).toBeInTheDocument();
      });
    });

    it("should show cancel button during authentication", async () => {
      const user = userEvent.setup();

      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.googleAuthStart.mockImplementation(
        () => new Promise(() => {})
      );

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("google.auth.connect");
      await user.click(connectButton);

      // Should show cancel button
      await waitFor(() => {
        expect(screen.getByText("Cancel")).toBeInTheDocument();
      });
    });

    it("should call cancel when cancel button is clicked", async () => {
      const user = userEvent.setup();

      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });

      let resolveAuth: (value: any) => void;
      mockedCommands.googleAuthStart.mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveAuth = resolve;
          })
      );
      mockedCommands.googleAuthCancel.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("google.auth.connect");
      await user.click(connectButton);

      await waitFor(() => {
        expect(screen.getByText("Cancel")).toBeInTheDocument();
      });

      const cancelButton = screen.getByText("Cancel");
      await user.click(cancelButton);

      await waitFor(() => {
        expect(mockedCommands.googleAuthCancel).toHaveBeenCalledTimes(1);
      });

      // Resolve the auth with cancelled error
      resolveAuth!({
        status: "error",
        error: "Authentication cancelled",
      });

      // Should return to connect button
      await waitFor(() => {
        expect(screen.getByText("google.auth.connect")).toBeInTheDocument();
      });
    });

    it("should update to authenticated state on success", async () => {
      const user = userEvent.setup();

      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.googleAuthStart.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("google.auth.connect");
      await user.click(connectButton);

      // Should show connected state after success
      await waitFor(() => {
        expect(screen.getByText("google.auth.connected")).toBeInTheDocument();
      });
    });
  });

  describe("Logout", () => {
    it("should logout when disconnect button is clicked", async () => {
      const user = userEvent.setup();

      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });
      mockedCommands.googleLogout.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.disconnect")).toBeInTheDocument();
      });

      const logoutButton = screen.getByText("google.auth.disconnect");
      await user.click(logoutButton);

      await waitFor(() => {
        expect(mockedCommands.googleLogout).toHaveBeenCalledTimes(1);
      });
    });

    it("should return to not authenticated state after logout", async () => {
      const user = userEvent.setup();

      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });
      mockedCommands.googleLogout.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      render(<GoogleAuthPanel />);

      await waitFor(() => {
        expect(screen.getByText("google.auth.disconnect")).toBeInTheDocument();
      });

      const logoutButton = screen.getByText("google.auth.disconnect");
      await user.click(logoutButton);

      // Should show not connected state
      await waitFor(() => {
        expect(screen.getByText("google.auth.notConnected")).toBeInTheDocument();
      });
    });
  });

  describe("Auth Check Error Handling", () => {
    it("should handle auth check error gracefully", async () => {
      mockedCommands.googleCheckAuth.mockResolvedValueOnce({
        status: "error",
        error: "Network error",
      });

      // Should not crash
      render(<GoogleAuthPanel />);

      // Should still show not connected state
      await waitFor(() => {
        expect(screen.getByText("google.auth.notConnected")).toBeInTheDocument();
      });
    });
  });
});
