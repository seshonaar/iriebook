import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands, type BookInfo, type GoogleDocInfo } from "../bindings";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "./ui/dialog";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { Loader2, FileText, Calendar } from "lucide-react";

interface LinkGoogleDocDialogProps {
  book: BookInfo;
  onClose: () => void;
  onLinked: () => void;
}

export function LinkGoogleDocDialog({ book, onClose, onLinked }: LinkGoogleDocDialogProps) {
  const { t } = useTranslation();
  const [docs, setDocs] = useState<GoogleDocInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [linking, setLinking] = useState<string | null>(null);
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

  const handleLink = async (doc: GoogleDocInfo) => {
    setLinking(doc.id);
    setError(null);
    try {
      const result = await commands.googleLinkDoc(book.path, doc.id);
      if (result.status === "ok") {
        console.log(t("google.sync.messages.linkSuccess"), book.display_name, "→", doc.name);
        onLinked();
      } else {
        setError(`${t("google.sync.messages.linkFailed")}: ${result.error}`);
      }
    } catch (err) {
      setError(`${t("google.sync.messages.linkFailed")}: ${String(err)}`);
    } finally {
      setLinking(null);
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
      <DialogContent className="max-w-2xl max-h-[80vh]">
        <DialogHeader>
          <DialogTitle>{t("google.dialog.title")}</DialogTitle>
          <DialogDescription>
            {t("google.dialog.description", { bookName: book.display_name })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <Input
            placeholder={t("google.dialog.search")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            disabled={loading}
          />

          {/* Error display */}
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
            <ScrollArea className="h-[400px] pr-4">
              <div className="space-y-2">
                {filteredDocs.map((doc) => (
                  <div
                    key={doc.id}
                    className="flex items-start justify-between p-4 border rounded-lg hover:bg-accent transition-colors"
                  >
                    <div className="flex-1 min-w-0 mr-4">
                      <div className="flex items-center gap-2 mb-1">
                        <FileText className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                        <h4 className="font-medium truncate">{doc.name}</h4>
                      </div>
                      <div className="flex items-center gap-1 text-sm text-muted-foreground">
                        <Calendar className="h-3 w-3" />
                        <span>{t("google.dialog.modified", { date: formatDate(doc.modified_time) })}</span>
                      </div>
                    </div>
                    <Button
                      onClick={() => handleLink(doc)}
                      disabled={linking !== null}
                      size="sm"
                    >
                      {linking === doc.id ? (
                        <>
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                          {t("google.sync.actions.linking")}
                        </>
                      ) : (
                        t("google.dialog.actions.link")
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
