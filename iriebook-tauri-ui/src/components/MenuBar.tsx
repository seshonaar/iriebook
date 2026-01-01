import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  DropdownMenuShortcut,
} from "@/components/ui/dropdown-menu";
import { AboutDialog } from "./AboutDialog";
import { FolderOpenIcon, XIcon, HelpCircleIcon, DownloadIcon } from "lucide-react";
import { commands } from "../bindings";
import { toast } from "sonner";

interface MenuBarProps {
  onSelectFolder?: () => void;
  currentFolder?: string | null;
}

export function MenuBar({ onSelectFolder, currentFolder }: MenuBarProps) {
  const { t } = useTranslation();
  const [aboutDialogOpen, setAboutDialogOpen] = useState(false);

  const handleOpenWorkspace = async () => {
    if (currentFolder) {
      try {
        const result = await commands.openFolder(currentFolder);
        if (result.status === "error") {
          throw new Error(result.error);
        }
        toast.success(t('toasts.success.workspaceOpened'));
      } catch (err) {
        toast.error(t('errors.operations.openWorkspace'), {
          description: String(err),
        });
      }
    } else {
      toast.info(t('toasts.info.noWorkspace'));
    }
  };

  const handleExit = () => {
    window.close();
  };

  const handleCheckForUpdates = async () => {
    // Errors are emitted as events and shown in Results panel
    await commands.checkForUpdates();
  };

  return (
    <>
      <div className="flex items-center h-10 bg-secondary/30 border-b border-border px-2 gap-1">
        {/* File Menu */}
        <DropdownMenu>
          <DropdownMenuTrigger className="px-3 py-1 text-sm rounded hover:bg-accent hover:text-accent-foreground outline-none">
            {t('menu.file.title')}
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-56">
            <DropdownMenuItem onClick={onSelectFolder}>
              <FolderOpenIcon className="mr-2 h-4 w-4" />
              {t('menu.file.changeWorkspace')}
              <DropdownMenuShortcut>{t('menu.shortcuts.changeWorkspace')}</DropdownMenuShortcut>
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={handleOpenWorkspace}
              disabled={!currentFolder}
            >
              <FolderOpenIcon className="mr-2 h-4 w-4" />
              {t('menu.file.openWorkspaceFolder')}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={handleExit}>
              <XIcon className="mr-2 h-4 w-4" />
              {t('menu.file.exit')}
              <DropdownMenuShortcut>{t('menu.shortcuts.exit')}</DropdownMenuShortcut>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {/* Help Menu */}
        <DropdownMenu>
          <DropdownMenuTrigger className="px-3 py-1 text-sm rounded hover:bg-accent hover:text-accent-foreground outline-none">
            {t('menu.help.title')}
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuItem onClick={handleCheckForUpdates}>
              <DownloadIcon className="mr-2 h-4 w-4" />
              {t('menu.help.checkForUpdates')}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => setAboutDialogOpen(true)}>
              <HelpCircleIcon className="mr-2 h-4 w-4" />
              {t('menu.help.about')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* About Dialog */}
      <AboutDialog open={aboutDialogOpen} onOpenChange={setAboutDialogOpen} />
    </>
  );
}
