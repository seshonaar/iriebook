import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useAppContext } from "../contexts/AppContext";
import {
  setLoading,
  setSession,
  setCurrentBookMode,
  setPublicationOptions,
  setBooks,
  setViewedBook,
  setError,
} from "../contexts/actions";
import { commands, type SessionData, type BookPath } from "../bindings";
import { sortBooks } from "../lib/utils";

// Default debounce delay for session save (can be overridden for testing)
export const SESSION_SAVE_DEBOUNCE_MS = 500;

/**
 * Hook to manage session persistence
 * Automatically saves session whenever relevant state changes
 */
export function useSession() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();

  // Track whether initial load has completed (to avoid saving during load)
  const isInitialLoadRef = useRef(true);
  // Track the save timeout for cleanup
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load session on mount
  useEffect(() => {
    async function loadInitialSession() {
      try {
        dispatch(setLoading(true));
        const sessionResult = await commands.loadSession();

        if (sessionResult.status === "error") {
          throw new Error(sessionResult.error);
        }

        const session = sessionResult.data;

        if (session) {
          dispatch(setSession(session));

          // Restore current book mode preference
          dispatch(setCurrentBookMode(session.current_book_mode as boolean));
          dispatch(setPublicationOptions(session.publication_options ?? {
            embed_cover: true,
            epub: true,
            pdf: true,
            azw3: true,
          }));

          // If we have a folder, scan for books
          if (session.folder_path) {
            const booksResult = await commands.scanBooks(session.folder_path);

            if (booksResult.status === "error") {
              throw new Error(booksResult.error);
            }

            // Restore selection state
            let books = booksResult.data.map((book) => ({
              ...book,
              selected: session.selected_book_paths.includes(book.path),
            }));

            // Sort locally to determine correct index
            books = sortBooks(books);

            dispatch(setBooks(books));

            // Auto-view logic: Prefer first selected book, otherwise first book
            if (books.length > 0) {
              const firstSelectedIndex = books.findIndex((b) => b.selected);
              const targetIndex = firstSelectedIndex >= 0 ? firstSelectedIndex : 0;
              dispatch(setViewedBook(targetIndex));
            }
          }
        }
      } catch (error) {
        console.error(t('errors.operations.loadSession'), error);
        dispatch(setError(`${t('errors.operations.loadSession')}: ${error}`));
      } finally {
        dispatch(setLoading(false));
        // Mark initial load as complete after a tick to let React batch updates
        setTimeout(() => {
          isInitialLoadRef.current = false;
        }, 0);
      }
    }

    loadInitialSession();
  }, [dispatch, t]);

  // Auto-save session whenever relevant state changes
  useEffect(() => {
    // Don't save during initial load or if no folder selected
    if (isInitialLoadRef.current || !state.selectedFolder) {
      return;
    }

    // Clear any pending save
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }

    const sessionData: SessionData = {
      folder_path: state.selectedFolder as any, // FolderPath type
      selected_book_paths: state.books
        .filter((book) => book.selected)
        .map((book) => book.path) as BookPath[],
      current_book_mode: state.currentBookMode,
      publication_options: state.publicationOptions,
    };

    // Debounce save to avoid too many writes
    saveTimeoutRef.current = setTimeout(async () => {
      const result = await commands.saveSession(sessionData);
      if (result.status === "error") {
        console.error("Failed to save session:", result.error);
      }
    }, SESSION_SAVE_DEBOUNCE_MS);

    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, [
    state.selectedFolder,
    state.books,
    state.currentBookMode,
    state.publicationOptions,
  ]);
}
