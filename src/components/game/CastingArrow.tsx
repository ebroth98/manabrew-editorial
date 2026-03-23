import { useEffect, useState, useCallback } from "react";
import { createPortal } from "react-dom";
import { useGameThemeColors } from "./game.theme";

interface CastingArrowProps {
  /** Card ID used to locate the source element via data-casting-card attribute. */
  castingCardId: string;
  /** When set, arrow points to this target element instead of the cursor. */
  targetId?: string | null;
  /** Whether the target is hostile (opponent) or friendly (self). */
  hostile?: boolean;
}

const STROKE_WIDTH = 3;
const BEND_FACTOR = 0.18;

function controlPoint(x1: number, y1: number, x2: number, y2: number) {
  const mx = (x1 + x2) / 2;
  const my = (y1 + y2) / 2;
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 1) return { cx: mx, cy: my };
  const px = -dy / len;
  const py = dx / len;
  return { cx: mx + px * len * BEND_FACTOR, cy: my + py * len * BEND_FACTOR };
}

function getElementCenter(el: HTMLElement): { x: number; y: number } {
  const rect = el.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

function findTargetElement(targetId: string): HTMLElement | null {
  return (
    document.querySelector(`[data-card-id="${targetId}"]`) as HTMLElement ??
    document.querySelector(`[data-player-id="${targetId}"]`) as HTMLElement ??
    null
  );
}

export function CastingArrow({ castingCardId, targetId, hostile }: CastingArrowProps) {
  const themeColors = useGameThemeColors();
  const color = hostile ? themeColors.arrow.hostileTarget : themeColors.arrow.friendlyTarget;

  const [mouse, setMouse] = useState<{ x: number; y: number } | null>(null);
  const [from, setFrom] = useState<{ x: number; y: number } | null>(null);
  const [targetPos, setTargetPos] = useState<{ x: number; y: number } | null>(null);

  const updatePositions = useCallback(() => {
    const sourceEl = document.querySelector(`[data-casting-card="${castingCardId}"]`) as HTMLElement | null;
    if (sourceEl) {
      setFrom(getElementCenter(sourceEl));
    }
    if (targetId) {
      const targetEl = findTargetElement(targetId);
      if (targetEl) {
        setTargetPos(getElementCenter(targetEl));
      }
    }
  }, [castingCardId, targetId]);

  useEffect(() => {
    updatePositions();
    const onMove = (e: MouseEvent) => {
      setMouse({ x: e.clientX, y: e.clientY });
    };
    const onResize = () => updatePositions();
    window.addEventListener("mousemove", onMove);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("resize", onResize);
    };
  }, [updatePositions]);

  // Re-measure positions periodically (stack/layout animations)
  useEffect(() => {
    const id = setInterval(updatePositions, 200);
    return () => clearInterval(id);
  }, [updatePositions]);

  // Determine the arrow endpoint: locked target or cursor
  const to = targetId ? targetPos : mouse;

  if (!from || !to) return null;

  const { cx, cy } = controlPoint(from.x, from.y, to.x, to.y);
  const d = `M ${from.x},${from.y} Q ${cx},${cy} ${to.x},${to.y}`;
  const markerId = "casting-arrow-head";

  return createPortal(
    <svg
      className="fixed inset-0 w-full h-full pointer-events-none"
      style={{ zIndex: 9998 }}
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <defs>
        <marker
          id={markerId}
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="5"
          markerHeight="5"
          orient="auto"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill={color} />
        </marker>
        <filter id="casting-arrow-shadow" x="-20%" y="-20%" width="140%" height="140%">
          <feDropShadow dx="0" dy="1" stdDeviation="1.5" floodOpacity="0.45" />
        </filter>
      </defs>
      <g filter="url(#casting-arrow-shadow)">
        <path
          d={d}
          stroke={color}
          strokeWidth={STROKE_WIDTH}
          fill="none"
          strokeLinecap="round"
          markerEnd={`url(#${markerId})`}
        />
      </g>
    </svg>,
    document.body,
  );
}
