import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Search, ArrowUp, ArrowDown, ArrowUpDown } from "lucide-react";
import type { WordAnalysisStats } from "../../bindings";
import { ScrollArea } from "../ui/scroll-area";

interface WordTableProps {
  stats: WordAnalysisStats;
}

type SortField = "rank" | "word" | "count" | "percentage";
type SortDirection = "asc" | "desc";

export function WordTable({ stats }: WordTableProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [sortField, setSortField] = useState<SortField>("rank");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");

  const tableData = useMemo(() => {
    const data = stats.top_words.map(([word, count]: [string, number], index: number) => ({
      rank: index + 1,
      word,
      count,
      percentage: stats.total_words > 0 ? (count / stats.total_words) * 100 : 0,
    }));

    // Filter by search
    const filtered = searchQuery
      ? data.filter((row: { word: string }) =>
          row.word.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : data;

    // Sort
    const sorted = [...filtered].sort((a, b) => {
      let comparison = 0;
      switch (sortField) {
        case "rank":
          comparison = a.rank - b.rank;
          break;
        case "word":
          comparison = a.word.localeCompare(b.word);
          break;
        case "count":
          comparison = a.count - b.count;
          break;
        case "percentage":
          comparison = a.percentage - b.percentage;
          break;
      }
      return sortDirection === "asc" ? comparison : -comparison;
    });

    return sorted;
  }, [stats, searchQuery, sortField, sortDirection]);

  const handleSort = (field: SortField) => {
    if (field === sortField) {
      setSortDirection(sortDirection === "asc" ? "desc" : "asc");
    } else {
      setSortField(field);
      setSortDirection(field === "word" ? "asc" : "desc");
    }
  };

  const SortIcon = ({ field }: { field: SortField }) => {
    if (field !== sortField) {
      return <ArrowUpDown className="h-4 w-4 opacity-50" />;
    }
    return sortDirection === "asc" ? (
      <ArrowUp className="h-4 w-4" />
    ) : (
      <ArrowDown className="h-4 w-4" />
    );
  };

  return (
    <div className="bg-card border border-border rounded-lg p-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">{t("analysis.table.title")}</h3>
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <input
            type="text"
            placeholder={t("analysis.table.search")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 pr-4 py-2 text-sm border border-border rounded-md bg-background w-64 focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
          />
        </div>
      </div>

      <ScrollArea className="h-[400px]">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-card border-b border-border">
            <tr>
              <th
                className="text-left py-3 px-4 font-medium cursor-pointer hover:bg-muted/50 transition-colors"
                onClick={() => handleSort("rank")}
              >
                <div className="flex items-center gap-2">
                  {t("analysis.table.columns.rank")}
                  <SortIcon field="rank" />
                </div>
              </th>
              <th
                className="text-left py-3 px-4 font-medium cursor-pointer hover:bg-muted/50 transition-colors"
                onClick={() => handleSort("word")}
              >
                <div className="flex items-center gap-2">
                  {t("analysis.table.columns.word")}
                  <SortIcon field="word" />
                </div>
              </th>
              <th
                className="text-right py-3 px-4 font-medium cursor-pointer hover:bg-muted/50 transition-colors"
                onClick={() => handleSort("count")}
              >
                <div className="flex items-center justify-end gap-2">
                  {t("analysis.table.columns.count")}
                  <SortIcon field="count" />
                </div>
              </th>
              <th
                className="text-right py-3 px-4 font-medium cursor-pointer hover:bg-muted/50 transition-colors"
                onClick={() => handleSort("percentage")}
              >
                <div className="flex items-center justify-end gap-2">
                  {t("analysis.table.columns.percentage")}
                  <SortIcon field="percentage" />
                </div>
              </th>
            </tr>
          </thead>
          <tbody>
            {tableData.map((row) => (
              <tr
                key={row.word}
                className="border-b border-border/50 hover:bg-muted/30 transition-colors"
              >
                <td className="py-2 px-4 text-muted-foreground">{row.rank}</td>
                <td className="py-2 px-4 font-mono">{row.word}</td>
                <td className="py-2 px-4 text-right tabular-nums">
                  {row.count.toLocaleString()}
                </td>
                <td className="py-2 px-4 text-right tabular-nums text-muted-foreground">
                  {row.percentage.toFixed(2)}%
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </ScrollArea>

      <p className="text-xs text-muted-foreground mt-3">
        {t("analysis.table.showing", {
          count: tableData.length,
          total: stats.top_words.length,
        })}
      </p>
    </div>
  );
}
