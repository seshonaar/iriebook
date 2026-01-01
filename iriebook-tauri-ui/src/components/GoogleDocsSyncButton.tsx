import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type BookInfo } from "../bindings";
import { Button } from "./ui/button";
import { Cloud, CloudOff, CloudCheck, CloudAlert, Loader2, X } from "lucide-react";
import { LinkGoogleDocDialog } from "./LinkGoogleDocDialog";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "./ui/dialog";

interface GoogleDocsSyncButtonProps {
  book: BookInfo;
  onBookUpdated?: () => void;
  hideUnlink?: boolean;
}

export function GoogleDocsSyncButton({ book, onBookUpdated, hideUnlink = false }: GoogleDocsSyncButtonProps) {
  const { t } = useTranslation();
  const [isSyncing, setIsSyncing] = useState(false);
  const [showLinkDialog, setShowLinkDialog] = useState(false);
  const [showAuthFlow, setShowAuthFlow] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<"link" | "sync" | null>(null);

  const syncInfo = book.google_docs_sync_info;
  const isLinked = syncInfo != null;
  const syncStatus = syncInfo?.["sync-status"] || "never_synced";

  const handleSync = async () => {
    // Check if still authenticated (token might have expired)
    try {
      const authResult = await commands.googleCheckAuth();
      if (authResult.status === "ok" && !authResult.data) {
        // Token expired, re-authenticate
        setPendingAction("sync");
        startAuthFlow();
        return;
      }
    } catch (err) {
      setPendingAction("sync");
      startAuthFlow();
      return;
    }

    performSync();
  };

  const performSync = async () => {
    setIsSyncing(true);
    try {
      const result = await commands.googleSyncDoc(book.path);
      if (result.status === "ok") {
        console.log(t("google.sync.messages.syncSuccess"), book.display_name);
        onBookUpdated?.();
      } else {
        console.error(t("google.sync.messages.syncFailed"), result.error);
      }
    } catch (err) {
      console.error(t("google.sync.messages.syncFailed"), err);
    } finally {
      setIsSyncing(false);
    }
  };

  const handleUnlink = async () => {
    try {
      const result = await commands.googleUnlinkDoc(book.path);
      if (result.status === "ok") {
        console.log(t("google.sync.messages.unlinkSuccess"), book.display_name);
        onBookUpdated?.();
      } else {
        console.error(t("google.sync.messages.unlinkFailed"), result.error);
      }
    } catch (err) {
      console.error(t("google.sync.messages.unlinkFailed"), err);
    }
  };

  const handleLinkClick = async () => {
    // Check if authenticated first
    try {
      const result = await commands.googleCheckAuth();
      if (result.status === "ok" && result.data) {
        setShowLinkDialog(true);
      } else {
        // Not authenticated, start auth flow
        setPendingAction("link");
        startAuthFlow();
      }
    } catch (err) {
      setPendingAction("link");
      startAuthFlow();
    }
  };

  const startAuthFlow = async () => {
    setIsAuthenticating(true);
    setAuthError(null);
    setShowAuthFlow(true);

    try {
      // Start auth flow
      const result = await commands.googleAuthStart();
      if (result.status === "error") {
        // If "Authentication cancelled" is the error, we handle it gracefully
        if (result.error.includes("cancelled")) {
            setIsAuthenticating(false);
            setShowAuthFlow(false);
            setPendingAction(null);
            return;
        }
        throw new Error(result.error);
      }

      // Auth successful!
      setIsAuthenticating(false);
      setShowAuthFlow(false);
      setAuthError(null);

      // Execute the action that triggered auth
      if (pendingAction === "link") {
        setShowLinkDialog(true);
      } else if (pendingAction === "sync") {
        performSync();
      }
      setPendingAction(null);
    } catch (err) {
      setAuthError(String(err));
      setIsAuthenticating(false);
    }
  };

  const cancelAuthFlow = async () => {
      try {
          await commands.googleAuthCancel();
      } catch (err) {
          console.error("Failed to cancel auth:", err);
      }
      setShowAuthFlow(false);
      setIsAuthenticating(false);
      setPendingAction(null);
  };

  const getStatusIcon = () => {
    if (isSyncing) {
      return <Loader2 className="h-4 w-4 animate-spin" />;
    }

    if (!isLinked) {
      return <CloudOff className="h-4 w-4 text-muted-foreground" />;
    }

    switch (syncStatus) {
      case "synced":
        return <CloudCheck className="h-4 w-4 text-green-500" />;
      case "sync_failed":
        return <CloudAlert className="h-4 w-4 text-red-500" />;
      case "never_synced":
      default:
        return <Cloud className="h-4 w-4 text-gray-400" />;
    }
  };

  const getTooltip = () => {
    if (!isLinked) return t("google.sync.button.link");
    if (syncStatus === "synced") {
      return t("google.sync.button.synced");
    }
    if (syncStatus === "sync_failed") return t("google.sync.button.syncFailed");
    return t("google.sync.button.neverSynced");
  };

  return (
    <>
      <div className="flex gap-1">
        {isLinked ? (
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleSync}
              disabled={isSyncing}
              title={getTooltip()}
              className="h-8 w-8 p-0"
            >
              {getStatusIcon()}
            </Button>
            {!hideUnlink && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleUnlink}
                title={t("google.sync.button.unlink")}
                className="h-8 w-8 p-0"
              >
                <X className="h-4 w-4 text-muted-foreground" />
              </Button>
            )}
          </>
        ) : (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleLinkClick}
            title={getTooltip()}
            className="h-8 w-8 p-0"
          >
            {getStatusIcon()}
          </Button>
        )}
      </div>

      {showLinkDialog && (
        <LinkGoogleDocDialog
          book={book}
          onClose={() => setShowLinkDialog(false)}
          onLinked={() => {
            setShowLinkDialog(false);
            onBookUpdated?.();
          }}
        />
      )}

      {showAuthFlow && (
        <Dialog open onOpenChange={() => {
             // Closing the dialog manually triggers cancellation
             if (isAuthenticating) {
                 cancelAuthFlow();
             } else {
                 setShowAuthFlow(false);
             }
        }}>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>{t("google.auth.title")}</DialogTitle>
              <DialogDescription>
                Please sign in to your Google account to continue.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              {isAuthenticating && (
                <div className="flex flex-col items-center justify-center py-6 space-y-4 border border-dashed rounded-md bg-muted/50">
                  <Loader2 className="h-8 w-8 animate-spin text-primary" />
                  <div className="text-center">
                    <p className="font-medium">Browser opened...</p>
                    <p className="text-xs text-muted-foreground">Check your browser to complete sign in.</p>
                  </div>
                </div>
              )}

              {authError && (
                <div className="p-3 bg-destructive/10 border border-destructive/20 text-destructive rounded-md text-sm">
                  <p>{authError}</p>
                </div>
              )}

              <div className="flex justify-end">
                  <Button variant="ghost" onClick={cancelAuthFlow}>
                      Cancel
                  </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
