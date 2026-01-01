import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useAppContext } from "../contexts/AppContext";
import {
  setLoading,
  setSession,
  setCurrentBookMode,
  setBooks,
  setViewedBook,
  setError,
} from "../contexts/actions";
import { commands, type SessionData, type BookPath } from "../bindings";
import { sortBooks } from "../lib/utils";

/**
 * Hook to manage session persistence
 * Automatically saves session whenever relevant state changes
 */
export function useSession() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();

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
      }
    }

    loadInitialSession();
  }, [dispatch, t]);

  // Auto-save session whenever relevant state changes
  useEffect(() => {
    // Only save if we have a valid session
    if (!state.selectedFolder) {
      return;
    }

    const sessionData: SessionData = {
      folder_path: state.selectedFolder as any, // FolderPath type
      selected_book_paths: state.books
        .filter((book) => book.selected)
        .map((book) => book.path) as BookPath[],
      current_book_mode: state.currentBookMode,
    };

    // Debounce save to avoid too many writes
    const timeoutId = setTimeout(async () => {
      const result = await commands.saveSession(sessionData);
      if (result.status === "error") {
        console.error("Failed to save session:", result.error);
      }
    }, 500);

    return () => clearTimeout(timeoutId);
  }, [
    state.selectedFolder,
    state.books,
    state.currentBookMode,
  ]);
}
