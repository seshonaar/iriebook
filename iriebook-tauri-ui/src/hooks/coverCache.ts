// Shared cache state for cover images
// This allows useProcessingEvents to clear cache across all useCoverImage instances
const globalCoverCache = new Map<string, string>();
const globalLoadingCovers = new Set<string>();

export function clearGlobalCoverCache() {
  globalCoverCache.clear();
  globalLoadingCovers.clear();
}

export function setGlobalCoverCache(key: string, value: string) {
  globalCoverCache.set(key, value);
}

export function getGlobalCoverCache(key: string): string | null {
  return globalCoverCache.get(key) || null;
}

export function hasGlobalCoverCache(key: string): boolean {
  return globalCoverCache.has(key);
}

export function deleteGlobalCoverCache(key: string) {
  globalCoverCache.delete(key);
}

export function addGlobalLoadingCover(key: string) {
  globalLoadingCovers.add(key);
}

export function removeGlobalLoadingCover(key: string) {
  globalLoadingCovers.delete(key);
}

export function isLoadingGlobalCover(key: string): boolean {
  return globalLoadingCovers.has(key);
}
