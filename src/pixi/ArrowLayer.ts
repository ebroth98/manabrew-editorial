import { Graphics } from "pixi.js";
import type { Theme } from "@/hooks/useTheme";
import { getTheme } from "@/hooks/useTheme";
import { hexToNum, colorAlpha } from "./colorUtils";
import type { ArrowType } from "./types";

// Re-export so existing callers still import ArrowType from this module.
export type { ArrowType } from "./types";

export interface ArrowDef {
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  type: ArrowType;
}

// ── Visual tuning ──────────────────────────────────────────────────────────
const STROKE_WIDTH = 3;
const SHADOW_WIDTH = 5;
const SHADOW_OFFSET_Y = 2;
const SHADOW_ALPHA = 0.35;
const GLOW_WIDTH = 7;
const GLOW_ALPHA = 0.22;
const BEND_FACTOR = 0.22;
const TIP_SHORTEN = 12;
const TAIL_SHORTEN = 6;
const ARROWHEAD_LENGTH = 14;
const ARROWHEAD_WIDTH = 11;
const ARROWHEAD_NOTCH = 0.45; // controls the concave base for a sleek look
const ARROW_Z_INDEX = 8000;

const PLACEMENT_ARROW_ALPHA = 0.7;

// Dashed (marching-ants) placement arrow.
const DASH_BEZIER_STEPS = 64;
const DASH_LEN = 9;
const GAP_LEN = 6;
const DASH_CYCLE = DASH_LEN + GAP_LEN;
const DASH_SPEED_PX_PER_SEC = 48;

// ── Geometry helpers ───────────────────────────────────────────────────────
function unit(dx: number, dy: number): { ux: number; uy: number; len: number } {
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 1) return { ux: 0, uy: 0, len: 0 };
  return { ux: dx / len, uy: dy / len, len };
}

function controlPoint(x1: number, y1: number, x2: number, y2: number) {
  const mx = (x1 + x2) / 2;
  const my = (y1 + y2) / 2;
  const { ux, uy, len } = unit(x2 - x1, y2 - y1);
  if (len === 0) return { cx: mx, cy: my };
  // perpendicular to (ux, uy)
  const px = -uy;
  const py = ux;
  return {
    cx: mx + px * len * BEND_FACTOR,
    cy: my + py * len * BEND_FACTOR,
  };
}

function shortenLine(x1: number, y1: number, x2: number, y2: number) {
  const { ux, uy, len } = unit(x2 - x1, y2 - y1);
  if (len === 0) return { ax1: x1, ay1: y1, ax2: x2, ay2: y2 };
  return {
    ax1: x1 + ux * TAIL_SHORTEN,
    ay1: y1 + uy * TAIL_SHORTEN,
    ax2: x2 - ux * TIP_SHORTEN,
    ay2: y2 - uy * TIP_SHORTEN,
  };
}

function sampleQuadratic(
  x1: number,
  y1: number,
  cx: number,
  cy: number,
  x2: number,
  y2: number,
  steps: number,
) {
  const points: { x: number; y: number }[] = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const it = 1 - t;
    points.push({
      x: it * it * x1 + 2 * it * t * cx + t * t * x2,
      y: it * it * y1 + 2 * it * t * cy + t * t * y2,
    });
  }
  return points;
}

// ── Arrow colors from theme ────────────────────────────────────────────────
function getArrowColor(type: ArrowType, theme: Theme): { color: number; alpha: number } {
  const g = theme.gameTheme;
  switch (type) {
    case "attack":
      return { color: hexToNum(g.arrow.attack), alpha: colorAlpha(g.arrow.attack) };
    case "block":
      return { color: hexToNum(g.arrow.block), alpha: colorAlpha(g.arrow.block) };
    case "placement":
      return { color: hexToNum(g.activeAction.active), alpha: PLACEMENT_ARROW_ALPHA };
  }
}

export class ArrowLayer {
  /** Shadow + glow layers sit below the main stroke. Separate Graphics so
   *  each can hold its own stroke without bleeding into siblings. */
  private shadowGfx: Graphics;
  private glowGfx: Graphics;
  private strokeGfx: Graphics;
  private headGfx: Graphics;
  /** Container-like Graphics wrapper so the scene has a single child to own. */
  private root: Graphics;

  // Seeded synchronously from the active preset so the first tick before
  // `setTheme` fires still draws theme-correct arrows.
  private theme: Theme = getTheme();
  private arrows: ArrowDef[] = [];
  private dashOffset = 0;

  constructor() {
    this.root = new Graphics();
    this.root.zIndex = ARROW_Z_INDEX;

    // Children drawn in order: glow (bottom) → shadow → main stroke → head (top)
    this.glowGfx = new Graphics();
    this.shadowGfx = new Graphics();
    this.strokeGfx = new Graphics();
    this.headGfx = new Graphics();
    this.root.addChild(this.glowGfx);
    this.root.addChild(this.shadowGfx);
    this.root.addChild(this.strokeGfx);
    this.root.addChild(this.headGfx);
  }

  get graphics(): Graphics {
    return this.root;
  }

  setTheme(theme: Theme): void {
    this.theme = theme;
    if (this.arrows.length > 0) this.redraw();
  }

  /**
   * Set the current arrow list and advance the dash animation by `deltaMs`.
   * The scene calls this every frame so placement arrows animate at the
   * same rate as the ticker regardless of frame rate.
   */
  update(arrows: ArrowDef[], deltaMs = 0): void {
    this.dashOffset = (this.dashOffset + (deltaMs / 1000) * DASH_SPEED_PX_PER_SEC) % DASH_CYCLE;
    this.arrows = arrows;
    this.redraw();
  }

  private redraw(): void {
    this.glowGfx.clear();
    this.shadowGfx.clear();
    this.strokeGfx.clear();
    this.headGfx.clear();
    for (const arrow of this.arrows) this.drawArrow(arrow);
  }

  private drawArrow(arrow: ArrowDef): void {
    const { ax1, ay1, ax2, ay2 } = shortenLine(arrow.fromX, arrow.fromY, arrow.toX, arrow.toY);
    const { cx, cy } = controlPoint(ax1, ay1, ax2, ay2);
    const { color, alpha } = getArrowColor(arrow.type, this.theme);

    if (arrow.type === "placement") {
      this.strokePlacementPath(ax1, ay1, cx, cy, ax2, ay2, color, alpha);
    } else {
      this.strokeSolidPath(ax1, ay1, cx, cy, ax2, ay2, color, alpha);
    }

    this.drawArrowhead(ax2, ay2, cx, cy, color, alpha);
  }

  private strokeSolidPath(
    x1: number,
    y1: number,
    cx: number,
    cy: number,
    x2: number,
    y2: number,
    color: number,
    alpha: number,
  ): void {
    // Soft outer glow — widest, most transparent
    this.glowGfx.moveTo(x1, y1);
    this.glowGfx.quadraticCurveTo(cx, cy, x2, y2);
    this.glowGfx.stroke({
      color,
      width: GLOW_WIDTH,
      alpha: alpha * GLOW_ALPHA,
      cap: "round",
      join: "round",
    });

    // Offset drop shadow for depth
    this.shadowGfx.moveTo(x1, y1 + SHADOW_OFFSET_Y);
    this.shadowGfx.quadraticCurveTo(cx, cy + SHADOW_OFFSET_Y, x2, y2 + SHADOW_OFFSET_Y);
    this.shadowGfx.stroke({
      color: hexToNum(this.theme.gameTheme.canvas.shadow),
      width: SHADOW_WIDTH,
      alpha: alpha * SHADOW_ALPHA,
      cap: "round",
      join: "round",
    });

    // Main stroke
    this.strokeGfx.moveTo(x1, y1);
    this.strokeGfx.quadraticCurveTo(cx, cy, x2, y2);
    this.strokeGfx.stroke({
      color,
      width: STROKE_WIDTH,
      alpha,
      cap: "round",
      join: "round",
    });
  }

  private strokePlacementPath(
    x1: number,
    y1: number,
    cx: number,
    cy: number,
    x2: number,
    y2: number,
    color: number,
    alpha: number,
  ): void {
    const points = sampleQuadratic(x1, y1, cx, cy, x2, y2, DASH_BEZIER_STEPS);

    // Shadow-less placement: just the animated dash. Start past `dashOffset`
    // pixels so the pattern visibly marches toward the target.
    let drawing = this.dashOffset % DASH_CYCLE < DASH_LEN;
    let remaining = drawing
      ? DASH_LEN - (this.dashOffset % DASH_CYCLE)
      : DASH_CYCLE - (this.dashOffset % DASH_CYCLE);

    let prevX = points[0]!.x;
    let prevY = points[0]!.y;
    if (drawing) this.strokeGfx.moveTo(prevX, prevY);

    for (let i = 1; i < points.length; i++) {
      const px = points[i]!.x;
      const py = points[i]!.y;
      const segLen = Math.hypot(px - prevX, py - prevY);

      if (segLen <= remaining) {
        if (drawing) this.strokeGfx.lineTo(px, py);
        remaining -= segLen;
      } else {
        if (drawing) {
          this.strokeGfx.lineTo(px, py);
          this.strokeGfx.stroke({
            color,
            width: STROKE_WIDTH,
            alpha,
            cap: "round",
            join: "round",
          });
        }
        drawing = !drawing;
        remaining = drawing ? DASH_LEN : GAP_LEN;
        if (drawing) this.strokeGfx.moveTo(px, py);
      }

      prevX = px;
      prevY = py;
    }

    if (drawing) {
      this.strokeGfx.stroke({
        color,
        width: STROKE_WIDTH,
        alpha,
        cap: "round",
        join: "round",
      });
    }
  }

  private drawArrowhead(
    tipX: number,
    tipY: number,
    ctrlX: number,
    ctrlY: number,
    color: number,
    alpha: number,
  ): void {
    const { ux, uy, len } = unit(tipX - ctrlX, tipY - ctrlY);
    if (len === 0) return;
    // Perpendicular
    const px = -uy;
    const py = ux;

    const baseX = tipX - ux * ARROWHEAD_LENGTH;
    const baseY = tipY - uy * ARROWHEAD_LENGTH;
    const halfW = ARROWHEAD_WIDTH / 2;
    const notchX = baseX + ux * (ARROWHEAD_LENGTH * ARROWHEAD_NOTCH);
    const notchY = baseY + uy * (ARROWHEAD_LENGTH * ARROWHEAD_NOTCH);

    const leftX = baseX + px * halfW;
    const leftY = baseY + py * halfW;
    const rightX = baseX - px * halfW;
    const rightY = baseY - py * halfW;

    // Drop shadow beneath head
    this.headGfx.moveTo(tipX, tipY + SHADOW_OFFSET_Y);
    this.headGfx.lineTo(leftX, leftY + SHADOW_OFFSET_Y);
    this.headGfx.lineTo(notchX, notchY + SHADOW_OFFSET_Y);
    this.headGfx.lineTo(rightX, rightY + SHADOW_OFFSET_Y);
    this.headGfx.closePath();
    this.headGfx.fill({ color: this.theme.gameTheme.canvas.shadow, alpha: alpha * SHADOW_ALPHA });

    // Filled arrowhead with a subtle concave base for a sleek silhouette.
    this.headGfx.moveTo(tipX, tipY);
    this.headGfx.lineTo(leftX, leftY);
    this.headGfx.lineTo(notchX, notchY);
    this.headGfx.lineTo(rightX, rightY);
    this.headGfx.closePath();
    this.headGfx.fill({ color, alpha });
  }

  destroy(): void {
    this.glowGfx.destroy({ children: true });
    this.shadowGfx.destroy({ children: true });
    this.strokeGfx.destroy({ children: true });
    this.headGfx.destroy({ children: true });
    this.root.destroy({ children: true });
    this.arrows = [];
  }
}
