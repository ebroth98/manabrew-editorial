/**
 * ArrowOverlay — SVG overlay for rendering targeting/combat arrows.
 *
 * Inspired by Forge desktop's TargetingOverlay.java.
 * Draws curved quadratic-bezier arrows with filled arrowheads.
 *
 * Arrow types and colours are themable via game theme settings.
 */

import { useGameThemeColors } from "./game.theme";

/** Visual category that controls color and semantics. */
export type ArrowType = "attack" | "block" | "hostile-target" | "friendly-target";

/** A single arrow to render, in container-relative pixel coordinates. */
export interface ArrowDef {
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  type: ArrowType;
}

// ─── Visual constants ────────────────────────────────────────────────────────

const STROKE_WIDTH = 3.5;
const BEND_FACTOR = 0.22;      // fraction of line length for bezier control-point offset
const TIP_SHORTEN = 10;        // px trimmed from arrowhead end (avoids overlapping target)
const TAIL_SHORTEN = 6;        // px trimmed from tail end

const DEFAULT_ARROW_COLORS: Record<ArrowType, string> = {
  attack: "rgba(255, 138, 0, 0.88)",
  block: "rgba(210, 40, 40, 0.88)",
  "hostile-target": "rgba(210, 40, 40, 0.88)",
  "friendly-target": "rgba(90, 150, 255, 0.88)",
};

// Stable marker IDs per arrow type (must be unique in the SVG <defs>)
const MARKER_IDS: Record<ArrowType, string> = {
  attack: "ao-attack",
  block: "ao-block",
  "hostile-target": "ao-hostile",
  "friendly-target": "ao-friendly",
};

// ─── Internal helpers ────────────────────────────────────────────────────────

/**
 * Compute the quadratic bezier control point that gives the arrow a slight
 * perpendicular bend, matching Forge's visual style.
 */
function controlPoint(
  x1: number, y1: number,
  x2: number, y2: number,
) {
  const mx = (x1 + x2) / 2;
  const my = (y1 + y2) / 2;
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 1) return { cx: mx, cy: my };
  // Perpendicular unit vector
  const px = -dy / len;
  const py = dx / len;
  return { cx: mx + px * len * BEND_FACTOR, cy: my + py * len * BEND_FACTOR };
}

/**
 * Return adjusted (fromX,fromY) and (toX,toY) after trimming the line
 * at both ends so arrows don't overlap the source/target elements.
 */
function shortenLine(
  x1: number, y1: number,
  x2: number, y2: number,
) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 1) return { ax1: x1, ay1: y1, ax2: x2, ay2: y2 };
  const ux = dx / len;
  const uy = dy / len;
  return {
    ax1: x1 + ux * TAIL_SHORTEN,
    ay1: y1 + uy * TAIL_SHORTEN,
    ax2: x2 - ux * TIP_SHORTEN,
    ay2: y2 - uy * TIP_SHORTEN,
  };
}

// ─── Sub-components ──────────────────────────────────────────────────────────

/** SVG <marker> element for one arrow colour. */
function ArrowheadMarker({ id, color }: { id: string; color: string }) {
  return (
    <marker
      id={id}
      viewBox="0 0 10 10"
      refX="9"
      refY="5"
      markerWidth="5"
      markerHeight="5"
      orient="auto"
    >
      {/* triangle: base on the left, tip at (10,5) */}
      <path d="M 0 0 L 10 5 L 0 10 z" fill={color} />
    </marker>
  );
}

/** One curved arrow path. */
function ArrowPath({ arrow, colors }: { arrow: ArrowDef; colors: Record<ArrowType, string> }) {
  const { fromX, fromY, toX, toY, type } = arrow;
  const color = colors[type];
  const { ax1, ay1, ax2, ay2 } = shortenLine(fromX, fromY, toX, toY);
  const { cx, cy } = controlPoint(ax1, ay1, ax2, ay2);
  const d = `M ${ax1},${ay1} Q ${cx},${cy} ${ax2},${ay2}`;
  return (
    <path
      d={d}
      stroke={color}
      strokeWidth={STROKE_WIDTH}
      fill="none"
      strokeLinecap="round"
      markerEnd={`url(#${MARKER_IDS[type]})`}
    />
  );
}

// ─── Public component ────────────────────────────────────────────────────────

/**
 * Absolutely-positioned SVG overlay that draws targeting/combat arrows.
 *
 * Place inside a `position: relative` container. The SVG fills that container
 * with `pointer-events: none` so it never blocks user interaction.
 */
export function ArrowOverlay({ arrows }: { arrows: ArrowDef[] }) {
  const themeColors = useGameThemeColors();
  const arrowColors: Record<ArrowType, string> = {
    attack: themeColors.arrow?.attack ?? DEFAULT_ARROW_COLORS.attack,
    block: themeColors.arrow?.block ?? DEFAULT_ARROW_COLORS.block,
    "hostile-target": themeColors.arrow?.hostileTarget ?? DEFAULT_ARROW_COLORS["hostile-target"],
    "friendly-target": themeColors.arrow?.friendlyTarget ?? DEFAULT_ARROW_COLORS["friendly-target"],
  };

  if (arrows.length === 0) return null;

  return (
    <svg
      className="absolute inset-0 w-full h-full pointer-events-none"
      style={{ zIndex: 40 }}
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <defs>
        {(Object.keys(arrowColors) as ArrowType[]).map((type) => (
          <ArrowheadMarker
            key={type}
            id={MARKER_IDS[type]}
            color={arrowColors[type]}
          />
        ))}
        {/* Drop-shadow filter for readability against any background */}
        <filter id="ao-shadow" x="-20%" y="-20%" width="140%" height="140%">
          <feDropShadow dx="0" dy="1" stdDeviation="1.5" floodOpacity="0.45" />
        </filter>
      </defs>

      <g filter="url(#ao-shadow)">
        {arrows.map((arrow, i) => (
          <ArrowPath key={i} arrow={arrow} colors={arrowColors} />
        ))}
      </g>
    </svg>
  );
}
