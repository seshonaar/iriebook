import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useAppContext } from "../contexts/AppContext";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { FileText, FolderOpen } from "lucide-react";
import { commands } from "../bindings";

export function StatusBar() {
  const { t } = useTranslation();
  const { state } = useAppContext();
  const [dialogOpen, setDialogOpen] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when dialog opens or new log entries are added
  useEffect(() => {
    if (dialogOpen && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [dialogOpen, state.resultsLog]);

  const lastEntry = state.resultsLog.length > 0
    ? state.resultsLog[state.resultsLog.length - 1]
    : null;

  const handleOpenFolder = async (filePath: string) => {
    try {
      const lastSlash = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
      const folderPath = lastSlash > 0 ? filePath.substring(0, lastSlash) : filePath;
      await commands.openFolder(folderPath);
    } catch (error) {
      console.error("Failed to open folder:", error);
    }
  };

  const getEntryColorClass = (type: string) => {
    switch (type) {
      case "success":
        return "text-green-600 dark:text-green-400";
      case "error":
        return "text-red-600 dark:text-red-400";
      default:
        return "text-foreground";
    }
  };

  return (
    <>
      <div className="shrink-0 flex items-center h-8 bg-secondary/30 border-t border-border px-3 gap-2">
        {/* Last log message */}
        <div className="flex-1 min-w-0">
          {lastEntry ? (
            <span
              className={`text-sm truncate block ${getEntryColorClass(lastEntry.type)}`}
            >
              {lastEntry.message}
            </span>
          ) : (
            <span className="text-sm text-muted-foreground">
              {t("log.statusBar.noActivity")}
            </span>
          )}
        </div>

        {/* Show log button */}
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2 text-xs gap-1"
          onClick={() => setDialogOpen(true)}
        >
          <FileText className="h-3.5 w-3.5" />
          {t("log.statusBar.showLog")}
        </Button>
      </div>

      {/* Results Log Dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] flex flex-col">
          <DialogHeader>
            <DialogTitle>{t("log.title")}</DialogTitle>
          </DialogHeader>

          {state.resultsLog.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-8">
              {t("log.empty")}
            </p>
          ) : (
            <ScrollArea className="flex-1 min-h-[300px] max-h-[60vh] w-full rounded-md border p-4" ref={scrollRef}>
              <div className="space-y-4 font-mono text-sm">
                {state.resultsLog.map((entry, index) => (
                  <div key={index} className="flex flex-col gap-1">
                    <div
                      className={`whitespace-pre-wrap ${getEntryColorClass(entry.type)}`}
                    >
                      {entry.message}
                    </div>

                    {entry.outputPath && (
                      <div className="pl-4">
                        <Button
                          variant="outline"
                          size="sm"
                          className="h-7 text-xs gap-1.5"
                          onClick={() => handleOpenFolder(entry.outputPath!)}
                        >
                          <FolderOpen className="h-3.5 w-3.5" />
                          {t("log.openOutputFolder")}
                        </Button>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
