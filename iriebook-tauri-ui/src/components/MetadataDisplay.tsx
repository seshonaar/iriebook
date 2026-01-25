import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands, type BookMetadata, type ChangeBookResult } from "../bindings";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "./ui/dropdown-menu";
import { MoreVertical, FileText, GitCompareArrows, FolderOpen } from "lucide-react";
import { useAppContext } from "../contexts/AppContext";
import { openDiffTab } from "../contexts/actions";
import { CoverImage } from "./CoverImage";

interface MetadataDisplayProps {
  bookPath: string;
  coverImagePath: string | null;
  workspaceRoot: string | null;
  metadata: BookMetadata;
  onReplaceCover: () => void;
  onEdit: () => void;
  onBookChanged?: (result: ChangeBookResult) => void;
}

export function MetadataDisplay({
  bookPath,
  coverImagePath,
  workspaceRoot,
  metadata,
  onReplaceCover,
  onEdit,
  onBookChanged,
}: MetadataDisplayProps) {
  const { t } = useTranslation();
  const { dispatch } = useAppContext();
  const [isChanging, setIsChanging] = useState(false);
  const [isViewing, setIsViewing] = useState(false);

  const handleViewBook = async () => {
    try {
      setIsViewing(true);

      // Show loading toast
      const loadingToast = toast.loading(t('toasts.info.launchingViewer'));

      try {
        // Call view_book command
        const result = await commands.viewBook(bookPath);

        if (result.status === "error") {
          throw new Error(result.error);
        }

        // Dismiss loading toast
        toast.dismiss(loadingToast);

        // Success toast
        toast.success(t('toasts.success.viewerLaunched'));
      } catch (err) {
        toast.dismiss(loadingToast);
        throw err;
      }
    } catch (error) {
      console.error("Failed to view book:", error);

      // Check for specific error types
      const errorMsg = String(error);
      if (errorMsg.includes("metadata.yaml")) {
        toast.error(t('errors.operations.viewBook'), {
          description: "metadata.yaml not found. Please add book metadata first.",
        });
      } else if (errorMsg.includes("ebook-viewer")) {
        toast.error(t('errors.operations.viewBook'), {
          description: "ebook-viewer not found. Please install Calibre.",
        });
      } else {
        toast.error(t('errors.operations.viewBook'), {
          description: errorMsg,
        });
      }
    } finally {
      setIsViewing(false);
    }
  };

  const handleOpenFolder = async () => {
    try {
      const lastSlash = Math.max(bookPath.lastIndexOf("/"), bookPath.lastIndexOf("\\"));
      const folderPath = lastSlash > 0 ? bookPath.substring(0, lastSlash) : bookPath;
      await commands.openFolder(folderPath);
    } catch (error) {
      console.error("Failed to open folder:", error);
    }
  };

  const handleChangeBookFile = async () => {
    if (!workspaceRoot) {
      console.error("No workspace root selected");
      return;
    }

    setIsChanging(true);

    try {
      // Open file dialog to select new markdown file
      const selectResult = await commands.selectFile(
        "Select New Markdown File",
        [["Markdown Files", ["md", "MD"]]]
      );
      if (selectResult.status === "error") {
        throw new Error(selectResult.error);
      }
      const newSource = selectResult.data;

      if (!newSource) {
        setIsChanging(false);
        return; // User cancelled
      }

      // Change the book file
      const result = await commands.changeBookFile(
        bookPath,
        newSource,
        workspaceRoot
      );
      if (result.status === "error") {
        throw new Error(result.error);
      }

      // Notify parent component
      if (onBookChanged) {
        onBookChanged(result.data);
      }
    } catch (error) {
      console.error("Failed to change book file:", error);
    } finally {
      setIsChanging(false);
    }
  };

  const handleViewChanges = async () => {
    try {
      const result = await commands.getBookProcessingDiff(bookPath);
      if (result.status === "error") {
        toast.error(result.error);
        return;
      }

      // Open diff tab (dispatch to AppContext)
      // Use book file name for tab title
      const fileName = bookPath.split('/').pop() || 'book';

      dispatch(openDiffTab({
        commitHash: "processed",
        filePath: bookPath,
        title: `${fileName} (processed)`,
        diffData: result.data,
      }));
    } catch (err) {
      toast.error(`Failed to compare: ${err}`);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex w-full flex-wrap items-center gap-2 mb-4">
        <Button onClick={onReplaceCover} variant="outline" size="sm">
          {t('books.viewer.replaceCoverButton')}
        </Button>
        <Button onClick={handleOpenFolder} variant="outline" size="sm">
          <FolderOpen className="h-4 w-4 mr-1" />
          {t('common.actions.openFolder')}
        </Button>
        <Button
          onClick={handleViewBook}
          variant="outline"
          size="sm"
          disabled={isViewing}
        >
          {isViewing ? t('common.status.processing') : t('books.viewer.viewBook')}
        </Button>
        <Button onClick={onEdit} variant="outline" size="sm">
          {t('books.viewer.editMetadata')}
        </Button>

        {/* Three-dots dropdown menu */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" className="h-8 w-8 p-0 ml-auto">
              <MoreVertical className="h-4 w-4" />
              <span className="sr-only">{t('common.actions.more')}</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onClick={handleChangeBookFile}
              disabled={isChanging}
            >
              <FileText className="mr-2 h-4 w-4" />
              <span>
                {isChanging ? t('common.status.saving') : t('metadata.display.changeFile')}
              </span>
            </DropdownMenuItem>
            <DropdownMenuItem onClick={handleViewChanges}>
              <GitCompareArrows className="mr-2 h-4 w-4" />
              <span>{t('metadata.display.viewChanges')}</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <div className="flex flex-col gap-6 lg:flex-row">
        <div className="flex-shrink-0">
          <CoverImage
            coverImagePath={coverImagePath}
            onReplaceCover={onReplaceCover}
          />
        </div>
        <div className="space-y-3">
          <div>
            <label className="text-sm font-medium text-gray-600">{t('metadata.editor.fields.title')}</label>
            <p className="text-base mt-1">{metadata.title}</p>
          </div>

          <div>
            <label className="text-sm font-medium text-gray-600">{t('metadata.display.author')}</label>
            <p className="text-base mt-1">{metadata.author}</p>
          </div>

          {metadata["belongs-to-collection"] && (
            <div>
              <label className="text-sm font-medium text-gray-600">
                {t('metadata.editor.fields.collection')}
              </label>
              <p className="text-base mt-1">{metadata["belongs-to-collection"]}</p>
            </div>
          )}

          {metadata["group-position"] !== null &&
            metadata["group-position"] !== undefined && (
              <div>
                <label className="text-sm font-medium text-gray-600">
                  {t('metadata.editor.fields.position')}
                </label>
                <p className="text-base mt-1">{metadata["group-position"]}</p>
              </div>
            )}
        </div>
      </div>
    </div>
  );
}
