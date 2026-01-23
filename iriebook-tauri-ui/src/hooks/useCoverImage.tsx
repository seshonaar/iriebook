import { useCallback } from "react";
import { useAppContext } from "../contexts/AppContext";
import { setCoverStatus, clearCoverStatus } from "../contexts/actions";

/**
 * Hook for cover image operations.
 *
 * Most cover state is now managed in AppContext. This hook provides
 * helper functions for invalidating covers (triggering reload).
 */
export function useCoverImage() {
  const { dispatch } = useAppContext();

  /**
   * Clear all cover status from context, forcing all covers to reload
   */
  const clearCache = useCallback(() => {
    dispatch(clearCoverStatus());
  }, [dispatch]);

  /**
   * Invalidate a specific cover, triggering reload on next render
   */
  const invalidateCover = useCallback((coverPath: string) => {
    dispatch(setCoverStatus(coverPath, { type: "not_started" }));
  }, [dispatch]);

  return {
    clearCache,
    invalidateCover,
  };
}
