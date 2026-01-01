import { useState, useCallback, useRef } from "react";
import { commands } from "../bindings";

// Hook to load and cache cover images
export function useCoverImage() {
  const [coverCache, setCoverCache] = useState<Map<string, string>>(new Map());
  const [loadingCovers, setLoadingCovers] = useState<Set<string>>(new Set());

  // Use refs to avoid stale closures
  const cacheRef = useRef(coverCache);
  const loadingRef = useRef(loadingCovers);

  // Keep refs in sync
  cacheRef.current = coverCache;
  loadingRef.current = loadingCovers;

  const loadCover = useCallback(async (coverPath: string): Promise<string | null> => {
    // Check cache first
    if (cacheRef.current.has(coverPath)) {
      return cacheRef.current.get(coverPath) || null;
    }

    // Check if already loading
    if (loadingRef.current.has(coverPath)) {
      return null;
    }

    // Mark as loading
    setLoadingCovers((prev) => new Set(prev).add(coverPath));

    try {
      const result = await commands.loadCoverImage(coverPath);
      if (result.status === "error") {
        throw new Error(result.error);
      }
      const coverData = result.data;

      // Update cache
      setCoverCache((prev) => {
        const newCache = new Map(prev);
        newCache.set(coverPath, coverData.data_url);
        return newCache;
      });

      // Remove from loading set
      setLoadingCovers((prev) => {
        const newSet = new Set(prev);
        newSet.delete(coverPath);
        return newSet;
      });

      return coverData.data_url;
    } catch (error) {
      console.error("Failed to load cover for", coverPath, error);
      // Remove from loading set
      setLoadingCovers((prev) => {
        const newSet = new Set(prev);
        newSet.delete(coverPath);
        return newSet;
      });
      return null;
    }
  }, []);

  const getCachedCover = useCallback((bookPath: string): string | null => {
    return cacheRef.current.get(bookPath) || null;
  }, []);

  const isLoadingCover = useCallback((bookPath: string): boolean => {
    return loadingRef.current.has(bookPath);
  }, []);

  const clearCache = useCallback(() => {
    setCoverCache(new Map());
    setLoadingCovers(new Set());
  }, []);

  const invalidateCover = useCallback((bookPath: string) => {
    setCoverCache((prev) => {
      const newCache = new Map(prev);
      newCache.delete(bookPath);
      return newCache;
    });
  }, []);

  return {
    loadCover,
    getCachedCover,
    isLoadingCover,
    clearCache,
    invalidateCover,
  };
}
