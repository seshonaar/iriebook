import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { BookUp, Upload, RefreshCw } from "lucide-react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Switch } from "./ui/switch";
import { useAppContext } from "../contexts/AppContext";
import {
  setBooks,
  setError,
  setProcessing,
  setProcessingMode,
  clearLog,
  setPublicationOptions,
} from "../contexts/actions";
import { SyncSelectedButton } from "./SyncSelectedButton";
import { useActionTarget } from "../hooks/useActionTarget";
import { useGitOperations } from "../hooks/useGitOperations";

export function ProcessingPanel() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();
  const { targetBooks } = useActionTarget();
  const {
    commitMessage,
    setCommitMessage,
    showCommitDialog,
    isOperationInProgress,
    canSave,
    handleSave,
    handleGetLatest,
    openCommitDialog,
    closeCommitDialog,
  } = useGitOperations();

  const canProcess =
    targetBooks.length > 0 &&
    !state.isProcessing &&
    (state.publicationOptions.epub ||
      state.publicationOptions.pdf ||
      state.publicationOptions.azw3);

  const updatePublicationOptions = (
    next: Partial<typeof state.publicationOptions>
  ) => {
    const merged = { ...state.publicationOptions, ...next };
    if (merged.azw3) {
      merged.epub = true;
    }
    dispatch(setPublicationOptions(merged));
    commands.setPublicationOptions(merged).catch((error) => {
      console.error("Failed to sync publication options to backend:", error);
    });
  };

  const handleBookUpdated = async () => {
    if (state.selectedFolder) {
      try {
        const result = await commands.scanBooks(state.selectedFolder);
        if (result.status === "ok") {
          dispatch(setBooks(result.data));
        }
      } catch (err) {
        console.error("Failed to rescan books:", err);
      }
    }
  };

  const handleStartProcessing = async (publish: boolean, stats: boolean) => {
    if (!canProcess) return;

    try {
      // Clear previous log
      dispatch(clearLog());
      dispatch(setProcessing(true));
      dispatch(setProcessingMode(publish ? "publish" : "analyze"));
      dispatch(setError(null));

      // Start processing (returns immediately, events come via useProcessingEvents)
      const result = await commands.startProcessing(
        targetBooks,
        publish,
        stats
      );
      if (result.status === "error") {
        throw new Error(result.error);
      }
    } catch (error) {
      console.error("Failed to start processing:", error);
      dispatch(setError(`Failed to start processing: ${error}`));
      dispatch(setProcessing(false));
      dispatch(setProcessingMode(null));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-3">
        <SyncSelectedButton
          books={state.books}
          onBookUpdated={handleBookUpdated}
        />

        <Button
          onClick={openCommitDialog}
          disabled={isOperationInProgress || showCommitDialog || !canSave}
          variant="outline"
          className="min-w-max"
          size="lg"
        >
          <Upload />
          <span className="translate-y-px">{t('git.sync.actions.saveToCloud')}</span>
        </Button>

        <Button
          onClick={handleGetLatest}
          disabled={isOperationInProgress || !canSave}
          variant="outline"
          className="min-w-max"
          size="lg"
        >
          <RefreshCw className={isOperationInProgress ? 'animate-spin' : ''} />
          <span className="translate-y-px">{t('git.sync.actions.sync')}</span>
        </Button>

        <Button
          size="lg"
          onClick={() => handleStartProcessing(true, false)}
          disabled={!canProcess}
          className="min-w-max"
        >
          <BookUp />
          <span className="translate-y-px">{state.isProcessing && state.processingMode === 'publish'
            ? t('processing.panel.button.processing')
            : t('processing.panel.button.publish')}</span>
        </Button>

      </div>

      <div className="flex flex-wrap gap-3">
        <Button
          type="button"
          size="default"
          variant={state.publicationOptions.epub ? "default" : "outline"}
          disabled={state.isProcessing || state.publicationOptions.azw3}
          className="min-w-[92px]"
          onClick={() => updatePublicationOptions({ epub: !state.publicationOptions.epub })}
        >
          EPUB
        </Button>
        <Button
          type="button"
          size="default"
          variant={state.publicationOptions.pdf ? "default" : "outline"}
          disabled={state.isProcessing}
          className="min-w-[92px]"
          onClick={() => updatePublicationOptions({ pdf: !state.publicationOptions.pdf })}
        >
          PDF
        </Button>
        <Button
          type="button"
          size="default"
          variant={state.publicationOptions.azw3 ? "default" : "outline"}
          disabled={state.isProcessing}
          className="min-w-[92px]"
          onClick={() => updatePublicationOptions({ azw3: !state.publicationOptions.azw3 })}
        >
          AZW3
        </Button>

        <div className="flex h-9 min-w-max items-center gap-3 rounded-md border border-input bg-background px-4 shadow-sm">
          <Switch
            id="embed-cover"
            checked={state.publicationOptions.embed_cover}
            onCheckedChange={(checked) =>
              updatePublicationOptions({ embed_cover: checked })
            }
            disabled={state.isProcessing}
          />
          <Label htmlFor="embed-cover" className="cursor-pointer whitespace-nowrap">
            {t('processing.panel.options.embedCover')}
          </Label>
        </div>
      </div>

      {/* Commit Dialog */}
      {showCommitDialog && (
        <div className="p-3 bg-muted/30 border border-border rounded-lg space-y-3 animate-in fade-in slide-in-from-top-2">
          <Input
            type="text"
            value={commitMessage}
            onChange={(e) => setCommitMessage(e.target.value)}
            placeholder={t('git.sync.actions.enterRevision')}
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter' && commitMessage.trim() && !isOperationInProgress) {
                handleSave();
              }
              if (e.key === 'Escape') {
                closeCommitDialog();
              }
            }}
          />
          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              onClick={closeCommitDialog}
              disabled={isOperationInProgress}
              size="sm"
            >
              {t('common.actions.cancel')}
            </Button>
            <Button
              onClick={handleSave}
              disabled={!commitMessage.trim() || isOperationInProgress}
              size="sm"
            >
              {t('git.sync.actions.saveToCloudGithub')}
            </Button>
          </div>
        </div>
      )}

      {state.processingProgress && (
        <div className="text-sm">
          <div className="flex items-center justify-between mb-1">
            <span className="text-muted-foreground">{t('processing.panel.progress.currentBook')}</span>
            <span className="font-medium">
              {state.processingProgress.currentBookName}
            </span>
          </div>
          <div className="w-full bg-secondary rounded-full h-2">
            <div
              className="bg-primary h-2 rounded-full transition-all duration-300"
              style={{
                width: `${((state.processingProgress.currentBookIndex + 1) / targetBooks.length) * 100}%`,
              }}
            />
          </div>
          <p className="text-xs text-muted-foreground mt-1 text-center">
            {t('processing.panel.progress.bookCount', {
              current: state.processingProgress.currentBookIndex + 1,
              total: targetBooks.length
            })}
          </p>
        </div>
      )}
    </div>
  );
}
