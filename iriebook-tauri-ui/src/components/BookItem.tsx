import { useState } from "react";
import { useTranslation } from "react-i18next";
import { MoreVertical, Trash, Unlink, PencilLine } from "lucide-react";
import { Checkbox } from "./ui/checkbox";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";
import { GoogleDocsSyncButton } from "./GoogleDocsSyncButton";
import { commands, type BookInfo } from "../bindings";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { ConfirmDialog } from "./ConfirmDialog";
import { useAppContext } from "../contexts/AppContext";
import { openDiffTab } from "../contexts/actions";

interface BookItemProps {
  book: BookInfo;
  index: number;
  workspaceRoot: string | null;
  isViewed: boolean;
  hideCheckbox?: boolean;
  onToggle: (index: number) => void;
  onView: (index: number) => void;
  onBookDeleted: (updatedBooks: BookInfo[]) => void;
  onBookUpdated: () => void;
}

export function BookItem({
  book,
  index,
  workspaceRoot,
  isViewed,
  hideCheckbox = false,
  onToggle,
  onView,
  onBookDeleted,
  onBookUpdated,
}: BookItemProps) {
  const { t } = useTranslation();
  const { dispatch } = useAppContext();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [loadingDiffs, setLoadingDiffs] = useState(false);

  // Check if book is linked to Google Docs
  const isLinked = book.google_docs_sync_info != null;

  const handleDelete = async () => {
    if (!workspaceRoot) return;
    setIsDeleting(true);
    try {
      const result = await commands.deleteBook(book.path, workspaceRoot);
      if (result.status === "error") {
        throw new Error(result.error);
      }
      onBookDeleted(result.data);
    } catch (error) {
      console.error("Failed to delete book:", error);
    } finally {
      setIsDeleting(false);
    }
  };

  const handleUnlink = async () => {
    try {
      const result = await commands.googleUnlinkDoc(book.path);
      if (result.status === "ok") {
        onBookUpdated();
      } else {
        console.error(t("google.sync.messages.unlinkFailed"), result.error);
      }
    } catch (err) {
      console.error(t("google.sync.messages.unlinkFailed"), err);
    }
  };

  const handleShowLocalDiffs = async () => {
    if (!workspaceRoot) return;

    try {
      setLoadingDiffs(true);
      const result = await (commands as any).gitGetLocalDiffs(workspaceRoot);

      if (result.status === "error") {
        console.error("Failed to get local diffs:", result.error);
        return;
      }

      // Open a tab for each diff
      result.data.forEach((revisionDiff: any) => {
        const fileName = revisionDiff.file_path.split('/').pop() || revisionDiff.file_path;
        dispatch(openDiffTab({
          commitHash: "local",
          filePath: revisionDiff.file_path,
          title: `${fileName} (Local)`,
          diffData: revisionDiff.comparison,
        }));
      });

      // Show message if no .md files changed
      if (result.data.length === 0) {
        console.log("No manuscript files changed");
      }
    } catch (err) {
      console.error("Failed to get local diffs:", err);
    } finally {
      setLoadingDiffs(false);
    }
  };

  return (
    <li
      className={`flex items-center gap-3 p-3 rounded-md transition-colors group ${isViewed ? "bg-accent" : "hover:bg-accent/50"
        }`}
      data-testid="book-item"
      data-book-path={book.path}
    >
      {!hideCheckbox && (
        <Checkbox
          id={`book-${index}`}
          checked={book.selected}
          onCheckedChange={() => onToggle(index)}
          onClick={(e) => e.stopPropagation()}
          className="flex-shrink-0"
        />
      )}
      <div
        className="flex-1 min-w-0 cursor-pointer"
        onClick={() => onView(index)}
      >
        <div className="font-medium break-words flex items-center gap-2">
          {book.display_name}
          {book.git_changed_files.length > 0 && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleShowLocalDiffs();
                    }}
                    className="inline-flex items-center cursor-pointer hover:bg-accent rounded p-0.5 transition-colors"
                    disabled={loadingDiffs}
                  >
                    <PencilLine className="h-3.5 w-3.5 text-amber-500" />
                  </button>
                </TooltipTrigger>
                <TooltipContent>
                  <div className="text-xs">
                    <div className="font-semibold mb-1">Modified files (click to view diff):</div>
                    <ul className="list-disc list-inside">
                      {book.git_changed_files.map((file) => (
                        <li key={file}>{file}</li>
                      ))}
                    </ul>
                  </div>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}
        </div>
        {book.metadata && (
          <div className="text-sm text-muted-foreground break-words">
            {book.metadata.title} by {book.metadata.author}
          </div>
        )}
      </div>
      <div className="flex-shrink-0 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <GoogleDocsSyncButton
          book={book}
          onBookUpdated={onBookUpdated}
          hideUnlink={true}
        />

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
              <MoreVertical className="h-4 w-4" />
              <span className="sr-only">Open menu</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {isLinked && (
              <DropdownMenuItem onClick={(e) => {
                e.stopPropagation();
                handleUnlink();
              }}>
                <Unlink className="mr-2 h-4 w-4" />
                <span>{t("google.sync.button.unlink")}</span>
              </DropdownMenuItem>
            )}
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                setShowDeleteConfirm(true);
              }}
              className="text-destructive focus:text-destructive"
            >
              <Trash className="mr-2 h-4 w-4" />
              <span>{t("common.actions.delete")}</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <ConfirmDialog
          open={showDeleteConfirm}
          onOpenChange={setShowDeleteConfirm}
          title={t('books.delete.confirmTitle')}
          description={t('books.delete.confirmDescription', { bookName: book.display_name })}
          onConfirm={handleDelete}
          onCancel={() => { }}
          confirmText={isDeleting ? t('books.delete.deleting') : t('common.actions.delete')}
          cancelText={t('common.actions.cancel')}
          variant="destructive"
        />
      </div>
    </li>
  );
}