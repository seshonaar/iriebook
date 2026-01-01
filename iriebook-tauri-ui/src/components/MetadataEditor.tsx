import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands, type BookMetadata, type BookInfo } from "../bindings";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

interface MetadataEditorProps {
  bookPath: string;
  metadata: BookMetadata;
  allBooks: BookInfo[];
  onSave: (metadata: BookMetadata) => void;
  onCancel: () => void;
}

export function MetadataEditor({
  bookPath,
  metadata,
  allBooks,
  onSave,
  onCancel,
}: MetadataEditorProps) {
  const { t } = useTranslation();
  const [title, setTitle] = useState(metadata.title);
  const [author, setAuthor] = useState(metadata.author);
  const [collection, setCollection] = useState(
    metadata["belongs-to-collection"] || ""
  );
  const [position, setPosition] = useState(
    metadata["group-position"]?.toString() || ""
  );

  const [authors, setAuthors] = useState<string[]>([]);
  const [series, setSeries] = useState<string[]>([]);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);

  // Load autocomplete data
  useEffect(() => {
    const loadAutocomplete = async () => {
      try {
        const authorsResult = await commands.getAutocompleteAuthors(allBooks);
        if (authorsResult.status === "error") {
          throw new Error(authorsResult.error);
        }
        const seriesResult = await commands.getAutocompleteSeries(allBooks);
        if (seriesResult.status === "error") {
          throw new Error(seriesResult.error);
        }
        setAuthors(authorsResult.data);
        setSeries(seriesResult.data);
      } catch (error) {
        console.error("Failed to load autocomplete data:", error);
      }
    };

    loadAutocomplete();
  }, [allBooks]);

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!title.trim()) {
      newErrors.title = t('metadata.validation.titleRequired');
    }

    if (!author.trim()) {
      newErrors.author = t('metadata.validation.authorRequired');
    }

    if (position && isNaN(Number(position))) {
      newErrors.position = t('metadata.validation.positionInvalid');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSave = async () => {
    if (!validate()) {
      return;
    }

    setSaving(true);

    try {
      const updatedMetadata: BookMetadata = {
        title: title.trim(),
        author: author.trim(),
        "belongs-to-collection": collection.trim() || null,
        "group-position": position ? Number(position) : null,
        language: metadata.language,
        rights: metadata.rights,
        "cover-image": metadata["cover-image"],
      };

      const result = await commands.saveBookMetadata(bookPath, updatedMetadata);
      if (result.status === "error") {
        throw new Error(result.error);
      }

      onSave(updatedMetadata);
    } catch (error) {
      console.error("Failed to save metadata:", error);
      setErrors({ save: `${t('errors.operations.saveMetadata')}: ${error}` });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">{t('metadata.editor.title')}</h3>
      </div>

      <div className="space-y-4">
        {/* Title */}
        <div>
          <Label htmlFor="title">
            {t('metadata.editor.fields.title')} <span className="text-red-500">*</span>
          </Label>
          <Input
            id="title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className={errors.title ? "border-red-500" : ""}
          />
          {errors.title && (
            <p className="text-sm text-red-500 mt-1">{errors.title}</p>
          )}
        </div>

        {/* Author */}
        <div>
          <Label htmlFor="author">
            {t('metadata.editor.fields.author')} <span className="text-red-500">*</span>
          </Label>
          <Input
            id="author"
            value={author}
            onChange={(e) => setAuthor(e.target.value)}
            list="authors-list"
            className={errors.author ? "border-red-500" : ""}
          />
          <datalist id="authors-list">
            {authors.map((a) => (
              <option key={a} value={a} />
            ))}
          </datalist>
          {errors.author && (
            <p className="text-sm text-red-500 mt-1">{errors.author}</p>
          )}
        </div>

        {/* Collection/Series */}
        <div>
          <Label htmlFor="collection">{t('metadata.editor.fields.collection')}</Label>
          <Input
            id="collection"
            value={collection}
            onChange={(e) => setCollection(e.target.value)}
            list="series-list"
            placeholder={t('common.labels.optional')}
          />
          <datalist id="series-list">
            {series.map((s) => (
              <option key={s} value={s} />
            ))}
          </datalist>
        </div>

        {/* Position in Series */}
        <div>
          <Label htmlFor="position">{t('metadata.editor.fields.position')}</Label>
          <Input
            id="position"
            type="text"
            value={position}
            onChange={(e) => setPosition(e.target.value)}
            placeholder={t('metadata.editor.fields.positionPlaceholder')}
            className={errors.position ? "border-red-500" : ""}
          />
          {errors.position && (
            <p className="text-sm text-red-500 mt-1">{errors.position}</p>
          )}
        </div>

        {/* Error message */}
        {errors.save && (
          <div className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-600">
            {errors.save}
          </div>
        )}

        {/* Buttons */}
        <div className="flex gap-2 pt-2">
          <Button
            onClick={handleSave}
            disabled={saving}
            className="flex-1"
          >
            {saving ? t('common.status.saving') : t('common.actions.save')}
          </Button>
          <Button
            onClick={onCancel}
            variant="outline"
            disabled={saving}
            className="flex-1"
          >
            {t('common.actions.cancel')}
          </Button>
        </div>
      </div>
    </div>
  );
}
