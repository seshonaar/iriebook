import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type AddBookResult } from "../bindings";
import { Button } from "./ui/button";
import { ConfirmDialog } from "./ConfirmDialog";

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
  const [pendingFile, setPendingFile] = useState<string | null>(null);
  const [duplicateName, setDuplicateName] = useState<string>("");

  const handleAddBook = async () => {
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
      <Button
        onClick={handleAddBook}
        disabled={!workspaceRoot || isAdding}
      >
        {isAdding ? t('common.status.saving') : t('books.list.addBook')}
      </Button>

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
