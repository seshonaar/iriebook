import { useTranslation } from "react-i18next";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts";
import type { WordAnalysisStats } from "../../bindings";

interface WordFrequencyChartProps {
  stats: WordAnalysisStats;
}

export function WordFrequencyChart({ stats }: WordFrequencyChartProps) {
  const { t } = useTranslation();

  // Take top 20 words and format for the chart
  const chartData = stats.top_words.slice(0, 20).map(([word, count]: [string, number], index: number) => ({
    word,
    count,
    index,
  }));

  // Generate gradient colors from primary to muted
  const getBarColor = (index: number) => {
    const hue = 220; // Blue base
    const saturation = Math.max(30, 70 - index * 2);
    const lightness = Math.min(65, 45 + index);
    return `hsl(${hue}, ${saturation}%, ${lightness}%)`;
  };

  if (chartData.length === 0) {
    return null;
  }

  return (
    <div className="bg-card border border-border rounded-lg p-4">
      <h3 className="text-lg font-semibold mb-4">{t("analysis.chart.title")}</h3>
      <div className="h-[400px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart
            data={chartData}
            layout="vertical"
            margin={{ top: 5, right: 30, left: 80, bottom: 5 }}
          >
            <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
            <XAxis
              type="number"
              tick={{ fontSize: 12 }}
              className="text-muted-foreground"
            />
            <YAxis
              type="category"
              dataKey="word"
              tick={{ fontSize: 12 }}
              width={75}
              className="text-muted-foreground"
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "hsl(var(--card))",
                border: "1px solid hsl(var(--border))",
                borderRadius: "6px",
              }}
              labelStyle={{ color: "hsl(var(--foreground))" }}
              formatter={(value) => [
                typeof value === "number" ? value.toLocaleString() : value,
                t("analysis.chart.wordCount"),
              ]}
            />
            <Bar dataKey="count" radius={[0, 4, 4, 0]}>
              {chartData.map((entry: { word: string; count: number; index: number }) => (
                <Cell key={entry.word} fill={getBarColor(entry.index)} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
