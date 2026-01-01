import { useState } from "react";
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

interface SyncSelectedButtonProps {
  books: BookInfo[];
  onBookUpdated: () => void;
}

export function SyncSelectedButton({ onBookUpdated }: SyncSelectedButtonProps) {
  const { t } = useTranslation();
  const [isSyncing, setIsSyncing] = useState(false);

  // Filter to books that have Google Docs linked
  const { targetBooks: linkedBooks, isCurrentBookMode } = useActionTarget(
    (book) => book.google_docs_sync_info != null
  );

  const handleSyncSelected = async () => {
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

  const isDisabled = isSyncing || linkedBooks.length === 0;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="outline"
            size="lg"
            onClick={handleSyncSelected}
            disabled={isDisabled}
            className="min-w-max"
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
  );
}
