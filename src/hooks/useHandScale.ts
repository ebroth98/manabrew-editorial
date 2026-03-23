import { useSyncExternalStore } from "react";

/** Reference viewport width — sizes are authored at this width. */
const REF_WIDTH = 1920;

/** Clamp the scale so cards don't get absurdly tiny or huge. */
const MIN_SCALE = 0.45;
const MAX_SCALE = 1.3;

function getScale() {
  const s = window.innerWidth / REF_WIDTH;
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, s));
}

function subscribe(cb: () => void) {
  const handler = () => cb();
  window.addEventListener("resize", handler);
  return () => window.removeEventListener("resize", handler);
}

function getSnapshot() {
  return getScale();
}

/**
 * Returns a multiplier (0.45–1.3) that scales hand card sizes
 * proportionally to the viewport width, using 1920px as 1×.
 */
export function useHandScale() {
  return useSyncExternalStore(subscribe, getSnapshot, () => 1);
}
