import { Graphics } from "pixi.js";
import type { PixiThemeColors } from "./themeAdapter";

export type ArrowType = "attack" | "block" | "hostile-target" | "friendly-target" | "placement";

export interface ArrowDef {
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  type: ArrowType;
}

const STROKE_WIDTH = 3.5;
const BEND_FACTOR = 0.22;
const TIP_SHORTEN = 10;
const TAIL_SHORTEN = 6;
const ARROWHEAD_SIZE = 10;
const ARROWHEAD_HALF_RATIO = 0.45;
const ARROW_Z_INDEX = 8000;
const ARROW_FALLBACK_COLOR = 0xffffff;
const ARROW_FALLBACK_ALPHA = 0.8;
const PLACEMENT_ARROW_ALPHA = 0.6;
const DASH_BEZIER_STEPS = 60;
const DASH_LEN = 8;
const GAP_LEN = 6;

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

function shortenLine(x1: number, y1: number, x2: number, y2: number) {
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

function getArrowColor(type: ArrowType, theme: PixiThemeColors | null): { color: number; alpha: number } {
  if (!theme) return { color: ARROW_FALLBACK_COLOR, alpha: ARROW_FALLBACK_ALPHA };
  switch (type) {
    case "attack": return theme.arrow.attack;
    case "block": return theme.arrow.block;
    case "hostile-target": return theme.arrow.hostileTarget;
    case "friendly-target": return theme.arrow.friendlyTarget;
    case "placement": return { color: theme.activeAction.active, alpha: PLACEMENT_ARROW_ALPHA };
  }
}

export class ArrowLayer {
  private gfx: Graphics;
  private theme: PixiThemeColors | null = null;

  constructor() {
    this.gfx = new Graphics();
    this.gfx.zIndex = ARROW_Z_INDEX;
  }

  get graphics(): Graphics {
    return this.gfx;
  }

  setTheme(theme: PixiThemeColors): void {
    this.theme = theme;
  }

  update(arrows: ArrowDef[]): void {
    this.gfx.clear();

    for (const arrow of arrows) {
      const { ax1, ay1, ax2, ay2 } = shortenLine(arrow.fromX, arrow.fromY, arrow.toX, arrow.toY);
      const { cx, cy } = controlPoint(ax1, ay1, ax2, ay2);
      const { color, alpha } = getArrowColor(arrow.type, this.theme);

      if (arrow.type === "placement") {
        this.drawDashedQuadratic(ax1, ay1, cx, cy, ax2, ay2, color, alpha);
      } else {
        this.gfx.moveTo(ax1, ay1);
        this.gfx.quadraticCurveTo(cx, cy, ax2, ay2);
        this.gfx.stroke({ color, width: STROKE_WIDTH, alpha });
      }

      this.drawArrowhead(ax2, ay2, cx, cy, color, alpha);
    }
  }

  private drawArrowhead(
    tipX: number, tipY: number,
    ctrlX: number, ctrlY: number,
    color: number, alpha: number,
  ): void {
    const dx = tipX - ctrlX;
    const dy = tipY - ctrlY;
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 1) return;
    const ux = dx / len;
    const uy = dy / len;
    const px = -uy;
    const py = ux;

    const baseX = tipX - ux * ARROWHEAD_SIZE;
    const baseY = tipY - uy * ARROWHEAD_SIZE;
    const halfW = ARROWHEAD_SIZE * ARROWHEAD_HALF_RATIO;

    this.gfx.moveTo(tipX, tipY);
    this.gfx.lineTo(baseX + px * halfW, baseY + py * halfW);
    this.gfx.lineTo(baseX - px * halfW, baseY - py * halfW);
    this.gfx.closePath();
    this.gfx.fill({ color, alpha });
  }

  private drawDashedQuadratic(
    x1: number, y1: number,
    cx: number, cy: number,
    x2: number, y2: number,
    color: number, alpha: number,
    dashLen = DASH_LEN, gapLen = GAP_LEN,
  ): void {
    const steps = DASH_BEZIER_STEPS;
    const points: { x: number; y: number }[] = [];
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      const it = 1 - t;
      points.push({
        x: it * it * x1 + 2 * it * t * cx + t * t * x2,
        y: it * it * y1 + 2 * it * t * cy + t * t * y2,
      });
    }

    let drawing = true;
    let remaining = dashLen;
    let prevX = points[0]!.x;
    let prevY = points[0]!.y;
    this.gfx.moveTo(prevX, prevY);

    for (let i = 1; i < points.length; i++) {
      const px = points[i]!.x;
      const py = points[i]!.y;
      const segDx = px - prevX;
      const segDy = py - prevY;
      const segLen = Math.sqrt(segDx * segDx + segDy * segDy);

      if (segLen <= remaining) {
        if (drawing) {
          this.gfx.lineTo(px, py);
        } else {
          this.gfx.moveTo(px, py);
        }
        remaining -= segLen;
      } else {
        if (drawing) {
          this.gfx.lineTo(px, py);
          this.gfx.stroke({ color, width: STROKE_WIDTH, alpha });
        }
        drawing = !drawing;
        remaining = drawing ? dashLen : gapLen;
        this.gfx.moveTo(px, py);
      }

      prevX = px;
      prevY = py;
    }

    if (drawing) {
      this.gfx.stroke({ color, width: STROKE_WIDTH, alpha });
    }
  }

  destroy(): void {
    this.gfx.destroy({ children: true });
  }
}
