import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { events, commands } from "../bindings";
import { useAppContext } from "../contexts/AppContext";
import { useCoverImage } from "./useCoverImage";
import {
  addLogEntry,
  setProcessingProgress,
  setProcessing,
  setProcessingMode,
  clearProcessingProgress,
  setBooks,
  setCoverStatus,
  clearCoverStatus,
} from "../contexts/actions";

/**
 * Hook to listen for processing events from the Tauri backend
 * Updates state in real-time as books are processed
 */
export function useProcessingEvents() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();
  const { clearCache } = useCoverImage();

  // Use refs to access latest state inside event listeners without re-subscribing
  const booksRef = useRef(state.books);
  const folderRef = useRef(state.selectedFolder);

  useEffect(() => {
    booksRef.current = state.books;
    folderRef.current = state.selectedFolder;
  }, [state.books, state.selectedFolder]);

  useEffect(() => {
    // Set up git progress listener
    const unlistenGitPromise = events.gitOperationProgressEvent.listen((event) => {
      dispatch(addLogEntry({
        message: event.payload,
        type: "info",
      }));
    });

    // Set up Google Docs progress listener
    const unlistenGoogleDocsPromise = events.googleDocsProgressEvent.listen((event) => {
      dispatch(addLogEntry({
        message: event.payload,
        type: "info",
      }));
    });

    // Set up processing event listener
    const unlistenPromise = events.processingUpdateEvent.listen((event) => {
      const payload = event.payload;

      if (payload.type === "started") {
        // Add log entry for started
        dispatch(addLogEntry({
          message: t('log.processing.bookStarted', { bookName: payload.book_name }),
          type: "info",
        }));
        dispatch(setProcessingProgress({
          currentBookIndex: payload.book_index,
          currentBookName: payload.book_name,
        }));
      } else if (payload.type === "completed") {
        // Add log entry for completed
        dispatch(addLogEntry({
          message: payload.success
            ? `✓ ${payload.message}`
            : `✗ ${payload.message}`,
          type: payload.success ? "success" : "error",
          outputPath: payload.output_path || undefined,
        }));
      } else if (payload.type === "all_done") {
        dispatch(setProcessing(false));
        dispatch(setProcessingMode(null));
        dispatch(clearProcessingProgress());
        dispatch(addLogEntry({
          message: "\n" + t('log.processing.allDone'),
          type: "success",
        }));
      }
    });

    // Set up update progress listener
    const unlistenUpdatePromise = events.updateProgressEvent.listen((event) => {
      const payload = event.payload;
      let message = "";
      let type: "info" | "success" | "error" = "info";

      switch (payload.type) {
        case "Checking":
          message = t('log.update.checking');
          break;
        case "NoUpdate":
          message = t('log.update.noUpdate');
          type = "success";
          break;
        case "UpdateAvailable":
          message = t('log.update.available', { version: payload.version });
          break;
        case "Downloading":
          message = t('log.update.downloading', { percent: payload.percent });
          break;
        case "Installing":
          message = t('log.update.installing');
          break;
        case "Done":
          message = t('log.update.done');
          type = "success";
          break;
        case "Error":
          message = t('log.update.error', { message: payload.message });
          type = "error";
          break;
      }

      dispatch(addLogEntry({ message, type }));
    });

    // Set up book list changed listener
    const unlistenBookListPromise = events.bookListChangedEvent.listen(async () => {
      // Clear cover cache to force reload with updated images
      clearCache();
      dispatch(clearCoverStatus());

      const currentFolder = folderRef.current;
      const currentBooks = booksRef.current;

      if (currentFolder) {
        // Capture currently selected paths to preserve selection
        const selectedPaths = new Set(
          currentBooks
            .filter((b) => b.selected)
            .map((b) => b.path)
        );

        try {
          const result = await commands.scanBooks(currentFolder);
          if (result.status === "ok") {
            // Restore selection
            const mergedBooks = result.data.map((book) => ({
              ...book,
              selected: selectedPaths.has(book.path),
            }));
            dispatch(setBooks(mergedBooks));
          }
        } catch (err) {
          console.error("Failed to rescan books:", err);
        }
      }
    });

    // Set up cover reload event listener
    const unlistenCoverReloadPromise = events.coverReloadEvent.listen(async (event) => {
      const { book_path: bookPath } = event.payload;

      // Load the cover and dispatch the result to context
      const result = await commands.loadCoverImage(bookPath);
      if (result.status === "ok") {
        dispatch(setCoverStatus(bookPath, result.data));
      } else {
        // Dispatch error status
        dispatch(setCoverStatus(bookPath, { type: "error", message: result.error }));
      }
    });

    // Cleanup on unmount
    return () => {
      unlistenGitPromise.then((unlisten) => unlisten());
      unlistenGoogleDocsPromise.then((unlisten) => unlisten());
      unlistenPromise.then((unlisten) => unlisten());
      unlistenBookListPromise.then((unlisten) => unlisten());
      unlistenUpdatePromise.then((unlisten) => unlisten());
      unlistenCoverReloadPromise.then((unlisten) => unlisten());
    };
  }, [dispatch, t, clearCache]);
}
