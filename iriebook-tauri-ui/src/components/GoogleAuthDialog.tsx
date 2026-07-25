import { useTranslation } from "react-i18next";
import { Button } from "./ui/button";
import { Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "./ui/dialog";

interface GoogleAuthDialogProps {
  /** Whether the dialog should be visible. */
  open: boolean;
  /** Whether the OAuth flow is in progress (spinner + "Browser opened..."). */
  isAuthenticating: boolean;
  /** Error message to display, if any. */
  authError: string | null;
  /** Called when the user dismisses the dialog (Cancel button or backdrop). */
  onCancel: () => void;
}

/**
 * Shared Google OAuth dialog.
 *
 * Renders the "sign in to your Google account" wizard used by every
 * auth-gated action. Pair with the {@link useGoogleAuthGate} hook, which
 * owns the state this component renders.
 */
export function GoogleAuthDialog({
  open,
  isAuthenticating,
  authError,
  onCancel,
}: GoogleAuthDialogProps) {
  const { t } = useTranslation();

  if (!open) return null;

  return (
    <Dialog open onOpenChange={onCancel}>
      <DialogContent className="max-w-md" data-testid="google-auth-dialog">
        <DialogHeader>
          <DialogTitle>{t("google.auth.title")}</DialogTitle>
          <DialogDescription>
            Please sign in to your Google account to continue.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {isAuthenticating && (
            <div
              className="flex flex-col items-center justify-center py-6 space-y-4 border border-dashed rounded-md bg-muted/50"
              data-testid="google-auth-loading"
            >
              <Loader2 className="h-8 w-8 animate-spin text-primary" />
              <div className="text-center">
                <p className="font-medium">Browser opened...</p>
                <p className="text-xs text-muted-foreground">
                  Check your browser to complete sign in.
                </p>
              </div>
            </div>
          )}

          {authError && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 text-destructive rounded-md text-sm">
              <p>{authError}</p>
            </div>
          )}

          <div className="flex justify-end">
            <Button
              variant="ghost"
              onClick={onCancel}
              data-testid="google-auth-cancel-button"
            >
              Cancel
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
