import { useTranslation } from "react-i18next";
import { Book } from "lucide-react";
import { cn } from "../../lib/utils";
import type { BookInfo } from "../../bindings";

interface BookListForAnalysisProps {
  books: BookInfo[];
  selectedPath: string | null;
  onSelectBook: (path: string) => void;
}

export function BookListForAnalysis({
  books,
  selectedPath,
  onSelectBook,
}: BookListForAnalysisProps) {
  const { t } = useTranslation();

  if (books.length === 0) {
    return (
      <div className="p-4 text-center text-muted-foreground">
        {t("analysis.bookList.noBooks")}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <h3 className="px-4 py-2 text-sm font-medium border-b">
        {t("analysis.bookList.title")}
      </h3>
      <div className="flex-1 overflow-y-auto">
        {books.map((book) => (
          <button
            key={book.path}
            onClick={() => onSelectBook(book.path)}
            className={cn(
              "w-full px-4 py-2 text-left text-sm flex items-center gap-2 hover:bg-accent transition-colors",
              selectedPath === book.path && "bg-accent"
            )}
          >
            <Book className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
            <span className="truncate" title={book.metadata?.title || book.display_name}>
              {book.metadata?.title || book.display_name}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
