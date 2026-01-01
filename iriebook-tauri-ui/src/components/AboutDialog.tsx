import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ExternalLinkIcon } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";

interface AboutDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AboutDialog({ open, onOpenChange }: AboutDialogProps) {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string>("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle className="text-2xl">{t('dialogs.about.title')}</DialogTitle>
          <DialogDescription>{t('dialogs.about.subtitle')}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div>
            <h4 className="text-sm font-semibold mb-2">{t('dialogs.about.version')}</h4>
            <p className="text-sm text-muted-foreground">{version}</p>
          </div>

          <div>
            <h4 className="text-sm font-semibold mb-2">{t('dialogs.about.description')}</h4>
            <p className="text-sm text-muted-foreground">
              {t('dialogs.about.descriptionText')}
            </p>
          </div>

          <div>
            <h4 className="text-sm font-semibold mb-2">{t('dialogs.about.license')}</h4>
            <p className="text-sm text-muted-foreground">
              Open Source - Check repository for license details
            </p>
          </div>

          <div>
            <h4 className="text-sm font-semibold mb-2">{t('dialogs.about.repository')}</h4>
            <a
              href="https://github.com/yourusername/iriebook"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-sm text-primary hover:underline"
            >
              {t('dialogs.about.repositoryLink')}
              <ExternalLinkIcon className="h-3 w-3" />
            </a>
          </div>

          <div className="pt-4 border-t">
            <p className="text-xs text-muted-foreground text-center">
              {t('dialogs.about.builtWith')}
            </p>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
