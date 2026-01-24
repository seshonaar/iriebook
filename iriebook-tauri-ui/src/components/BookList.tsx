import { useTranslation } from "react-i18next";
import { Checkbox } from "./ui/checkbox";
import { Label } from "./ui/label";
import { Switch } from "./ui/switch";
import { BookItem } from "./BookItem";
import { AddBookButton } from "./AddBookButton";
import { useAppContext } from "../contexts/AppContext";
import {
  setCurrentBookMode,
  toggleAllBooks,
  toggleBook,
  setViewedBook,
  setBooks,
  clearCoverStatus,
} from "../contexts/actions";
import { useMemo } from "react";
import { commands, type BookInfo, type AddBookResult } from "../bindings";
import { RefreshCw } from "lucide-react";
import { Button } from "./ui/button";
import { useActionTarget } from "../hooks/useActionTarget";

export function BookList() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();
  const { isCurrentBookMode } = useActionTarget();

  // Calculate if all books are selected
  const allSelected = useMemo(() => {
    if (state.books.length === 0) return false;
    return state.books.every((book) => book.selected);
  }, [state.books]);

  const handleToggleCurrentBookMode = () => {
    dispatch(setCurrentBookMode(!state.currentBookMode));
  };

  const handleToggleAll = () => {
    dispatch(toggleAllBooks(!allSelected));
  };

  const handleToggleBook = (index: number) => {
    dispatch(toggleBook(index));
  };

  const handleViewBook = (index: number) => {
    dispatch(setViewedBook(index));
  };

  const handleBookAdded = (result: AddBookResult) => {
    // Update book list
    dispatch(setBooks(result.books));

    // Auto-view the newly added book if index is available
    if (result.new_book_index !== null && result.new_book_index !== undefined) {
      dispatch(setViewedBook(result.new_book_index));
    }
  };

  const handleBookDeleted = (updatedBooks: BookInfo[]) => {
    // Update book list
    dispatch(setBooks(updatedBooks));

    // Clear viewed book since it might have been deleted
    dispatch(setViewedBook(null));
  };

  const handleBookUpdated = async () => {
    // Clear cover status in context to trigger fresh loads
    dispatch(clearCoverStatus());

    // Rescan books to pick up changes (e.g., Google Docs sync info)
    if (state.selectedFolder) {
      // Capture currently selected paths to preserve selection
      const selectedPaths = new Set(
        state.books
          .filter((b) => b.selected)
          .map((b) => b.path)
      );

      try {
        const result = await commands.scanBooks(state.selectedFolder);
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
  };

  if (state.books.length === 0) {
    return (
      <div className="bg-card border border-border rounded-lg p-8 text-center">
        <p className="text-muted-foreground">
          {t('books.list.noBooks')}
        </p>
      </div>
    );
  }

  return (
    <div className="bg-card border border-border rounded-lg p-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">
          {t('books.list.count', { count: state.books.length })}
        </h3>
        <div className="flex items-center gap-3">
          <AddBookButton
            workspaceRoot={state.selectedFolder}
            onBookAdded={handleBookAdded}
          />
          {/* Mode Toggle */}
          <div className="flex items-center gap-2 border-l border-border pl-3">
            <Switch
              id="current-book-mode"
              checked={isCurrentBookMode}
              onCheckedChange={handleToggleCurrentBookMode}
              data-testid="current-book-mode-toggle"
            />
            <Label htmlFor="current-book-mode" className="cursor-pointer text-sm">
              {isCurrentBookMode
                ? t('books.list.currentBookMode')
                : t('books.list.multiSelectMode')}
            </Label>
          </div>
          {/* Select All (only in multi-select mode) */}
          {!isCurrentBookMode && (
            <div className="flex items-center gap-2 border-l border-border pl-3">
              <Checkbox
                id="select-all"
                checked={allSelected}
                onCheckedChange={handleToggleAll}
              />
              <Label htmlFor="select-all" className="cursor-pointer text-sm">
                {t('books.list.selectAll')}
              </Label>
            </div>
          )}
          <Button
            variant="ghost"
            size="icon"
            onClick={handleBookUpdated}
            title={t('books.list.refresh')}
            className="h-8 w-8"
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
      </div>

      <ul className="space-y-2">
        {state.books.map((book, index) => (
          <BookItem
            key={book.path}
            book={book}
            index={index}
            workspaceRoot={state.selectedFolder}
            isViewed={state.viewedBookIndex === index}
            hideCheckbox={isCurrentBookMode}
            onToggle={handleToggleBook}
            onView={handleViewBook}
            onBookDeleted={handleBookDeleted}
            onBookUpdated={handleBookUpdated}
          />
        ))}
      </ul>

      <div className="mt-4 pt-4 border-t border-border">
        <p className="text-sm text-muted-foreground">
          {isCurrentBookMode
            ? (state.viewedBookIndex !== null && state.books[state.viewedBookIndex]
                ? t('books.list.currentBookStatus', { name: state.books[state.viewedBookIndex].display_name })
                : t('books.list.noBookViewed'))
            : t('books.list.selectedCount', {
                count: state.books.filter((b) => b.selected).length
              })}
        </p>
      </div>
    </div>
  );
}
