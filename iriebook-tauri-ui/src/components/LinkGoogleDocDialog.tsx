import { useTranslation } from "react-i18next";
import { commands, type BookInfo, type GoogleDocInfo } from "../bindings";
import { GoogleDocPickerDialog } from "./GoogleDocPickerDialog";

interface LinkGoogleDocDialogProps {
  book: BookInfo;
  onClose: () => void;
  onLinked: () => void;
}

export function LinkGoogleDocDialog({ book, onClose, onLinked }: LinkGoogleDocDialogProps) {
  const { t } = useTranslation();

  const handleLink = async (doc: GoogleDocInfo) => {
    const result = await commands.googleLinkDoc(book.path, doc.id);
    if (result.status === "ok") {
      console.log(t("google.sync.messages.linkSuccess"), book.display_name, "->", doc.name);
      onLinked();
      return;
    }

    throw new Error(`${t("google.sync.messages.linkFailed")}: ${result.error}`);
  };

  return (
    <GoogleDocPickerDialog
      title={t("google.dialog.title")}
      description={t("google.dialog.description", { bookName: book.display_name })}
      actionLabel={t("google.dialog.actions.link")}
      loadingActionLabel={t("google.sync.actions.linking")}
      dataTestId="link-google-doc-dialog"
      actionTestIdPrefix="google-doc-link-button"
      onClose={onClose}
      onSelect={handleLink}
    />
  );
}
