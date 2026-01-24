import { ScrollArea } from "./ui/scroll-area";
import type { DiffTab } from "../contexts/AppContext";

interface DiffViewerProps {
  tab: DiffTab;
}

export function DiffViewer({ tab }: DiffViewerProps) {
  const { diffData } = tab;
  const { segments, stats } = diffData.diff;

  // Group segments into aligned rows based on newlines in 'unchanged' segments
  const rows: Array<{
    left: Array<{ text: string; type: "removed" | "unchanged" }>;
    right: Array<{ text: string; type: "added" | "unchanged" }>;
    contextHeader?: string;
  }> = [];

  let currentLeft: Array<{ text: string; type: "removed" | "unchanged" }> = [];
  let currentRight: Array<{ text: string; type: "added" | "unchanged" }> = [];
  let lastContext: string | null = null;

  const commitRow = () => {
    if (currentLeft.length > 0 || currentRight.length > 0) {
      rows.push({
        left: [...currentLeft],
        right: [...currentRight],
      });
      currentLeft = [];
      currentRight = [];
    }
  };

  for (const segment of segments) {
    // Skip content trimmed markers entirely as requested
    if (segment.text.includes('[content trimmed]')) {
      continue;
    }

    // Check for context change
    if (segment.context_header !== lastContext) {
      commitRow();
      
      // Add a header row
      // We check if it's actually a new header or just the start
      // Note: We might want to skip "None" headers if they are just at the start before any header
      if (segment.context_header) {
        rows.push({
          left: [],
          right: [],
          contextHeader: segment.context_header,
        });
      }
      lastContext = segment.context_header;
    }

    if (segment.segment_type === "unchanged") {
      // Split unchanged text by newlines to create sync points (rows)
      const parts = segment.text.split('\n');
      
      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const token = { text: part, type: "unchanged" as const };
        
        currentLeft.push(token);
        currentRight.push(token);

        // If this is not the last part, it means we hit a newline in the original text.
        // Commit the current accumulated row and start a new one.
        if (i < parts.length - 1) {
          commitRow();
        }
      }
    } else if (segment.segment_type === "removed") {
      currentLeft.push({ text: segment.text, type: "removed" });
    } else if (segment.segment_type === "added") {
      currentRight.push({ text: segment.text, type: "added" });
    }
  }
  
  // Commit any remaining buffered segments
  commitRow();

  // Helper to render a list of segments
  const renderSegments = (segs: Array<{ text: string; type: string }>) => {
    return segs.map((seg, idx) => {
      return (
        <span
          key={idx}
          className={
            seg.type === "removed"
              ? "bg-red-100 text-red-900 dark:bg-red-950 dark:text-red-200 rounded-sm px-0.5 mx-px"
              : seg.type === "added"
              ? "bg-green-100 text-green-900 dark:bg-green-950 dark:text-green-200 rounded-sm px-0.5 mx-px"
              : ""
          }
        >
          {seg.text}
        </span>
      );
    });
  };

  // Group rows into sections for proper sticky behavior
  const sections: Array<{
    header: string | undefined;
    rows: typeof rows;
  }> = [];

  let currentSectionRows: typeof rows = [];
  let currentHeader: string | undefined = undefined;

  for (const row of rows) {
    if (row.contextHeader !== undefined) {
      // New header found - commit previous section if it has content
      if (currentSectionRows.length > 0 || currentHeader !== undefined) {
        sections.push({
          header: currentHeader,
          rows: currentSectionRows,
        });
      }
      currentHeader = row.contextHeader;
      currentSectionRows = [];
    } else {
      currentSectionRows.push(row);
    }
  }
  // Commit final section
  if (currentSectionRows.length > 0 || currentHeader !== undefined) {
    sections.push({
      header: currentHeader,
      rows: currentSectionRows,
    });
  }

  return (
    <div className="flex flex-col h-full min-h-0" data-testid="diff-view">
      {/* Header with file info and stats */}
      <div className="px-4 py-3 border-b border-border bg-muted/30 flex-none">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h3 className="font-medium text-foreground">{tab.title}</h3>
          </div>
          <div className="flex items-center gap-4 text-xs text-muted-foreground">
            <span className="text-green-600 dark:text-green-400">+{stats.added}</span>
            <span className="text-red-600 dark:text-red-400">-{stats.removed}</span>
          </div>
        </div>
      </div>

      {/* Fixed Column Headers */}
      <div className="grid grid-cols-2 divide-x divide-border border-b border-border bg-muted/50 flex-none">
        <div className="px-4 py-2">
          <h4 className="text-sm font-medium text-muted-foreground truncate">
            {diffData.left_display_name}
          </h4>
        </div>
        <div className="px-4 py-2">
          <h4 className="text-sm font-medium text-muted-foreground truncate">
            {diffData.right_display_name}
          </h4>
        </div>
      </div>

      {/* Synchronized Scroll Area */}
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col pb-4">
          {sections.map((section, sectionIdx) => (
            <div key={sectionIdx} className="relative">
              {section.header && (
                <div className="sticky top-0 z-10 bg-muted/50 backdrop-blur supports-[backdrop-filter]:bg-muted/30 border-y border-border px-4 py-1 text-xs font-semibold text-muted-foreground text-center shadow-sm uppercase tracking-wider">
                  {section.header.replace(/^#+\s*/, '')}
                </div>
              )}
              {section.rows.map((row, rowIdx) => (
                <div key={rowIdx} className="grid grid-cols-2 divide-x divide-border">
                  {/* Left Cell */}
                  <div className="px-4 py-1 font-mono text-sm whitespace-pre-wrap overflow-hidden min-h-[1.5em]" data-testid="diff-left">
                    {renderSegments(row.left)}
                  </div>
                  {/* Right Cell */}
                  <div className="px-4 py-1 font-mono text-sm whitespace-pre-wrap overflow-hidden min-h-[1.5em]" data-testid="diff-right">
                    {renderSegments(row.right)}
                  </div>
                </div>
              ))}
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
