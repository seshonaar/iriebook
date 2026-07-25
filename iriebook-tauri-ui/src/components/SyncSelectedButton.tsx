import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands, events, type BookInfo } from "../bindings";
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
import { useGoogleAuthGate } from "../hooks/useGoogleAuthGate";
import { LinkGoogleDocDialog } from "./LinkGoogleDocDialog";
import { GoogleAuthDialog } from "./GoogleAuthDialog";

interface SyncSelectedButtonProps {
  books: BookInfo[];
  onBookUpdated: () => void;
}

export function SyncSelectedButton({ onBookUpdated }: SyncSelectedButtonProps) {
  const { t } = useTranslation();
  const [isSyncing, setIsSyncing] = useState(false);
  const [showLinkDialog, setShowLinkDialog] = useState(false);
  const [linkTarget, setLinkTarget] = useState<BookInfo | null>(null);
  const auth = useGoogleAuthGate();

  // Filter to books that have Google Docs linked
  const {
    targetBooks: linkedBooks,
    isCurrentBookMode,
    viewedBook,
  } = useActionTarget((book) => book.google_docs_sync_info != null);

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

  const performBatchSync = async () => {
    setIsSyncing(true);

    try {
      const result = await commands.googleSyncSelected(linkedBooks);
      if (result.status === "error") {
        toast.error(t("google.sync.messages.syncFailed"), {
          description: result.error,
        });
        setIsSyncing(false);
      }
      // UI updates will come from events
    } catch (err) {
      toast.error(t("google.sync.messages.syncFailed"), {
        description: String(err),
      });
      setIsSyncing(false);
    }
  };

  const handleSyncSelected = () => {
    // Current-book mode: if the viewed book isn't linked, treat this as a
    // link action (authenticate, then open the link dialog).
    if (isCurrentBookMode) {
      if (!viewedBook) {
        return;
      }

      const isLinked = viewedBook.google_docs_sync_info != null;
      if (!isLinked) {
        setLinkTarget(viewedBook);
        auth.ensureAuthenticated(() => setShowLinkDialog(true));
        return;
      }
    }

    if (linkedBooks.length === 0) return;

    // Auth gate: an expired token (e.g. after a system change) surfaces the
    // re-auth wizard instead of failing the batch sync silently.
    auth.ensureAuthenticated(performBatchSync);
  };

  // Listen to batch sync events
  useEffect(() => {
    const unlisten = events.googleDocsBatchSyncUpdateEvent.listen((event) => {
      const payload = event.payload;

      switch (payload.type) {
        case "started":
          // Track that sync started (no toast needed)
          break;

        case "progress":
          // Optional: could show progress updates
          break;

        case "completed":
          if (payload.success) {
            toast.success(
              t("google.sync.messages.syncSuccessDesc", {
                name: payload.book_name,
              })
            );
          } else {
            toast.error(`${payload.book_name}: ${payload.message}`);
          }
          break;

        case "all_done":
          setIsSyncing(false);

          if (payload.success_count > 0) {
            toast.success(
              t("google.sync.messages.bulkSyncSuccess", {
                count: payload.success_count,
              })
            );
            onBookUpdated(); // Refresh book list
          }

          if (payload.fail_count > 0) {
            toast.error(
              t("google.sync.messages.bulkSyncPartialFail", {
                count: payload.fail_count,
              })
            );
          }
          break;
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [t, onBookUpdated]);

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

      <GoogleAuthDialog
        open={auth.showAuthFlow}
        isAuthenticating={auth.isAuthenticating}
        authError={auth.authError}
        onCancel={auth.cancelAuthFlow}
      />
    </>
  );
}
