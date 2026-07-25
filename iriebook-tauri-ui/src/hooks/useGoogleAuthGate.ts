import { useState, useCallback } from "react";
import { commands } from "../bindings";

/**
 * Return type of {@link useGoogleAuthGate}.
 *
 * Exposes the rendering state for an auth dialog plus the three operations
 * every Google-auth-gated action needs.
 */
export interface UseGoogleAuthGateResult {
  /** Whether the auth dialog should be rendered. */
  showAuthFlow: boolean;
  /** Whether the OAuth flow is in progress (waiting on the browser). */
  isAuthenticating: boolean;
  /** Error message to display inside the dialog, if any. */
  authError: string | null;

  /**
   * Ensures the user is authenticated before running `onSuccess`.
   *
   * - Already authenticated → runs `onSuccess` immediately.
   * - Not authenticated / check failed → shows the auth wizard,
   *   then runs `onSuccess` after a successful authentication.
   *
   * This is the single entry point every auth-gated action should use,
   * so that an expired token (e.g. after a system change) always surfaces
   * the re-auth wizard instead of failing silently.
   */
  ensureAuthenticated: (onSuccess: () => void) => Promise<void>;

  /**
   * Starts the OAuth flow explicitly (always shows the wizard),
   * then runs `onSuccess` after a successful authentication.
   */
  startAuthFlow: (onSuccess?: () => void) => Promise<void>;

  /** Cancels any in-progress auth flow and hides the dialog. */
  cancelAuthFlow: () => Promise<void>;
}

/**
 * Shared Google OAuth gate for components that need to authenticate before
 * acting (syncing, linking, etc.).
 *
 * Replaces the auth-state + startAuthFlow + cancelAuthFlow logic that was
 * previously duplicated in `GoogleDocsSyncButton` and `SyncSelectedButton`.
 */
export function useGoogleAuthGate(): UseGoogleAuthGateResult {
  const [showAuthFlow, setShowAuthFlow] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);

  const startAuthFlow = useCallback(async (onSuccess?: () => void) => {
    setIsAuthenticating(true);
    setAuthError(null);
    setShowAuthFlow(true);

    // Give React time to flush state updates to DOM before starting OAuth,
    // so the dialog is visible before the (blocking) async operation begins.
    await new Promise((resolve) => setTimeout(resolve, 100));

    try {
      const result = await commands.googleAuthStart();
      if (result.status === "error") {
        // User dismissed the browser flow — close quietly without an error.
        if (result.error.includes("cancelled")) {
          setIsAuthenticating(false);
          setShowAuthFlow(false);
          return;
        }
        throw new Error(result.error);
      }

      setIsAuthenticating(false);
      setShowAuthFlow(false);
      setAuthError(null);
      onSuccess?.();
    } catch (err) {
      setAuthError(String(err));
      setIsAuthenticating(false);
    }
  }, []);

  const ensureAuthenticated = useCallback(
    async (onSuccess: () => void) => {
      try {
        const authResult = await commands.googleCheckAuth();
        if (authResult.status === "ok" && authResult.data) {
          onSuccess();
          return;
        }
      } catch (err) {
        // Auth check itself failed — fall through to the wizard as a
        // safe default rather than blocking the user entirely.
        console.error("Failed to check auth:", err);
      }
      await startAuthFlow(onSuccess);
    },
    [startAuthFlow]
  );

  const cancelAuthFlow = useCallback(async () => {
    try {
      await commands.googleAuthCancel();
    } catch (err) {
      console.error("Failed to cancel auth:", err);
    }
    setShowAuthFlow(false);
    setIsAuthenticating(false);
    setAuthError(null);
  }, []);

  return {
    showAuthFlow,
    isAuthenticating,
    authError,
    ensureAuthenticated,
    startAuthFlow,
    cancelAuthFlow,
  };
}
