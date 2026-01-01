import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { BarChart3, RefreshCw } from "lucide-react";
import { useAppContext } from "../contexts/AppContext";
import { commands, type AnalysisResponse } from "../bindings";
import { BookListForAnalysis } from "./analysis/BookListForAnalysis";
import { StatisticsCards } from "./analysis/StatisticsCards";
import { WordFrequencyChart } from "./analysis/WordFrequencyChart";
import { WordTable } from "./analysis/WordTable";
import { ScrollArea } from "./ui/scroll-area";
import { Button } from "./ui/button";

function formatTimeAgo(timestamp: number, t: (key: string, options?: Record<string, unknown>) => string): string {
  const now = Math.floor(Date.now() / 1000);
  const diff = now - timestamp;

  if (diff < 60) {
    return t("analysis.cache.justNow");
  } else if (diff < 3600) {
    const minutes = Math.floor(diff / 60);
    return t("analysis.cache.minutesAgo", { count: minutes });
  } else if (diff < 86400) {
    const hours = Math.floor(diff / 3600);
    return t("analysis.cache.hoursAgo", { count: hours });
  } else {
    const days = Math.floor(diff / 86400);
    return t("analysis.cache.daysAgo", { count: days });
  }
}

export function AnalysisResultsPanel() {
  const { t } = useTranslation();
  const { state } = useAppContext();
  const [selectedBookPath, setSelectedBookPath] = useState<string | null>(null);
  const [analysisData, setAnalysisData] = useState<AnalysisResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Auto-select first book on mount or when books change
  useEffect(() => {
    if (state.books.length > 0 && !selectedBookPath) {
      setSelectedBookPath(state.books[0].path);
    } else if (state.books.length > 0 && selectedBookPath) {
      // Check if selected book still exists
      const bookExists = state.books.some((b) => b.path === selectedBookPath);
      if (!bookExists) {
        setSelectedBookPath(state.books[0].path);
      }
    } else if (state.books.length === 0) {
      setSelectedBookPath(null);
      setAnalysisData(null);
    }
  }, [state.books, selectedBookPath]);

  const fetchAnalysis = useCallback(async (bookPath: string, forceRefresh: boolean) => {
    setIsLoading(true);
    setError(null);

    try {
      const result = await commands.getBookAnalysis(bookPath, forceRefresh);
      if (result.status === "error") {
        throw new Error(result.error);
      }
      setAnalysisData(result.data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setAnalysisData(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Fetch analysis when selected book changes
  useEffect(() => {
    if (selectedBookPath) {
      fetchAnalysis(selectedBookPath, false);
    }
  }, [selectedBookPath, fetchAnalysis]);

  const handleRefresh = () => {
    if (selectedBookPath) {
      fetchAnalysis(selectedBookPath, true);
    }
  };

  const handleSelectBook = (path: string) => {
    setSelectedBookPath(path);
  };

  // Get selected book name (prefer metadata title, fallback to display_name)
  const selectedBook = state.books.find((b) => b.path === selectedBookPath);
  const bookName = selectedBook?.metadata?.title || selectedBook?.display_name || "";

  // Empty state - no books
  if (state.books.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-20">
        <BarChart3 className="h-16 w-16 text-muted-foreground opacity-30 mb-6" />
        <h2 className="text-2xl font-bold mb-2">{t("analysis.empty.title")}</h2>
        <p className="text-muted-foreground text-center max-w-md">
          {t("analysis.empty.noBooks")}
        </p>
      </div>
    );
  }

  return (
    <div className="h-full flex">
      {/* Left sidebar - Book list */}
      <div className="w-64 border-r flex-shrink-0 bg-muted/30">
        <BookListForAnalysis
          books={state.books}
          selectedPath={selectedBookPath}
          onSelectBook={handleSelectBook}
        />
      </div>

      {/* Right content - Analysis results */}
      <div className="flex-1 flex flex-col min-h-0">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b bg-card">
          <div>
            <h2 className="text-lg font-semibold truncate" title={bookName}>
              {bookName || t("analysis.header.selectBook")}
            </h2>
            {analysisData && !isLoading && (
              <p className="text-sm text-muted-foreground">
                {analysisData.was_cached
                  ? formatTimeAgo(analysisData.cache_timestamp, t)
                  : t("analysis.cache.freshAnalysis")}
              </p>
            )}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleRefresh}
            disabled={!selectedBookPath || isLoading}
            className="flex items-center gap-2"
          >
            <RefreshCw className={`h-4 w-4 ${isLoading ? "animate-spin" : ""}`} />
            {t("analysis.actions.refresh")}
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 min-h-0">
          {isLoading && !analysisData && (
            <div className="flex items-center justify-center h-full">
              <div className="flex flex-col items-center gap-4">
                <RefreshCw className="h-8 w-8 animate-spin text-primary" />
                <p className="text-muted-foreground">{t("analysis.loading")}</p>
              </div>
            </div>
          )}

          {error && (
            <div className="flex items-center justify-center h-full">
              <div className="text-center">
                <p className="text-destructive mb-2">{t("analysis.error.title")}</p>
                <p className="text-sm text-muted-foreground">{error}</p>
              </div>
            </div>
          )}

          {analysisData && !error && (
            <ScrollArea className="h-full">
              <div className="space-y-6 p-6">
                {/* Statistics Cards */}
                <StatisticsCards stats={analysisData.stats} />

                {/* Two-column layout for chart and table on larger screens */}
                <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
                  {/* Bar Chart */}
                  <WordFrequencyChart stats={analysisData.stats} />

                  {/* Word Table */}
                  <WordTable stats={analysisData.stats} />
                </div>
              </div>
            </ScrollArea>
          )}

          {!selectedBookPath && !isLoading && !error && (
            <div className="flex items-center justify-center h-full">
              <p className="text-muted-foreground">{t("analysis.tabs.selectBook")}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
