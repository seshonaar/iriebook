import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands, type BookInfo } from "../bindings";
import { Button } from "./ui/button";
import { RefreshCw, Loader2 } from "lucide-react";
import { toast } from "sonner";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";
import { useActionTarget } from "../hooks/useActionTarget";
import { LinkGoogleDocDialog } from "./LinkGoogleDocDialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";

interface SyncSelectedButtonProps {
  books: BookInfo[];
  onBookUpdated: () => void;
}

export function SyncSelectedButton({ onBookUpdated }: SyncSelectedButtonProps) {
  const { t } = useTranslation();
  const [isSyncing, setIsSyncing] = useState(false);
  const [showLinkDialog, setShowLinkDialog] = useState(false);
  const [linkTarget, setLinkTarget] = useState<BookInfo | null>(null);
  const [showAuthFlow, setShowAuthFlow] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);

  // Filter to books that have Google Docs linked
  const {
    targetBooks: linkedBooks,
    isCurrentBookMode,
    viewedBook,
  } = useActionTarget((book) => book.google_docs_sync_info != null);

  const startAuthFlow = async (bookToLink?: BookInfo) => {
    setIsAuthenticating(true);
    setAuthError(null);
    setShowAuthFlow(true);

    // Give React time to flush state updates to DOM before starting OAuth
    // This ensures the dialog is visible before the async operation begins
    await new Promise(resolve => setTimeout(resolve, 100));

    try {
      const authStart = await commands.googleAuthStart();
      if (authStart.status === "error") {
        if (authStart.error.includes("cancelled")) {
          setIsAuthenticating(false);
          setShowAuthFlow(false);
          return;
        }
        throw new Error(authStart.error);
      }

      setIsAuthenticating(false);
      setShowAuthFlow(false);

      // Use passed parameter instead of state to avoid stale state issues
      if (bookToLink) {
        setShowLinkDialog(true);
      }
    } catch (err) {
      setAuthError(String(err));
      setIsAuthenticating(false);
    }
  };

  const ensureAuthForLink = async (book: BookInfo) => {
    setLinkTarget(book);
    try {
      const authResult = await commands.googleCheckAuth();
      if (authResult.status === "ok" && authResult.data) {
        setShowLinkDialog(true);
        return;
      }
    } catch (err) {
      console.error("Failed to check auth:", err);
    }

    await startAuthFlow(book);  // Pass book to avoid stale state
  };

  const cancelAuthFlow = async () => {
    try {
      await commands.googleAuthCancel();
    } catch (err) {
      console.error("Failed to cancel auth:", err);
    }
    setShowAuthFlow(false);
    setIsAuthenticating(false);
    setAuthError(null);
  };

  const syncSingleBook = async (book: BookInfo) => {
    setIsSyncing(true);
    try {
      const result = await commands.googleSyncDoc(book.path);
      if (result.status === "ok") {
        toast.success(
          t("google.sync.messages.syncSuccessDesc", { name: book.display_name })
        );
        onBookUpdated();
      } else {
        toast.error(t("google.sync.messages.syncFailed"), {
          description: result.error,
        });
      }
    } catch (err) {
      toast.error(t("google.sync.messages.syncFailed"), {
        description: String(err),
      });
    } finally {
      setIsSyncing(false);
    }
  };

  const handleSyncSelected = async () => {
    if (isCurrentBookMode) {
      if (!viewedBook) {
        return;
      }

      const isLinked = viewedBook.google_docs_sync_info != null;
      if (!isLinked) {
        await ensureAuthForLink(viewedBook);
        return;
      }
    }

    if (linkedBooks.length === 0) return;

    setIsSyncing(true);
    let successCount = 0;
    let failCount = 0;

    try {
      // Check auth first
      const authResult = await commands.googleCheckAuth();
      if (authResult.status !== "ok" || !authResult.data) {
         const authStart = await commands.googleAuthStart();
         if (authStart.status === "error") {
            throw new Error("Authentication failed or cancelled");
         }
      }

      toast.info(t("google.sync.messages.startingBulkSync", { count: linkedBooks.length }));

      for (const book of linkedBooks) {
        try {
          const result = await commands.googleSyncDoc(book.path);
          if (result.status === "ok") {
            successCount++;
          } else {
            failCount++;
            console.error(`Failed to sync ${book.display_name}:`, result.error);
          }
        } catch (err) {
          failCount++;
          console.error(`Failed to sync ${book.display_name}:`, err);
        }
      }

      if (successCount > 0) {
        toast.success(t("google.sync.messages.bulkSyncSuccess", { count: successCount }));
        onBookUpdated();
      }
      
      if (failCount > 0) {
        toast.error(t("google.sync.messages.bulkSyncPartialFail", { count: failCount }));
      }

    } catch (err) {
      toast.error(t("google.sync.messages.syncFailed"), {
        description: String(err)
      });
    } finally {
      setIsSyncing(false);
    }
  };

  const isDisabled =
    isSyncing ||
    (!isCurrentBookMode && linkedBooks.length === 0) ||
    (isCurrentBookMode && !viewedBook);

  return (
    <>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="outline"
              size="lg"
              onClick={handleSyncSelected}
              disabled={isDisabled}
              className="min-w-max"
              data-testid="sync-selected-button"
            >
              {isSyncing ? (
                <Loader2 className="animate-spin" />
              ) : (
                <RefreshCw />
              )}
              <span className="translate-y-px">{t("google.sync.button.syncSelected")}</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{t("google.sync.tooltips.syncSelected", { count: linkedBooks.length })}</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      {showLinkDialog && linkTarget && (
        <LinkGoogleDocDialog
          book={linkTarget}
          onClose={() => {
            setShowLinkDialog(false);
            setLinkTarget(null);
          }}
          onLinked={async () => {
            setShowLinkDialog(false);
            if (linkTarget) {
              await syncSingleBook(linkTarget);
            }
            setLinkTarget(null);
          }}
        />
      )}

      {showAuthFlow && (
        <Dialog
          open
          onOpenChange={() => {
            if (isAuthenticating) {
              cancelAuthFlow();
            } else {
              setShowAuthFlow(false);
            }
          }}
        >
          <DialogContent className="max-w-md" data-testid="google-auth-dialog">
            <DialogHeader>
              <DialogTitle>{t("google.auth.title")}</DialogTitle>
              <DialogDescription>
                Please sign in to your Google account to continue.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              {isAuthenticating && (
                <div className="flex flex-col items-center justify-center py-6 space-y-4 border border-dashed rounded-md bg-muted/50" data-testid="google-auth-loading">
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
                <Button variant="ghost" onClick={cancelAuthFlow} data-testid="google-auth-cancel-button">
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
