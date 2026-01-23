import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useCoverImage } from "../hooks/useCoverImage";
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
  const { loadCover, getCachedCover, isLoadingCover } =
    useCoverImage();
  const [dataUrl, setDataUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Load cover on mount or when coverImagePath changes
  useEffect(() => {
    const loadCoverData = async () => {
      if (!coverImagePath) {
        setError(t('books.viewer.noCover'));
        setDataUrl(null);
        return;
      }

      const cachedCover = getCachedCover(coverImagePath);
      if (cachedCover) {
        setDataUrl(cachedCover);
        setError(null);
      } else {
        const url = await loadCover(coverImagePath);
        if (url) {
          setDataUrl(url);
          setError(null);
        } else {
          setError(t('books.viewer.coverNotFound'));
        }
      }
    };

    loadCoverData();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [coverImagePath]);

  const handleClick = async () => {
    try {
      // Open file dialog to select new cover
      const selectResult = await commands.selectFile(
        "Select Cover Image",
        [["Images", ["jpg", "jpeg", "png", "gif", "webp"]]]
      );
      if (selectResult.status === "error") {
        throw new Error(selectResult.error);
      }
      const newCoverPath = selectResult.data;

      if (!newCoverPath) {
        return; // User cancelled
      }

      // Show loading toast
      const loadingToast = toast.loading(t('toasts.info.replacingCover'));

      try {
        // Replace the cover
        const result = await commands.replaceCoverImage(bookPath, newCoverPath);
        if (result.status === "error") {
          throw new Error(result.error);
        }

        // Dismiss loading toast
        toast.dismiss(loadingToast);

        // Success toast
        // Note: Cover cache is automatically cleared via BookListChangedEvent
        toast.success(t('toasts.success.coverReplaced'));
      } catch (err) {
        toast.dismiss(loadingToast);
        throw err;
      }
    } catch (error) {
      console.error("Failed to replace cover:", error);
      toast.error(t('errors.operations.replaceCover'), {
        description: String(error),
      });
      setError(t('errors.operations.replaceCover'));
    }
  };

  const loading = coverImagePath ? isLoadingCover(coverImagePath) : false;

  return (
    <div className="flex flex-col items-center gap-2">
      <div
        className="relative w-[200px] h-[300px] border-2 border-border rounded-lg overflow-hidden cursor-pointer hover:border-primary transition-colors bg-muted"
        onClick={handleClick}
        title={t('books.viewer.replaceCover')}
      >
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-muted">
            <div className="text-muted-foreground">{t('common.labels.loading')}</div>
          </div>
        )}
        {error && !loading && !dataUrl && (
          <div className="absolute inset-0 flex flex-col items-center justify-center bg-muted p-4">
            <div className="text-muted-foreground text-center text-sm">
              {t('books.viewer.noCoverImage')}
            </div>
            <div className="text-xs text-muted-foreground mt-2">
              {t('books.viewer.clickToAdd')}
            </div>
          </div>
        )}
        {dataUrl && !loading && (
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
