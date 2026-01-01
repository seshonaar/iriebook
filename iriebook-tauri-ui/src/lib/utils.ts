import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"
import { BookInfo } from "../bindings"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function sortBooks(books: BookInfo[]): BookInfo[] {
  return [...books].sort((a, b) => {
    // 1. Author
    const authorA = a.metadata?.author || "";
    const authorB = b.metadata?.author || "";
    const authorCompare = authorA.localeCompare(authorB);
    if (authorCompare !== 0) return authorCompare;

    // 2. Series (belongs-to-collection)
    const seriesA = a.metadata?.["belongs-to-collection"] || "";
    const seriesB = b.metadata?.["belongs-to-collection"] || "";
    
    // If both have series, compare them
    // If one is empty, it usually comes before (or after? lets say empty series (standalone) comes before named series for same author?
    // Actually standard localeCompare handles empty string coming before letters.
    const seriesCompare = seriesA.localeCompare(seriesB);
    if (seriesCompare !== 0) return seriesCompare;

    // 3. Series Index (group-position)
    // If null, treat as infinity so it goes to the end
    const indexA = a.metadata?.["group-position"] ?? Number.MAX_VALUE;
    const indexB = b.metadata?.["group-position"] ?? Number.MAX_VALUE;
    
    if (indexA !== indexB) {
      return indexA - indexB;
    }
    
    // 4. Fallback to title
    const titleA = a.metadata?.title || a.display_name;
    const titleB = b.metadata?.title || b.display_name;
    return titleA.localeCompare(titleB);
  });
}
