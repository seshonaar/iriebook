import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands, type BookMetadata, type BookInfo } from "../bindings";
import { CoverImage } from "./CoverImage";
import { MetadataDisplay } from "./MetadataDisplay";
import { MetadataEditor } from "./MetadataEditor";

interface BookViewerProps {
  book: BookInfo;
  allBooks: BookInfo[];
  workspaceRoot: string | null;
  onMetadataUpdated?: () => void;
}

export function BookViewer({
  book,
  allBooks,
  workspaceRoot,
  onMetadataUpdated,
}: BookViewerProps) {
  const { t } = useTranslation();
  const [metadata, setMetadata] = useState<BookMetadata | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load metadata when book changes
  useEffect(() => {
    const loadBookMetadata = async () => {
      setLoading(true);
      setError(null);
      setIsEditing(false);

      try {
        const result = await commands.loadBookMetadata(book.path);
        if (result.status === "error") {
          throw new Error(result.error);
        }
        setMetadata(result.data);
      } catch (err) {
        console.error(t('errors.operations.loadMetadata'), err);
        setError(`${t('errors.operations.loadMetadata')}: ${err}`);
      } finally {
        setLoading(false);
      }
    };

    loadBookMetadata();
  }, [book.path, t]);

  const handleSave = (updatedMetadata: BookMetadata) => {
    setMetadata(updatedMetadata);
    setIsEditing(false);
    if (onMetadataUpdated) {
      onMetadataUpdated();
    }
  };

  const handleCancel = () => {
    setIsEditing(false);
  };

  if (loading) {
    return (
      <div className="bg-card border border-border rounded-lg p-6">
        <div className="flex items-center justify-center min-h-[400px]">
          <div className="text-muted-foreground">{t('books.viewer.loading')}</div>
        </div>
      </div>
    );
  }

  if (error || !metadata) {
    return (
      <div className="bg-card border border-border rounded-lg p-6">
        <div className="flex items-center justify-center min-h-[400px]">
          <div className="text-destructive">
            {error || t('errors.operations.loadMetadata')}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-card border border-border rounded-lg p-6">
      <div className="mb-4">
        <h2 className="text-2xl font-bold break-words">{book.display_name}</h2>
      </div>

      <div className="flex gap-6">
        {/* Left column - Cover image */}
        <div className="flex-shrink-0">
          <CoverImage
            bookPath={book.path}
            coverImagePath={book.cover_image_path}
          />
        </div>

        {/* Right column - Metadata */}
        <div className="flex-1 min-w-0 overflow-auto">
          {isEditing ? (
            <MetadataEditor
              bookPath={book.path}
              metadata={metadata}
              allBooks={allBooks}
              onSave={handleSave}
              onCancel={handleCancel}
            />
          ) : (
            <MetadataDisplay
              bookPath={book.path}
              workspaceRoot={workspaceRoot}
              metadata={metadata}
              onEdit={() => setIsEditing(true)}
            />
          )}
        </div>
      </div>
    </div>
  );
}
