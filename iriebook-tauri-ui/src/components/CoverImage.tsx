import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useAppContext } from "../contexts/AppContext";
import { setCoverStatus } from "../contexts/actions";
import { toast } from "sonner";

interface CoverImageProps {
  bookPath: string;
  coverImagePath: string | null;
}

export function CoverImage({
  bookPath,
  coverImagePath,
}: CoverImageProps) {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();

  // Get the cover status from context
  const status = coverImagePath ? state.coverStatus[coverImagePath] : undefined;

  // Trigger initial load if not started
  useEffect(() => {
    if (!coverImagePath) return;
    if (status) return; // Already have a status (loading, ready, or error)

    // Start loading
    dispatch(setCoverStatus(coverImagePath, { type: "loading" }));

    commands.loadCoverImage(coverImagePath).then((result) => {
      if (result.status === "ok") {
        dispatch(setCoverStatus(coverImagePath, result.data));
      } else {
        dispatch(setCoverStatus(coverImagePath, { type: "error", message: result.error }));
      }
    });
  }, [coverImagePath, status, dispatch]);

  const handleClick = async () => {
    try {
      const selectResult = await commands.selectFile(
        t('books.viewer.selectCoverTitle'),
        [["Images", ["jpg", "jpeg", "png", "gif", "webp"]]]
      );
      if (selectResult.status === "error") {
        throw new Error(selectResult.error);
      }
      const newCoverPath = selectResult.data;

      if (!newCoverPath) {
        return;
      }

      const loadingToast = toast.loading(t('toasts.info.replacingCover'));

      try {
        const result = await commands.replaceCoverImage(bookPath, newCoverPath);
        if (result.status === "error") {
          throw new Error(result.error);
        }

        toast.dismiss(loadingToast);
        toast.success(t('toasts.success.coverReplaced'));

        // Invalidate the cover to trigger reload
        if (coverImagePath) {
          dispatch(setCoverStatus(coverImagePath, { type: "not_started" }));
        }
      } catch (err) {
        toast.dismiss(loadingToast);
        throw err;
      }
    } catch (error) {
      console.error("Failed to replace cover:", error);
      toast.error(t('errors.operations.replaceCover'), {
        description: String(error),
      });
    }
  };

  // Determine loading state
  const isLoading = !status || status.type === "loading" || status.type === "not_started";
  const hasError = status?.type === "error";
  const isReady = status?.type === "ready";
  const dataUrl = isReady ? status.data_url : null;
  const errorMessage = hasError ? status.message : (coverImagePath ? null : t('books.viewer.noCover'));

  return (
    <div className="flex flex-col items-center gap-2">
      <div
        className="relative w-[200px] h-[300px] border-2 border-border rounded-lg overflow-hidden cursor-pointer hover:border-primary transition-colors bg-muted"
        onClick={handleClick}
        title={t('books.viewer.replaceCover')}
      >
        {isLoading && coverImagePath && (
          <div className="absolute inset-0 flex items-center justify-center bg-muted">
            <div className="text-muted-foreground">{t('common.labels.loading')}</div>
          </div>
        )}
        {(errorMessage || !coverImagePath) && !isLoading && !dataUrl && (
          <div className="absolute inset-0 flex flex-col items-center justify-center bg-muted p-4">
            <div className="text-muted-foreground text-center text-sm">
              {t('books.viewer.noCoverImage')}
            </div>
            <div className="text-xs text-muted-foreground mt-2">
              {t('books.viewer.clickToAdd')}
            </div>
          </div>
        )}
        {dataUrl && (
          <img
            src={dataUrl}
            alt={t('books.viewer.coverAlt')}
            className="w-full h-full object-contain"
          />
        )}
      </div>
      <p className="text-sm text-muted-foreground">{t('books.viewer.replaceCover')}</p>
    </div>
  );
}
