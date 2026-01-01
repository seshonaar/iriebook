import { useTranslation } from "react-i18next";
import { FileText, Fingerprint, Percent, Filter } from "lucide-react";
import type { WordAnalysisStats } from "../../bindings";

interface StatisticsCardsProps {
  stats: WordAnalysisStats;
}

interface StatCardProps {
  title: string;
  value: string | number;
  icon: React.ReactNode;
  description?: string;
}

function StatCard({ title, value, icon, description }: StatCardProps) {
  return (
    <div className="bg-card border border-border rounded-lg p-4 flex items-start gap-4">
      <div className="p-2 bg-primary/10 rounded-lg text-primary">
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-muted-foreground">{title}</p>
        <p className="text-2xl font-bold tracking-tight">{value}</p>
        {description && (
          <p className="text-xs text-muted-foreground mt-1">{description}</p>
        )}
      </div>
    </div>
  );
}

export function StatisticsCards({ stats }: StatisticsCardsProps) {
  const { t } = useTranslation();

  const vocabularyRichness = stats.total_words > 0
    ? ((stats.unique_words / stats.total_words) * 100).toFixed(1)
    : "0.0";

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <StatCard
        title={t("analysis.stats.totalWords")}
        value={stats.total_words.toLocaleString()}
        icon={<FileText className="h-5 w-5" />}
      />
      <StatCard
        title={t("analysis.stats.uniqueWords")}
        value={stats.unique_words.toLocaleString()}
        icon={<Fingerprint className="h-5 w-5" />}
      />
      <StatCard
        title={t("analysis.stats.vocabularyRichness")}
        value={`${vocabularyRichness}%`}
        icon={<Percent className="h-5 w-5" />}
      />
      <StatCard
        title={t("analysis.stats.stopwordsExcluded")}
        value={stats.excluded_count.toLocaleString()}
        icon={<Filter className="h-5 w-5" />}
      />
    </div>
  );
}
