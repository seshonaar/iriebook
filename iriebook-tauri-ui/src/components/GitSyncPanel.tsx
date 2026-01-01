import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useAppContext } from "../contexts/AppContext";
import {
  setGitAuthStatus,
  setGitSyncStatus,
  deviceFlowStarted,
  deviceFlowCompleted,
} from "../contexts/actions";
import { useGitOperations } from "../hooks/useGitOperations";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";
import {
  Download,
  Upload,
  RefreshCw,
  AlertCircle,
  Github,
  LogOut,
  Loader2,
  CheckCircle2
} from "lucide-react";

export function GitSyncPanel() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();
  const {
    commitMessage,
    setCommitMessage,
    showCommitDialog,
    syncStatusMessage,
    isOperationInProgress,
    handleSave,
    handleSyncOrClone,
    openCommitDialog,
    closeCommitDialog,
    refreshStatus,
  } = useGitOperations();

  // Auth State
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);

  // Initial checks
  useEffect(() => {
    checkAuthStatus();
  }, []);

  useEffect(() => {
    if (state.selectedFolder) {
      refreshStatus();
    }
  }, [state.selectedFolder]);

  // --- Auth Logic ---

  const checkAuthStatus = async () => {
    try {
      const result = await commands.githubCheckAuth();
      if (result.status === "error") {
        throw new Error(result.error);
      }
      const isAuthenticated = result.data;
      dispatch(setGitAuthStatus(
        isAuthenticated
          ? { status: "Authenticated" }
          : { status: "NotAuthenticated" }
      ));
    } catch (err) {
      console.error("Failed to check auth status:", err);
    }
  };

  const handleAuthenticate = async () => {
    setIsAuthenticating(true);
    setAuthError(null);

    try {
      const flowResult = await commands.githubDeviceFlowStart();
      if (flowResult.status === "error") {
        throw new Error(flowResult.error);
      }
      const flowInfo = flowResult.data;

      dispatch(deviceFlowStarted(flowInfo));

      const browserResult = await commands.openBrowser(flowInfo.verificationUri);
      if (browserResult.status === "error") {
        throw new Error(browserResult.error);
      }

      pollForToken(flowInfo.deviceCode);
    } catch (err) {
      setAuthError(err as string);
      setIsAuthenticating(false);
    }
  };

  const pollForToken = async (deviceCode: string) => {
    try {
      const pollResult = await commands.githubDeviceFlowPoll(deviceCode);
      if (pollResult.status === "error") {
        throw new Error(pollResult.error);
      }
      const token = pollResult.data;

      const storeResult = await commands.githubStoreToken(token);
      if (storeResult.status === "error") {
        throw new Error(storeResult.error);
      }

      dispatch(deviceFlowCompleted({ success: true }));

      setIsAuthenticating(false);
      setAuthError(null);

      // Check if we need to clone (auto-prompt logic from old AuthPanel)
      await checkAndPromptForClone();
    } catch (err) {
      setAuthError(err as string);
      dispatch(deviceFlowCompleted({ success: false }));
      setIsAuthenticating(false);
    }
  };

  const checkAndPromptForClone = async () => {
    if (!state.selectedFolder) return;

    try {
      // Verify auth first
      const authResult = await commands.githubCheckAuth();
      if (authResult.status === "error" || !authResult.data) return;

      const initResult = await commands.gitCheckInitialized(state.selectedFolder);
      if (initResult.status === "error") return;
      
      if (!initResult.data) {
        const githubUrl = prompt(t('git.auth.deviceFlow.prompt'));
        if (githubUrl && githubUrl.trim()) {
          setIsAuthenticating(true);
          const cloneResult = await commands.gitCloneRepository(
            githubUrl.trim(),
            state.selectedFolder
          );
          if (cloneResult.status === "error") throw new Error(cloneResult.error);

          dispatch(setGitSyncStatus({ status: "Clean" }));
          setIsAuthenticating(false);
        }
      }
    } catch (err) {
      setAuthError(`Failed to clone: ${err}`);
      setIsAuthenticating(false);
    }
  };

  const handleLogout = async () => {
    try {
      const result = await commands.githubLogout();
      if (result.status === "error") {
        throw new Error(result.error);
      }
      dispatch(setGitAuthStatus({ status: "NotAuthenticated" }));
    } catch (err) {
      setAuthError(err as string);
    }
  };

  // --- Render ---

  const isAuthenticated = state.gitAuthStatus.status === "Authenticated";
  const isUninitialized = state.gitSyncStatus.status === "Uninitialized";

  if (!isAuthenticated) {
    return (
      <div className="flex flex-col gap-4 p-1">
        {!state.deviceFlowInfo ? (
           <div className="flex items-center gap-3">
             <div className="flex-1 flex items-center gap-2 p-2 bg-amber-50 dark:bg-amber-950/20 text-amber-700 dark:text-amber-400 rounded-md border border-amber-200 dark:border-amber-900 text-sm">
                <AlertCircle className="h-4 w-4" />
                <p className="font-medium">{t('git.auth.notConnected')}</p>
             </div>
             <Button
                onClick={handleAuthenticate}
                disabled={isAuthenticating}
                size="sm"
              >
                {isAuthenticating ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    {t('git.auth.authenticating')}
                  </>
                ) : (
                  <>
                    <Github className="mr-2 h-4 w-4" />
                    {t('git.auth.connect')}
                  </>
                )}
              </Button>
           </div>
        ) : (
          <div className="p-4 bg-muted border border-border rounded-lg text-center space-y-3">
            <p className="text-sm text-muted-foreground">
              {t('git.auth.deviceFlow.enterCode')}
            </p>
            <div className="text-2xl font-bold font-mono tracking-wider py-2 bg-background rounded border border-border select-all">
              {state.deviceFlowInfo.userCode}
            </div>
            <p className="text-sm text-muted-foreground animate-pulse">
              {t('git.auth.deviceFlow.waiting')}
            </p>
          </div>
        )}
        
        {authError && (
          <div className="p-2 bg-destructive/10 border border-destructive/20 text-destructive rounded-md text-sm">
            <p>{authError}</p>
          </div>
        )}
      </div>
    );
  }

  // Authenticated Toolbar View
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between p-1">
        {/* Action Buttons */}
        <div className="flex items-center gap-1">
            {!isUninitialized && (
              <Button
                onClick={openCommitDialog}
                disabled={isOperationInProgress || showCommitDialog}
                variant="outline"
                size="sm"
                className="h-9"
              >
                <Upload className="mr-2 h-3.5 w-3.5" />
                <span className="sr-only sm:not-sr-only sm:inline-block">
                    {t('git.sync.actions.saveToCloud')}
                </span>
              </Button>
            )}

            <Button
              onClick={handleSyncOrClone}
              disabled={isOperationInProgress}
              size="sm"
              variant="outline"
              className="h-9"
            >
              {isOperationInProgress ? (
                  <RefreshCw className="mr-2 h-3.5 w-3.5 animate-spin" />
              ) : isUninitialized ? (
                  <Download className="mr-2 h-3.5 w-3.5" />
              ) : (
                  <RefreshCw className="mr-2 h-3.5 w-3.5" />
              )}
              <span className="sr-only sm:not-sr-only sm:inline-block">
                 {isOperationInProgress 
                    ? t('git.sync.actions.syncing') 
                    : isUninitialized 
                        ? t('git.sync.actions.clone') 
                        : t('git.sync.actions.sync')}
              </span>
            </Button>

            <div className="w-px h-6 bg-border mx-1" />

            {/* Disconnect Button */}
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleLogout}
                    className="h-9 w-9 px-0 text-muted-foreground hover:text-destructive"
                  >
                    <LogOut className="h-4 w-4" />
                    <span className="sr-only">{t('git.auth.disconnect')}</span>
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <div className="flex items-center gap-2">
                     <CheckCircle2 className="h-3 w-3 text-green-500" />
                     <p>{t('git.auth.connected')}</p>
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">{t('git.auth.disconnect')}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
        </div>
      </div>

      {/* Commit Dialog */}
      {showCommitDialog && (
        <div className="p-3 bg-muted/30 border border-border rounded-lg space-y-3 animate-in fade-in slide-in-from-top-2">
          <Input
            type="text"
            value={commitMessage}
            onChange={(e) => setCommitMessage(e.target.value)}
            placeholder={t('git.sync.actions.enterRevision')}
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter' && commitMessage.trim() && !isOperationInProgress) {
                handleSave();
              }
              if (e.key === 'Escape') {
                closeCommitDialog();
              }
            }}
          />
          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              onClick={closeCommitDialog}
              disabled={isOperationInProgress}
              size="sm"
            >
              {t('common.actions.cancel')}
            </Button>
            <Button
              onClick={handleSave}
              disabled={!commitMessage.trim() || isOperationInProgress}
              size="sm"
            >
              {t('git.sync.actions.saveToCloudGithub')}
            </Button>
          </div>
        </div>
      )}

      {/* Status/Error Messages */}
      {(syncStatusMessage || authError) && (
        <div className={`p-2 rounded-md text-sm border ${
            (syncStatusMessage && typeof syncStatusMessage === 'string' && (syncStatusMessage.toLowerCase().includes("success") || syncStatusMessage.includes("Clean") || syncStatusMessage.includes("Synced and pushed")))
            ? "bg-green-50 dark:bg-green-950/20 text-green-700 dark:text-green-400 border-green-200 dark:border-green-900"
            : "bg-destructive/10 text-destructive border-destructive/20"
        }`}>
          <p>{syncStatusMessage || authError}</p>
        </div>
      )}
    </div>
  );
}
