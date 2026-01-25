import { useEffect, useRef, useState, type MouseEvent, type WheelEvent } from "react";
import { useTranslation } from "react-i18next";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Scan, ZoomIn, ZoomOut } from "lucide-react";
import { commands } from "../bindings";
import { useAppContext } from "../contexts/AppContext";
import { setCoverStatus } from "../contexts/actions";
import { Dialog, DialogContent } from "./ui/dialog";

interface CoverImageProps {
  coverImagePath: string | null;
  onReplaceCover: () => void;
}

export function CoverImage({
  coverImagePath,
  onReplaceCover,
}: CoverImageProps) {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();
  const [isPreviewOpen, setIsPreviewOpen] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [fitZoom, setFitZoom] = useState(1);
  const [isPanning, setIsPanning] = useState(false);
  const panStart = useRef<{ x: number; y: number; left: number; top: number } | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const imageRef = useRef<HTMLImageElement | null>(null);
  const naturalSize = useRef<{ width: number; height: number } | null>(null);

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

  // Determine loading state
  const isLoading = !status || status.type === "loading" || status.type === "not_started";
  const hasError = status?.type === "error";
  const isReady = status?.type === "ready";
  const dataUrl = isReady ? status.data_url : null;
  const errorMessage = hasError ? status.message : (coverImagePath ? null : t('books.viewer.noCover'));
  const previewSrc = coverImagePath ? convertFileSrc(coverImagePath) : null;
  const canZoomOut = zoom > 0.5;
  const canZoomIn = zoom < 3;
  const scaledWidth = naturalSize.current ? naturalSize.current.width * zoom : undefined;
  const scaledHeight = naturalSize.current ? naturalSize.current.height * zoom : undefined;

  const computeFitZoom = (naturalWidth: number, naturalHeight: number) => {
    const container = containerRef.current;
    if (!container) return 1;
    const availableWidth = container.clientWidth;
    const availableHeight = container.clientHeight;
    if (!availableWidth || !availableHeight) return 1;
    return Math.min(1, availableWidth / naturalWidth, availableHeight / naturalHeight);
  };

  const handleCoverClick = () => {
    if (!coverImagePath || hasError) {
      onReplaceCover();
      return;
    }

    setIsPreviewOpen(true);
  };

  const handleZoomIn = () => {
    setZoom((current) => Math.min(3, Number((current + 0.2).toFixed(2))));
  };

  const handleZoomOut = () => {
    setZoom((current) => Math.max(0.5, Number((current - 0.2).toFixed(2))));
  };

  const handleResetZoom = () => {
    setZoom(fitZoom);
  };

  const handleWheel = (event: WheelEvent<HTMLDivElement>) => {
    event.preventDefault();
    const delta = event.deltaY > 0 ? -0.1 : 0.1;
    setZoom((current) => {
      const next = current + delta;
      return Math.min(3, Math.max(0.5, Number(next.toFixed(2))));
    });
  };

  const handleMouseDown = (event: MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    if (!containerRef.current) return;
    setIsPanning(true);
    panStart.current = {
      x: event.clientX,
      y: event.clientY,
      left: containerRef.current.scrollLeft,
      top: containerRef.current.scrollTop,
    };
  };

  const handleMouseMove = (event: MouseEvent<HTMLDivElement>) => {
    if (!isPanning || !panStart.current || !containerRef.current) return;
    const deltaX = event.clientX - panStart.current.x;
    const deltaY = event.clientY - panStart.current.y;
    containerRef.current.scrollLeft = panStart.current.left - deltaX;
    containerRef.current.scrollTop = panStart.current.top - deltaY;
  };

  const handleMouseUp = () => {
    setIsPanning(false);
    panStart.current = null;
  };

  useEffect(() => {
    if (isPreviewOpen) {
      if (naturalSize.current) {
        const nextFitZoom = computeFitZoom(
          naturalSize.current.width,
          naturalSize.current.height
        );
        setFitZoom(nextFitZoom);
        setZoom(nextFitZoom);
      } else {
        setZoom(fitZoom);
      }
      setIsPanning(false);
      panStart.current = null;
    }
  }, [isPreviewOpen, coverImagePath, fitZoom]);

  useEffect(() => {
    if (!isPreviewOpen) return;
    setZoom(fitZoom);
  }, [fitZoom, isPreviewOpen]);

  return (
    <div className="flex flex-col items-center gap-2">
      <div
        className="relative w-[200px] h-[300px] border-2 border-border rounded-lg overflow-hidden cursor-pointer hover:border-primary transition-colors bg-muted"
        onClick={handleCoverClick}
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
      {previewSrc && (
        <Dialog open={isPreviewOpen} onOpenChange={setIsPreviewOpen}>
          <DialogContent className="w-[min(95vw,90vh)] max-w-[900px] max-h-[96vh] aspect-[2/3] p-0">
            <div className="absolute bottom-4 right-4 z-10 flex items-center gap-2">
              <button
                type="button"
                onClick={handleZoomOut}
                disabled={!canZoomOut}
                className="h-8 w-8 rounded-sm text-lg leading-none opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none disabled:opacity-40"
                aria-label={t('books.viewer.zoomOut')}
                title={t('books.viewer.zoomOut')}
              >
                <ZoomOut className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={handleResetZoom}
                className="h-8 w-8 rounded-sm text-lg leading-none opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none disabled:opacity-40"
                aria-label={t('books.viewer.zoomReset')}
                title={t('books.viewer.zoomReset')}
              >
                <Scan className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={handleZoomIn}
                disabled={!canZoomIn}
                className="h-8 w-8 rounded-sm text-lg leading-none opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none disabled:opacity-40"
                aria-label={t('books.viewer.zoomIn')}
                title={t('books.viewer.zoomIn')}
              >
                <ZoomIn className="h-4 w-4" />
              </button>
            </div>
            <div
              ref={containerRef}
              className={`relative h-full w-full overflow-auto ${isPanning ? "cursor-grabbing" : "cursor-grab"}`}
              onWheel={handleWheel}
              onMouseDown={handleMouseDown}
              onMouseMove={handleMouseMove}
              onMouseUp={handleMouseUp}
              onMouseLeave={handleMouseUp}
            >
              <div className="w-fit h-fit">
                <img
                  src={previewSrc}
                  ref={imageRef}
                  alt={t('books.viewer.coverAlt')}
                  className="block select-none max-w-none max-h-none"
                  draggable={false}
                  onLoad={(event) => {
                    const { naturalWidth, naturalHeight } = event.currentTarget;
                    if (!naturalWidth || !naturalHeight) return;
                    naturalSize.current = { width: naturalWidth, height: naturalHeight };
                    const nextFitZoom = computeFitZoom(naturalWidth, naturalHeight);
                    setFitZoom(nextFitZoom);
                    setZoom(nextFitZoom);
                  }}
                  style={{
                    width: scaledWidth ? `${scaledWidth}px` : undefined,
                    height: scaledHeight ? `${scaledHeight}px` : undefined,
                    maxWidth: "none",
                    maxHeight: "none",
                  }}
                />
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}
