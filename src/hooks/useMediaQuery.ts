import { useCallback, useSyncExternalStore } from "react";

/**
 * Subscribe to a CSS media query and re-render when it flips.
 *
 * Used to gate React subtrees that would otherwise be mounted twice
 * (once per responsive branch) — Pixi canvases especially, where each
 * mounted instance burns a WebGL context against the browser's per-tab
 * cap (WebKit ≈ 8, Chrome ≈ 16).
 *
 * Implemented via `useSyncExternalStore` so the matchMedia subscription
 * lives entirely in the external-store callbacks — no in-effect setState
 * needed, and the snapshot is read synchronously on every commit so the
 * first paint already picks the right branch.
 */
export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onChange: () => void) => {
      const mq = window.matchMedia(query);
      mq.addEventListener("change", onChange);
      return () => mq.removeEventListener("change", onChange);
    },
    [query],
  );
  const getSnapshot = useCallback(() => window.matchMedia(query).matches, [query]);
  const getServerSnapshot = () => false;
  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}
