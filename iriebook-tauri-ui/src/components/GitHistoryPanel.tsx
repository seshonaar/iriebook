import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useAppContext } from "../contexts/AppContext";
import { setCommitHistory, openDiffTab } from "../contexts/actions";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { RefreshCw, History, FileText, Loader2 } from "lucide-react";

export function GitHistoryPanel() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();
  const [hoveredCommit, setHoveredCommit] = useState<string | null>(null);
  const [loadingCommit, setLoadingCommit] = useState<string | null>(null);

  // Load commit history when folder changes or after git operations
  useEffect(() => {
    if (state.selectedFolder && state.gitSyncStatus.status !== "Uninitialized") {
      loadCommitHistory();
    }
  }, [state.selectedFolder, state.gitSyncStatus]);

  const loadCommitHistory = async () => {
    if (!state.selectedFolder) return;

    try {
      const result = await commands.gitGetLog(state.selectedFolder, 10);
      if (result.status === "error") {
        throw new Error(result.error);
      }

      dispatch(setCommitHistory(result.data));
    } catch (err) {
      console.error("Failed to load commit history:", err);
    }
  };

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(Number(timestamp) * 1000);
    return date.toLocaleDateString() + " " + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const formatCommitHash = (hash: string) => {
    return hash.substring(0, 7);
  };

  const handleShowChanges = async (commit: any) => {
    if (!state.selectedFolder) return;

    try {
      setLoadingCommit(commit.hash);
      // Single command gets all diffs for .md files
      const result = await (commands as any).gitGetRevisionDiffs(
        state.selectedFolder,
        commit.hash
      );

      if (result.status === "error") {
        console.error("Failed to get revision diffs:", result.error);
        return;
      }

      // Open a tab for each diff
      result.data.forEach((revisionDiff: any) => {
        const fileName = revisionDiff.file_path.split('/').pop() || revisionDiff.file_path;
        dispatch(openDiffTab({
          commitHash: commit.hash,
          filePath: revisionDiff.file_path,
          title: fileName,
          diffData: revisionDiff.comparison,
        }));
      });

      // Show message if no .md files changed
      if (result.data.length === 0) {
        console.log("No manuscript files changed in this commit");
      }
    } catch (err) {
      console.error("Failed to get revision diffs:", err);
    } finally {
      setLoadingCommit(null);
    }
  };

  if (state.gitSyncStatus.status === "Uninitialized") {
    return null;
  }

  return (
    <div className="flex flex-col h-full space-y-2">
      <div className="flex justify-end px-1">
        <Button 
          variant="ghost" 
          size="icon" 
          onClick={loadCommitHistory} 
          title={t('git.history.refresh')} 
          className="h-7 w-7"
        >
          <RefreshCw className="h-3.5 w-3.5" />
        </Button>
      </div>

      {state.commitHistory.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground border border-dashed border-border rounded-lg bg-muted/20">
            <History className="mx-auto h-8 w-8 opacity-20 mb-2" />
            <p className="text-sm">{t('git.history.noHistory')}</p>
        </div>
      ) : (
        <ScrollArea className="flex-1">
          <div className="flex flex-col gap-1">
            {state.commitHistory.map((commit) => (
              <div
                key={commit.hash}
                className="px-2 py-1.5 bg-muted/20 rounded-md border-l-2 border-primary/40 text-sm hover:bg-muted/40 transition-colors relative group cursor-default select-none"
                onMouseEnter={() => setHoveredCommit(commit.hash)}
                onMouseLeave={() => setHoveredCommit(null)}
                onDoubleClick={() => !loadingCommit && handleShowChanges(commit)}
              >
                <div className="flex justify-between items-center gap-4">
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    <span className="font-mono text-xs text-muted-foreground opacity-60 shrink-0">
                      {formatCommitHash(commit.hash)}
                    </span>
                    <p className="font-medium text-foreground truncate" title={commit.message}>
                      {commit.message}
                    </p>
                  </div>
                  
                  <div className="flex items-center gap-2 shrink-0">
                    {(hoveredCommit === commit.hash || loadingCommit === commit.hash) && (
                      <Button
                        variant="secondary"
                        size="sm"
                        className="h-6 px-2 text-xs bg-background/50 hover:bg-background"
                        onClick={() => handleShowChanges(commit)}
                        disabled={loadingCommit !== null}
                      >
                        {loadingCommit === commit.hash ? (
                          <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                        ) : (
                          <FileText className="h-3 w-3 mr-1" />
                        )}
                        {t('git.history.showChanges')}
                      </Button>
                    )}
                    <span className="text-xs text-muted-foreground opacity-60">
                      {formatTimestamp(commit.timestamp)}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

