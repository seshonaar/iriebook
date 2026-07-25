import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type BookInfo } from "../bindings";
import { Button } from "./ui/button";
import { Cloud, CloudOff, CloudCheck, CloudAlert, Loader2, X } from "lucide-react";
import { LinkGoogleDocDialog } from "./LinkGoogleDocDialog";
import { GoogleAuthDialog } from "./GoogleAuthDialog";
import { useGoogleAuthGate } from "../hooks/useGoogleAuthGate";

interface GoogleDocsSyncButtonProps {
  book: BookInfo;
  onBookUpdated?: () => void;
  hideUnlink?: boolean;
}

export function GoogleDocsSyncButton({ book, onBookUpdated, hideUnlink = false }: GoogleDocsSyncButtonProps) {
  const { t } = useTranslation();
  const [isSyncing, setIsSyncing] = useState(false);
  const [showLinkDialog, setShowLinkDialog] = useState(false);
  const auth = useGoogleAuthGate();

  const syncInfo = book.google_docs_sync_info;
  const isLinked = syncInfo != null;
  const syncStatus = syncInfo?.["sync-status"] || "never_synced";

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

  const handleSync = () => {
    auth.ensureAuthenticated(performSync);
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

  const handleLinkClick = () => {
    auth.ensureAuthenticated(() => setShowLinkDialog(true));
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

      <GoogleAuthDialog
        open={auth.showAuthFlow}
        isAuthenticating={auth.isAuthenticating}
        authError={auth.authError}
        onCancel={auth.cancelAuthFlow}
      />
    </>
  );
}
