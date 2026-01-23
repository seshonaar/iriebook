import { useState, useCallback, useRef, useEffect } from "react";
import { commands } from "../bindings";
import {
  clearGlobalCoverCache,
  deleteGlobalCoverCache,
  getGlobalCoverCache,
  hasGlobalCoverCache,
  isLoadingGlobalCover,
  removeGlobalLoadingCover,
  setGlobalCoverCache,
  addGlobalLoadingCover,
} from "./coverCache";

// Hook to load and cache cover images
export function useCoverImage() {
  const [, forceUpdate] = useState({});
  const updateRef = useRef(() => {
    forceUpdate({});
  });

  const loadCover = useCallback(async (coverPath: string): Promise<string | null> => {
    // Check cache first
    if (hasGlobalCoverCache(coverPath)) {
      return getGlobalCoverCache(coverPath);
    }

    // Check if already loading
    if (isLoadingGlobalCover(coverPath)) {
      return null;
    }

    // Mark as loading
    addGlobalLoadingCover(coverPath);

    try {
      const result = await commands.loadCoverImage(coverPath);
      if (result.status === "error") {
        throw new Error(result.error);
      }
      const coverData = result.data;

      // Update cache
      setGlobalCoverCache(coverPath, coverData.data_url);

      // Remove from loading set
      removeGlobalLoadingCover(coverPath);

      return coverData.data_url;
    } catch (error) {
      console.error("Failed to load cover for", coverPath, error);
      // Remove from loading set
      removeGlobalLoadingCover(coverPath);
      return null;
    }
  }, []);

  const getCachedCover = useCallback((bookPath: string): string | null => {
    return getGlobalCoverCache(bookPath);
  }, []);

  const isLoadingCover = useCallback((bookPath: string): boolean => {
    return isLoadingGlobalCover(bookPath);
  }, []);

  const clearCache = useCallback(() => {
    clearGlobalCoverCache();
    // Force re-render of all components using this hook
    updateRef.current();
  }, []);

  const invalidateCover = useCallback((bookPath: string) => {
    deleteGlobalCoverCache(bookPath);
    updateRef.current();
  }, []);

  return {
    loadCover,
    getCachedCover,
    isLoadingCover,
    clearCache,
    invalidateCover,
  };
}
