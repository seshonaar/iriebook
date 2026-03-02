import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useAppContext } from "../contexts/AppContext";
import {
  setGitSyncStatus,
  gitOperationStarted,
  gitOperationCompleted,
} from "../contexts/actions";

export function useGitOperations() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();

  const [commitMessage, setCommitMessage] = useState("");
  const [showCommitDialog, setShowCommitDialog] = useState(false);
  const [syncStatusMessage, setSyncStatusMessage] = useState<string | null>(null);

  const refreshStatus = async () => {
    if (!state.selectedFolder) return;

    try {
      const initResult = await commands.gitCheckInitialized(state.selectedFolder);
      if (initResult.status === "error") throw new Error(initResult.error);

      if (!initResult.data) {
        dispatch(setGitSyncStatus({ status: "Uninitialized" }));
        return;
      }

      const statusResult = await commands.gitGetStatus(state.selectedFolder);
      if (statusResult.status === "ok") {
        dispatch(setGitSyncStatus(statusResult.data));
      }
    } catch (err) {
      console.error("Failed to check repository status:", err);
    }
  };

  const handleSave = async () => {
    if (!state.selectedFolder || !commitMessage.trim()) return;

    dispatch(gitOperationStarted());
    setSyncStatusMessage(null);

    try {
      const result = await commands.gitSave(state.selectedFolder, commitMessage);
      if (result.status === "error") throw new Error(result.error);

      setSyncStatusMessage(result.data);
      setCommitMessage("");
      setShowCommitDialog(false);

      dispatch(gitOperationCompleted({ success: true, message: t('git.sync.messages.saveSuccess') }));
      await refreshStatus();
    } catch (err) {
      const msg = String(err);
      setSyncStatusMessage(msg);
      dispatch(gitOperationCompleted({ success: false, message: msg }));
    }
  };

  const handleGetLatest = async () => {
    if (!state.selectedFolder) return;

    dispatch(gitOperationStarted());
    setSyncStatusMessage(null);

    try {
      const syncResult = await commands.gitSync(state.selectedFolder);
      if (syncResult.status === "error") throw new Error(syncResult.error);

      setSyncStatusMessage(String(syncResult.data));
      dispatch(gitOperationCompleted({ success: true, message: t('git.sync.messages.syncSuccess') }));
      await refreshStatus();
    } catch (err) {
      const msg = String(err);
      setSyncStatusMessage(msg);
      dispatch(gitOperationCompleted({ success: false, message: msg }));
    }
  };

  const handleSyncOrClone = async () => {
    if (!state.selectedFolder) return;

    dispatch(gitOperationStarted());
    setSyncStatusMessage(null);

    try {
      const initResult = await commands.gitCheckInitialized(state.selectedFolder);
      if (initResult.status === "error") throw new Error(initResult.error);

      if (!initResult.data) {
        const githubUrl = prompt("Enter GitHub repository URL (e.g., https://github.com/user/repo.git):");
        if (!githubUrl) {
          dispatch(gitOperationCompleted({ success: false, message: t('git.sync.messages.cloneCancelled') }));
          return;
        }

        const cloneResult = await commands.gitCloneRepository(githubUrl, state.selectedFolder);
        if (cloneResult.status === "error") throw new Error(cloneResult.error);

        setSyncStatusMessage(t('git.sync.messages.cloneSuccess'));
      } else {
        const syncResult = await commands.gitSync(state.selectedFolder);
        if (syncResult.status === "error") throw new Error(syncResult.error);
        setSyncStatusMessage(String(syncResult.data));
      }

      dispatch(gitOperationCompleted({ success: true, message: t('git.sync.messages.syncSuccess') }));
      await refreshStatus();
    } catch (err) {
      const msg = String(err);
      setSyncStatusMessage(msg);
      dispatch(gitOperationCompleted({ success: false, message: msg }));
    }
  };

  const handleResetLocalChanges = async () => {
    if (!state.selectedFolder) return;

    const confirmed = window.confirm(t("git.sync.actions.resetConfirm"));
    if (!confirmed) return;

    dispatch(gitOperationStarted());
    setSyncStatusMessage(null);

    try {
      const result = await (commands as any).gitResetLocalChanges(state.selectedFolder);
      if (result.status === "error") throw new Error(result.error);

      setSyncStatusMessage(String(result.data));
      dispatch(gitOperationCompleted({ success: true, message: t("git.sync.messages.resetSuccess") }));
      await refreshStatus();
    } catch (err) {
      const msg = String(err);
      setSyncStatusMessage(msg);
      dispatch(gitOperationCompleted({ success: false, message: msg }));
    }
  };

  const openCommitDialog = () => setShowCommitDialog(true);

  const closeCommitDialog = () => {
    setShowCommitDialog(false);
    setCommitMessage("");
  };

  return {
    // State
    commitMessage,
    setCommitMessage,
    showCommitDialog,
    syncStatusMessage,
    isOperationInProgress: state.gitOperationInProgress,
    canSave: !!state.selectedFolder && !state.gitOperationInProgress,

    // Actions
    handleSave,
    handleGetLatest,
    handleSyncOrClone,
    handleResetLocalChanges,
    openCommitDialog,
    closeCommitDialog,
    refreshStatus,
  };
}
