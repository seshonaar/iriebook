import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { GitSyncPanel } from "./GitSyncPanel";
import { AppProvider } from "../contexts/AppContext";
import { commands } from "../bindings";
import React from "react";

// Get the mocked commands
const mockedCommands = vi.mocked(commands);

// Wrapper with AppProvider
function renderWithProvider(ui: React.ReactElement) {
  return render(<AppProvider>{ui}</AppProvider>);
}

describe("GitSyncPanel", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe("Initial Auth Check", () => {
    it("should check auth status on mount", async () => {
      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        expect(mockedCommands.githubCheckAuth).toHaveBeenCalledTimes(1);
      });
    });

    it("should show connect button when not authenticated", async () => {
      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        // Button text is the translation key since we mock t() to return the key
        expect(screen.getByText("git.auth.connect")).toBeInTheDocument();
      });
    });

    it("should show authenticated UI when already authenticated", async () => {
      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });
      mockedCommands.gitGetStatus.mockResolvedValueOnce({
        status: "ok",
        data: { status: "Clean" },
      });

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        // Should show disconnect button when authenticated (logout icon)
        expect(screen.getByText("git.auth.disconnect")).toBeInTheDocument();
      });
    });
  });

  describe("GitHub Device Flow Authentication", () => {
    it("should start device flow when connect button is clicked", async () => {
      const user = userEvent.setup();

      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.githubDeviceFlowStart.mockResolvedValueOnce({
        status: "ok",
        data: {
          deviceCode: "test-device",
          userCode: "WDJB-MJHT",
          verificationUri: "https://github.com/login/device",
          expiresIn: 900,
        },
      });
      mockedCommands.openBrowser.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });
      // Mock polling to return pending forever (we'll test success separately)
      mockedCommands.githubDeviceFlowPoll.mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );

      renderWithProvider(<GitSyncPanel />);

      // Wait for initial render
      await waitFor(() => {
        expect(screen.getByText("git.auth.connect")).toBeInTheDocument();
      });

      // Click connect button
      const connectButton = screen.getByText("git.auth.connect");
      await user.click(connectButton);

      await waitFor(() => {
        expect(mockedCommands.githubDeviceFlowStart).toHaveBeenCalledTimes(1);
      });

      await waitFor(() => {
        expect(mockedCommands.openBrowser).toHaveBeenCalledWith(
          "https://github.com/login/device"
        );
      });
    });

    it("should display user code during device flow", async () => {
      const user = userEvent.setup();

      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.githubDeviceFlowStart.mockResolvedValueOnce({
        status: "ok",
        data: {
          deviceCode: "test-device",
          userCode: "WDJB-MJHT",
          verificationUri: "https://github.com/login/device",
          expiresIn: 900,
        },
      });
      mockedCommands.openBrowser.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });
      mockedCommands.githubDeviceFlowPoll.mockImplementation(
        () => new Promise(() => {})
      );

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        expect(screen.getByText("git.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("git.auth.connect");
      await user.click(connectButton);

      // Should display user code
      await waitFor(() => {
        expect(screen.getByText("WDJB-MJHT")).toBeInTheDocument();
      });
    });

    it("should complete authentication when polling succeeds", async () => {
      const user = userEvent.setup();

      mockedCommands.githubCheckAuth
        .mockResolvedValueOnce({ status: "ok", data: false }) // Initial check
        .mockResolvedValueOnce({ status: "ok", data: true }); // After success

      mockedCommands.githubDeviceFlowStart.mockResolvedValueOnce({
        status: "ok",
        data: {
          deviceCode: "test-device",
          userCode: "TEST-CODE",
          verificationUri: "https://github.com/login/device",
          expiresIn: 900,
        },
      });
      mockedCommands.openBrowser.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });
      mockedCommands.githubDeviceFlowPoll.mockResolvedValueOnce({
        status: "ok",
        data: "test-token-123",
      });
      mockedCommands.githubStoreToken.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });
      mockedCommands.gitCheckInitialized.mockResolvedValueOnce({
        status: "ok",
        data: true, // Already initialized, no clone prompt
      });
      mockedCommands.gitGetStatus.mockResolvedValueOnce({
        status: "ok",
        data: { status: "Clean" },
      });

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        expect(screen.getByText("git.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("git.auth.connect");
      await user.click(connectButton);

      // Wait for token to be stored
      await waitFor(() => {
        expect(mockedCommands.githubStoreToken).toHaveBeenCalledWith(
          "test-token-123"
        );
      });
    });

    it("should handle device flow start error", async () => {
      const user = userEvent.setup();

      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.githubDeviceFlowStart.mockResolvedValueOnce({
        status: "error",
        error: "Failed to start device flow",
      });

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        expect(screen.getByText("git.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("git.auth.connect");
      await user.click(connectButton);

      // Should show error message
      await waitFor(() => {
        expect(
          screen.getByText(/Failed to start device flow/i)
        ).toBeInTheDocument();
      });
    });

    it("should handle polling error", async () => {
      const user = userEvent.setup();

      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: false,
      });
      mockedCommands.githubDeviceFlowStart.mockResolvedValueOnce({
        status: "ok",
        data: {
          deviceCode: "test-device",
          userCode: "TEST-CODE",
          verificationUri: "https://github.com/login/device",
          expiresIn: 900,
        },
      });
      mockedCommands.openBrowser.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });
      mockedCommands.githubDeviceFlowPoll.mockResolvedValueOnce({
        status: "error",
        error: "User denied access",
      });

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        expect(screen.getByText("git.auth.connect")).toBeInTheDocument();
      });

      const connectButton = screen.getByText("git.auth.connect");
      await user.click(connectButton);

      // Should show error message
      await waitFor(() => {
        expect(screen.getByText(/User denied access/i)).toBeInTheDocument();
      });
    });
  });

  describe("Logout", () => {
    it("should logout when disconnect button is clicked", async () => {
      const user = userEvent.setup();

      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });
      mockedCommands.gitGetStatus.mockResolvedValueOnce({
        status: "ok",
        data: { status: "Clean" },
      });
      mockedCommands.githubLogout.mockResolvedValueOnce({
        status: "ok",
        data: null,
      });

      renderWithProvider(<GitSyncPanel />);

      // Wait for authenticated state - disconnect button has sr-only text
      await waitFor(() => {
        expect(screen.getByText("git.auth.disconnect")).toBeInTheDocument();
      });

      // Find the button containing the disconnect text
      const logoutButton = screen.getByText("git.auth.disconnect").closest("button")!;
      await user.click(logoutButton);

      await waitFor(() => {
        expect(mockedCommands.githubLogout).toHaveBeenCalledTimes(1);
      });
    });

    it("should handle logout error", async () => {
      const user = userEvent.setup();

      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "ok",
        data: true,
      });
      mockedCommands.gitGetStatus.mockResolvedValueOnce({
        status: "ok",
        data: { status: "Clean" },
      });
      mockedCommands.githubLogout.mockResolvedValueOnce({
        status: "error",
        error: "Failed to logout",
      });

      renderWithProvider(<GitSyncPanel />);

      await waitFor(() => {
        expect(screen.getByText("git.auth.disconnect")).toBeInTheDocument();
      });

      const logoutButton = screen
        .getByText("git.auth.disconnect")
        .closest("button")!;
      await user.click(logoutButton);

      // Should show error message
      await waitFor(() => {
        expect(screen.getByText(/Failed to logout/i)).toBeInTheDocument();
      });
    });
  });

  describe("Auth Check Error Handling", () => {
    it("should handle auth check error gracefully", async () => {
      mockedCommands.githubCheckAuth.mockResolvedValueOnce({
        status: "error",
        error: "Network error",
      });

      // Should not crash
      renderWithProvider(<GitSyncPanel />);

      // Should still show connect button (assumes not authenticated on error)
      await waitFor(() => {
        expect(screen.getByText("git.auth.connect")).toBeInTheDocument();
      });
    });
  });
});
