import "./App.css";
import { AppProvider, useAppContext } from "./contexts/AppContext";
import {
  setLoading,
  setError,
  setFolder,
  setBooks,
  setViewedBook,
  setActiveTab,
  closeDiffTab,
} from "./contexts/actions";
import { useSession } from "./hooks/useSession";
import { useProcessingEvents } from "./hooks/useProcessingEvents";
import { BookList } from "./components/BookList";
import { ProcessingPanel } from "./components/ProcessingPanel";
import { StatusBar } from "./components/StatusBar";
import { BookViewer } from "./components/BookViewer";
import { GitSyncPanel } from "./components/GitSyncPanel";
import { GitHistoryPanel } from "./components/GitHistoryPanel";
import { DiffViewer } from "./components/DiffViewer";
import { AnalysisResultsPanel } from "./components/AnalysisResultsPanel";
import ErrorBoundary from "./components/ErrorBoundary";
import { MenuBar } from "./components/MenuBar";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "./components/ui/button";
import { FolderOpen, X } from "lucide-react";
import { commands } from "./bindings";
import { toast } from "sonner";
import { useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";

function AppContent() {
  const { t } = useTranslation();
  const { state, dispatch } = useAppContext();

  // Helper to trim long tab titles with middle ellipsis
  const formatTabTitle = (title: string) => {
    const maxLen = 30;
    if (title.length <= maxLen) return title;

    const charsToShow = maxLen - 1; // 1 for the ellipsis character
    const frontChars = Math.ceil(charsToShow / 2);
    const backChars = Math.floor(charsToShow / 2);

    return title.substring(0, frontChars) + "…" + title.substring(title.length - backChars);
  };

  // Initialize session management and processing event listener
  useSession();
  useProcessingEvents();

  const handleSelectFolder = useCallback(async () => {
    try {
      dispatch(setLoading(true));
      dispatch(setError(null));

      const folderResult = await commands.selectFolder();
      if (folderResult.status === "error") {
        throw new Error(folderResult.error);
      }
      const folder = folderResult.data;

      if (folder) {
        dispatch(setFolder(folder));
        const booksResult = await commands.scanBooks(folder);
        if (booksResult.status === "error") {
          throw new Error(booksResult.error);
        }
        dispatch(setBooks(booksResult.data));

        // Auto-select first book if available
        if (booksResult.data.length > 0) {
           dispatch(setViewedBook(0));
        }
      }
    } catch (err) {
      toast.error(t('errors.operations.selectFolder'), {
        description: String(err),
      });
      dispatch(setError(String(err)));
    } finally {
      dispatch(setLoading(false));
    }
  }, [dispatch, t]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+O - Select folder
      if (e.ctrlKey && e.key === "o") {
        e.preventDefault();
        handleSelectFolder();
      }

      // Ctrl+Q - Exit
      if (e.ctrlKey && e.key === "q") {
        e.preventDefault();
        window.close();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSelectFolder]);

  return (
    <div className="h-screen bg-background flex flex-col">
      {/* Menu Bar */}
      <MenuBar
        onSelectFolder={handleSelectFolder}
        currentFolder={state.selectedFolder}
      />
      <div className="container mx-auto max-w-full p-6 lg:p-10 flex-1 flex flex-col overflow-auto">
        {/* Error Display */}
        {state.error && (
          <div className="mb-6 bg-destructive/10 border border-destructive text-destructive rounded-lg p-4">
            <strong>{t('common.labels.error')}:</strong> {state.error}
          </div>
        )}

        {/* Loading State */}
        {state.loading && (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto mb-4"></div>
              <p className="text-muted-foreground">{t('common.status.scanning')}</p>
            </div>
          </div>
        )}

        {/* Main Content */}
        {!state.loading && (
          <div className="flex-1 flex flex-col space-y-6 overflow-hidden">
            {state.selectedFolder ? (
              <Tabs
                value={state.activeTab}
                onValueChange={(value) => {
                  dispatch(setActiveTab(value));
                }}
                className="w-full flex-1 flex flex-col min-h-0"
              >
                <TabsList className="w-full mb-4 justify-start shrink-0">
                  <TabsTrigger value="books">{t('common.navigation.books') || 'Books'}</TabsTrigger>
                  <TabsTrigger value="history">{t('common.navigation.history') || 'Change History'}</TabsTrigger>
                  <TabsTrigger value="analysis">{t('common.navigation.analysis') || 'Analysis'}</TabsTrigger>

                  {/* Dynamic diff tabs */}
                  {state.openDiffTabs.map((tab) => (
                    <TabsTrigger
                      key={tab.id}
                      value={tab.id}
                      className="gap-2"
                      title={tab.title}
                    >
                      <span>{formatTabTitle(tab.title)}</span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          dispatch(closeDiffTab(tab.id));
                        }}
                        className="ml-1 rounded-sm opacity-70 hover:opacity-100 hover:bg-muted p-0.5"
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </TabsTrigger>
                  ))}
                </TabsList>

                <TabsContent value="books" className="flex-1 min-h-0 overflow-auto">
                  {/* Processing Panel - Full width row */}
                  <ProcessingPanel />

                  {/* 2-column layout for Books */}
                  <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 pb-6 mt-6">
                    {/* Left column */}
                    <div className="flex flex-col gap-6">
                      <BookList />
                    </div>

                    {/* Right column */}
                    <div className="flex flex-col gap-6">
                      {/* Show BookViewer if a book is selected */}
                      {state.viewedBookIndex !== null &&
                        state.books[state.viewedBookIndex] && (
                          <BookViewer
                            book={state.books[state.viewedBookIndex]}
                            allBooks={state.books}
                            workspaceRoot={state.selectedFolder}
                            onMetadataUpdated={async () => {
                              // Rescan books to update metadata
                              if (state.selectedFolder) {
                                try {
                                  const result = await commands.scanBooks(state.selectedFolder);
                                  if (result.status === "error") {
                                    throw new Error(result.error);
                                  }
                                  dispatch(setBooks(result.data));
                                  toast.success(t('toasts.success.bookUpdated'));
                                } catch (err) {
                                  toast.error(t('errors.operations.refreshBooks'), {
                                    description: String(err),
                                  });
                                }
                              }
                            }}
                          />
                        )}
                    </div>
                  </div>
                </TabsContent>

                <TabsContent value="history" className="flex-1 flex flex-col min-h-0 space-y-4">
                  <div className="bg-card border border-border rounded-lg p-2 shrink-0">
                    <GitSyncPanel />
                  </div>

                  <div className="flex-1 min-h-0">
                    <GitHistoryPanel />
                  </div>
                </TabsContent>

                <TabsContent value="analysis" className="flex-1 min-h-0">
                  <AnalysisResultsPanel />
                </TabsContent>

                {/* Dynamic diff tab content */}
                {state.openDiffTabs.map((tab) => (
                  <TabsContent key={tab.id} value={tab.id} className="h-[calc(100vh-12rem)]">
                    <div className="bg-card border border-border rounded-lg h-full overflow-hidden">
                      <DiffViewer tab={tab} />
                    </div>
                  </TabsContent>
                ))}
              </Tabs>
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center border-2 border-dashed border-muted rounded-xl bg-muted/5 py-20 px-4">
                <FolderOpen className="h-20 w-20 text-muted-foreground mb-6 opacity-20" />
                <h2 className="text-3xl font-bold mb-3 tracking-tight">{t('common.workspace.noWorkspaceTitle')}</h2>
                <p className="text-muted-foreground mb-10 max-w-md text-center text-lg">
                  {t('common.workspace.noWorkspaceDescription')}
                </p>
                <Button onClick={handleSelectFolder} size="lg" className="gap-2 px-8 h-12 text-base">
                  <FolderOpen className="h-5 w-5" />
                  {t('common.workspace.selectWorkspace')}
                </Button>
              </div>
            )}
          </div>
        )}
      </div>
      <StatusBar />
    </div>
  );
}

function App() {
  return (
    <ErrorBoundary>
      <AppProvider>
        <AppContent />
        <Toaster />
      </AppProvider>
    </ErrorBoundary>
  );
}

export default App;
