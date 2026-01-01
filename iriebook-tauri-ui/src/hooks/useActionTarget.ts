import { useMemo } from "react";
import { useAppContext } from "../contexts/AppContext";
import type { BookInfo } from "../bindings";

interface ActionTargetResult {
  /** Books to apply actions to */
  targetBooks: BookInfo[];
  /** Whether we're in current book mode */
  isCurrentBookMode: boolean;
  /** The currently viewed book (if any) */
  viewedBook: BookInfo | null;
  /** Status message for display */
  statusMessage: {
    count: number;
    bookName?: string;
  };
}

/**
 * Hook that returns the books to use for actions based on current mode.
 *
 * - If `currentBookMode` is ON: Returns the currently viewed book (or empty if none)
 * - If `currentBookMode` is OFF: Returns selected books (checked)
 *
 * Optionally filters by a predicate (e.g., only books with Google Docs linked)
 */
export function useActionTarget(
  filterFn?: (book: BookInfo) => boolean
): ActionTargetResult {
  const { state } = useAppContext();

  return useMemo(() => {
    const viewedBook = state.viewedBookIndex !== null
      ? state.books[state.viewedBookIndex] ?? null
      : null;

    let targetBooks: BookInfo[];

    if (state.currentBookMode) {
      // Current Book Mode: only the viewed book
      targetBooks = viewedBook ? [viewedBook] : [];
    } else {
      // Multi-select Mode: selected books
      targetBooks = state.books.filter((b) => b.selected);
    }

    // Apply optional filter
    if (filterFn) {
      targetBooks = targetBooks.filter(filterFn);
    }

    return {
      targetBooks,
      isCurrentBookMode: state.currentBookMode,
      viewedBook,
      statusMessage: {
        count: targetBooks.length,
        bookName: state.currentBookMode && viewedBook ? viewedBook.display_name : undefined,
      },
    };
  }, [state.books, state.viewedBookIndex, state.currentBookMode, filterFn]);
}
