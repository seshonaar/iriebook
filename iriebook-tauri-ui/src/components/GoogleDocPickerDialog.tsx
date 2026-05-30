import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type GoogleDocInfo } from "../bindings";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { Calendar, FileText, Loader2 } from "lucide-react";

interface GoogleDocPickerDialogProps {
  title: string;
  description: string;
  actionLabel: string;
  loadingActionLabel: string;
  dataTestId?: string;
  actionTestIdPrefix?: string;
  onClose: () => void;
  onSelect: (doc: GoogleDocInfo) => Promise<void>;
}

export function GoogleDocPickerDialog({
  title,
  description,
  actionLabel,
  loadingActionLabel,
  dataTestId = "google-doc-picker-dialog",
  actionTestIdPrefix = "google-doc-select-button",
  onClose,
  onSelect,
}: GoogleDocPickerDialogProps) {
  const { t } = useTranslation();
  const [docs, setDocs] = useState<GoogleDocInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [selecting, setSelecting] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadDocs();
  }, []);

  const loadDocs = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await commands.googleListDocs();
      if (result.status === "ok") {
        setDocs(result.data);
      } else {
        setError(`${t("google.dialog.errors.loadFailed")}: ${result.error}`);
      }
    } catch (err) {
      setError(`${t("google.errors.generic")}: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSelect = async (doc: GoogleDocInfo) => {
    setSelecting(doc.id);
    setError(null);
    try {
      await onSelect(doc);
    } catch (err) {
      setError(String(err));
    } finally {
      setSelecting(null);
    }
  };

  const filteredDocs = docs.filter((doc) =>
    doc.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const formatDate = (dateString: string) => {
    try {
      const date = new Date(dateString);
      return date.toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return dateString;
    }
  };

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent
        className="w-[calc(100vw-2rem)] max-w-3xl max-h-[85vh] overflow-hidden p-0 gap-0"
        data-testid={dataTestId}
      >
        <DialogHeader className="px-6 pt-6 pb-4 border-b border-border/60">
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 p-6 pt-4">
          <Input
            placeholder={t("google.dialog.search")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            disabled={loading}
            data-testid="google-doc-search-input"
          />

          {error && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 text-destructive rounded-md text-sm">
              <p>{error}</p>
            </div>
          )}

          {loading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : filteredDocs.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              {searchQuery
                ? t("google.dialog.noMatches")
                : t("google.dialog.noDocuments")}
            </div>
          ) : (
            <ScrollArea className="max-h-[52vh] sm:max-h-[420px] pr-2">
              <div className="space-y-2">
                {filteredDocs.map((doc) => (
                  <div
                    key={doc.id}
                    className="grid gap-3 rounded-xl border border-border/70 bg-card/70 p-4 transition-colors hover:bg-accent/70 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center"
                    data-testid={`google-doc-item-${doc.id}`}
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <FileText className="h-4 w-4 text-blue-400 flex-shrink-0" />
                        <h4 className="font-medium truncate">{doc.name}</h4>
                      </div>
                      <div className="flex items-center gap-1 text-sm text-muted-foreground">
                        <Calendar className="h-3 w-3" />
                        <span>{t("google.dialog.modified", { date: formatDate(doc.modified_time) })}</span>
                      </div>
                    </div>
                    <Button
                      onClick={() => handleSelect(doc)}
                      disabled={selecting !== null}
                      variant="secondary"
                      size="sm"
                      className="w-full sm:w-auto"
                      data-testid={`${actionTestIdPrefix}-${doc.id}`}
                    >
                      {selecting === doc.id ? (
                        <>
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                          {loadingActionLabel}
                        </>
                      ) : (
                        actionLabel
                      )}
                    </Button>
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
