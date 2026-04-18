/**
 * Shared hand-fan geometry.
 *
 * Used by `HandDisplayCool` to lay out the in-game hand and by the
 * mulligan modals so the cards sit on the same arc, tilt the same
 * amounts, and animate out to the same hover scale. Keeping this logic
 * in one place guarantees that tweaking the fan shape flows everywhere
 * at once.
 */

export const ARC_RADIUS = 900;
export const MAX_ARC_DEG = 30;
export const HOVER_SCALE = 1.8;

/** Base layout params at 1920px reference, keyed by size preference.
 *  The runtime `useHandScale()` hook scales these linearly to the
 *  current viewport. */
export const HAND_FAN_SIZE_PARAMS = {
  small: {
    hoverLift: 40,
    neighborPush: 30,
    maxSpread: 56,
    minSpread: 24,
    spreadWidth: 560,
  },
  medium: {
    hoverLift: 70,
    neighborPush: 48,
    maxSpread: 90,
    minSpread: 38,
    spreadWidth: 900,
  },
  large: {
    hoverLift: 90,
    neighborPush: 62,
    maxSpread: 118,
    minSpread: 50,
    spreadWidth: 1180,
  },
} as const;

export interface HandFanSlot {
  /** Horizontal offset from the fan's horizontal center, in px. */
  x: number;
  /** Extra vertical drop (outer cards dip lower than the middle), in px. */
  drop: number;
  /** Rotation in degrees (negative = tilt left). */
  rot: number;
}

/**
 * Compute per-card slots for a fan of `count` cards.
 */
export function computeHandFanLayout(
  count: number,
  cardW: number,
  maxSpread: number,
  minSpread: number,
  spreadWidth: number,
): HandFanSlot[] {
  if (count === 0) return [];
  if (count === 1) return [{ x: 0, drop: 0, rot: 0 }];

  const spread = Math.max(
    minSpread,
    Math.min(maxSpread, Math.floor((spreadWidth - cardW) / (count - 1))),
  );
  const totalWidth = (count - 1) * spread;
  const arcDeg = Math.min(MAX_ARC_DEG, count * 2.5);

  return Array.from({ length: count }, (_, i) => {
    const t = count === 1 ? 0 : (i / (count - 1)) * 2 - 1;
    const x = -totalWidth / 2 + i * spread;
    const rot = t * (arcDeg / 2);
    const drop = (1 - Math.cos((t * Math.PI) / 2)) * (ARC_RADIUS * 0.015);
    return { x, drop, rot };
  });
}
