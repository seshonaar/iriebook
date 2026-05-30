import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type AddBookResult, type GoogleDocInfo } from "../bindings";
import { Button } from "./ui/button";
import { ConfirmDialog } from "./ConfirmDialog";
import { GoogleDocPickerDialog } from "./GoogleDocPickerDialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { ChevronDown, Cloud, FilePlus } from "lucide-react";

interface AddBookButtonProps {
  workspaceRoot: string | null;
  onBookAdded: (result: AddBookResult) => void;
}

export function AddBookButton({
  workspaceRoot,
  onBookAdded,
}: AddBookButtonProps) {
  const { t } = useTranslation();
  const [isAdding, setIsAdding] = useState(false);
  const [showDuplicateDialog, setShowDuplicateDialog] = useState(false);
  const [showGoogleDocDialog, setShowGoogleDocDialog] = useState(false);
  const [pendingFile, setPendingFile] = useState<string | null>(null);
  const [duplicateName, setDuplicateName] = useState<string>("");

  const handleAddLocalBook = async () => {
    if (!workspaceRoot) {
      console.error("No workspace root selected");
      return;
    }

    setIsAdding(true);

    try {
      // Open file dialog to select markdown file
      const selectResult = await commands.selectFile(
        "Select Markdown File",
        [["Markdown Files", ["md", "MD"]]]
      );
      if (selectResult.status === "error") {
        throw new Error(selectResult.error);
      }
      const selectedFile = selectResult.data;

      if (!selectedFile) {
        setIsAdding(false);
        return; // User cancelled
      }

      // Extract filename from path
      const filename = selectedFile.split(/[/\\]/).pop() || "";

      // Check for duplicate
      const duplicateResult = await commands.checkDuplicate(
        workspaceRoot,
        filename
      );
      if (duplicateResult.status === "error") {
        throw new Error(duplicateResult.error);
      }
      const duplicate = duplicateResult.data;

      if (duplicate) {
        // Show confirmation dialog
        setDuplicateName(duplicate);
        setPendingFile(selectedFile);
        setShowDuplicateDialog(true);
        setIsAdding(false);
      } else {
        // No duplicate, proceed with adding
        await addBookImpl(selectedFile);
      }
    } catch (error) {
      console.error("Failed to add book:", error);
      setIsAdding(false);
    }
  };

  const handleAddGoogleDoc = async (doc: GoogleDocInfo) => {
    if (!workspaceRoot) return;

    setIsAdding(true);
    try {
      const result = await commands.googleAddBookFromDoc(
        workspaceRoot,
        doc.id,
        doc.name
      );
      if (result.status === "error") {
        throw new Error(result.error);
      }

      onBookAdded(result.data);
      setShowGoogleDocDialog(false);
    } catch (error) {
      console.error("Failed to add book from Google Docs:", error);
      throw error;
    } finally {
      setIsAdding(false);
    }
  };

  const addBookImpl = async (sourceMd: string) => {
    if (!workspaceRoot) return;

    setIsAdding(true);

    try {
      const result = await commands.addBook(workspaceRoot, sourceMd);
      if (result.status === "error") {
        throw new Error(result.error);
      }

      // Notify parent component
      onBookAdded(result.data);
    } catch (error) {
      console.error("Failed to add book:", error);
    } finally {
      setIsAdding(false);
    }
  };

  const handleConfirmDuplicate = async () => {
    if (pendingFile) {
      await addBookImpl(pendingFile);
      setPendingFile(null);
      setDuplicateName("");
    }
  };

  const handleCancelDuplicate = () => {
    setPendingFile(null);
    setDuplicateName("");
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button disabled={!workspaceRoot || isAdding}>
            {isAdding ? t('common.status.saving') : t('books.list.addBookFrom')}
            <ChevronDown className="ml-2 h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onClick={() => setShowGoogleDocDialog(true)}>
            <Cloud className="mr-2 h-4 w-4 text-blue-500" />
            <span>{t('books.list.addFromGoogleDocs')}</span>
          </DropdownMenuItem>
          <DropdownMenuItem onClick={handleAddLocalBook}>
            <FilePlus className="mr-2 h-4 w-4 text-emerald-500" />
            <span>{t('books.list.addFromLocalFile')}</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      {showGoogleDocDialog && (
        <GoogleDocPickerDialog
          title={t("google.dialog.addTitle")}
          description={t("google.dialog.addDescription")}
          actionLabel={t("google.dialog.actions.add")}
          loadingActionLabel={t("google.sync.actions.syncing")}
          onClose={() => setShowGoogleDocDialog(false)}
          onSelect={handleAddGoogleDoc}
        />
      )}

      <ConfirmDialog
        open={showDuplicateDialog}
        onOpenChange={setShowDuplicateDialog}
        title={t('books.duplicate.title')}
        description={t('books.duplicate.description', { bookName: duplicateName })}
        onConfirm={handleConfirmDuplicate}
        onCancel={handleCancelDuplicate}
        confirmText={t('books.duplicate.replace')}
        cancelText={t('common.actions.cancel')}
        variant="default"
      />
    </>
  );
}
