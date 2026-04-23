/**
 * Horizontal phase strip rendered in Pixi at the vertical center of the canvas.
 * Shows the current phase, enabled stops, and supports click-to-toggle.
 */

import { Container, Graphics, Text, TextStyle, Sprite } from "pixi.js";
import type { PixiThemeColors } from "./themeAdapter";
import { applyIcon, ICON_COLORS } from "./panelIcons";

/** Display cells. "combat" is a merged cell that represents all combat sub-phases. */
interface PhaseSpec {
  id: string;
  short: string;
  /** If set, this cell represents multiple phase ids (combat). */
  subPhases?: string[];
}

const COMBAT_SUB_PHASES = [
  "begin_combat", "declare_attackers", "declare_blockers",
  "first_strike_damage", "combat_damage", "end_combat",
];

const COMBAT_LABELS: Record<string, string> = {
  begin_combat: "BC",
  declare_attackers: "ATK",
  declare_blockers: "BLK",
  first_strike_damage: "1ST",
  combat_damage: "DMG",
  end_combat: "EC",
};

const PHASES: PhaseSpec[] = [
  { id: "upkeep", short: "UP" },
  { id: "draw", short: "DR" },
  { id: "main1", short: "M1" },
  { id: "combat", short: "COMBAT", subPhases: COMBAT_SUB_PHASES },
  { id: "main2", short: "M2" },
  { id: "end", short: "END" },
  { id: "cleanup", short: "CL" },
];

const CELL_W = 60;
const COMBAT_CELL_W = 84;
const CELL_H = 30;
const CELL_GAP = 5;
const CELL_R = 4;
const COMBAT_ICON_SIZE = 16;
const FONT = "system-ui, -apple-system, sans-serif";

const FLASH_DURATION_MS = 800;
const FLASH_MAX_EXPAND = 8;

function easeOut(t: number): number {
  const t1 = 1 - t;
  return 1 - t1 * t1 * t1;
}

// ── Indicator shapes for self (bottom) and opponents (top) ────────────
const INDICATOR_SIZE = 10;
const INDICATOR_GAP = 3;
const INDICATOR_MARGIN = 4; // distance from cell edge


type ShapeKind = "triangle" | "diamond" | "circle";
const OPPONENT_SHAPES: ShapeKind[] = ["triangle", "diamond", "circle"];

function drawShape(
  gfx: Graphics,
  kind: ShapeKind,
  cx: number,
  cy: number,
  size: number,
  color: number,
  filled: boolean,
  pointUp: boolean,
): void {
  const r = size / 2;
  if (kind === "triangle") {
    const tipY = pointUp ? cy - r : cy + r;
    const baseY = pointUp ? cy + r : cy - r;
    gfx.moveTo(cx, tipY);
    gfx.lineTo(cx - r, baseY);
    gfx.lineTo(cx + r, baseY);
    gfx.closePath();
  } else if (kind === "diamond") {
    gfx.moveTo(cx, cy - r);
    gfx.lineTo(cx + r, cy);
    gfx.lineTo(cx, cy + r);
    gfx.lineTo(cx - r, cy);
    gfx.closePath();
  } else {
    gfx.circle(cx, cy, r * 0.8);
  }
  if (filled) {
    gfx.fill({ color });
  } else {
    gfx.stroke({ color, width: 1.2, alpha: 0.5 });
  }
}

function drawShapeWithAlpha(
  gfx: Graphics,
  kind: ShapeKind,
  cx: number,
  cy: number,
  size: number,
  color: number,
  filled: boolean,
  pointUp: boolean,
  alpha: number,
): void {
  const r = size / 2;
  if (kind === "triangle") {
    const tipY = pointUp ? cy - r : cy + r;
    const baseY = pointUp ? cy + r : cy - r;
    gfx.moveTo(cx, tipY);
    gfx.lineTo(cx - r, baseY);
    gfx.lineTo(cx + r, baseY);
    gfx.closePath();
  } else if (kind === "diamond") {
    gfx.moveTo(cx, cy - r);
    gfx.lineTo(cx + r, cy);
    gfx.lineTo(cx, cy + r);
    gfx.lineTo(cx - r, cy);
    gfx.closePath();
  } else {
    gfx.circle(cx, cy, r * 0.8);
  }
  if (filled) {
    gfx.fill({ color, alpha });
  } else {
    gfx.stroke({ color, width: 1.2, alpha: alpha * 0.5 });
  }
}

const normalStyle = new TextStyle({ fontFamily: FONT, fontSize: 11, fontWeight: "600", fill: 0x555555, align: "center" });
const activeStyle = new TextStyle({ fontFamily: FONT, fontSize: 11, fontWeight: "bold", fill: 0xffffff, align: "center" });
const combatActiveStyle = new TextStyle({ fontFamily: FONT, fontSize: 11, fontWeight: "bold", fill: 0xffffff, align: "center" });
const enabledStyle = new TextStyle({ fontFamily: FONT, fontSize: 11, fontWeight: "600", fill: 0xaaaaaa, align: "center" });

interface PhaseCell {
  bg: Graphics;
  flashGfx: Graphics;
  hoverBg: Graphics;
  hitArea: Graphics;
  text: Text;
  icon?: Sprite;
  id: string;
  defaultLabel: string;
  subPhases?: string[];
  flashStart: number;
  /** Bottom indicator (self-turn toggle). */
  selfIndicator: Graphics;
  selfHitArea: Graphics;
  /** Top indicators (opponent-turn toggles, up to 3). */
  oppIndicators: Graphics;
  oppHitAreas: Graphics[];
}

export interface OpponentInfo {
  id: string;
  /** Display order index (0-2). Determines shape + color. */
  index: number;
}

export interface PhaseStripState {
  currentStep: string;
  isActiveTurn: boolean;
  /** ID of the player whose turn it is. */
  activePlayerId: string;
  /** The local player's ID. */
  myPlayerId: string;
  /** Self-turn enabled phases. */
  selfEnabledPhases: Set<string>;
  /** Per-opponent enabled phases, keyed by opponent id. */
  opponentEnabledPhases: Map<string, Set<string>>;
  /** Ordered opponent list (max 3). */
  opponents: OpponentInfo[];
  isInteractive: boolean;
}

export interface PhaseStripCallbacks {
  onToggleSelfPhase?: (phaseId: string) => void;
  onToggleOpponentPhase?: (opponentId: string, phaseId: string) => void;
}

export class PhaseStripLayer {
  readonly container: Container;
  private theme: PixiThemeColors;
  private callbacks: PhaseStripCallbacks = {};
  private lastState: PhaseStripState | null = null;
  private cells: PhaseCell[];
  private prevStep: string | null = null;
  private prevIsActiveTurn = false;
  /** Displayed active player — lags behind the real one so the color
   *  doesn't flip during cleanup (engine advances the active player
   *  before the new turn's first phase). */
  private displayActivePlayerId: string | null = null;
  private canvasWidth = 0;
  private canvasHeight = 0;
  private lineGfx: Graphics;

  constructor(theme: PixiThemeColors) {
    this.theme = theme;
    this.container = new Container();
    this.container.label = "phaseStrip";

    // Divider line behind the cells
    this.lineGfx = new Graphics();
    this.container.addChild(this.lineGfx);

    this.cells = [];
    for (const p of PHASES) {
      const bg = new Graphics();
      this.container.addChild(bg);
      const flashGfx = new Graphics();
      this.container.addChild(flashGfx);
      const hoverBg = new Graphics();
      hoverBg.visible = false;
      this.container.addChild(hoverBg);
      // Combat cell gets an icon; default label is empty (icon replaces it)
      let icon: Sprite | undefined;
      const isCombat = !!p.subPhases;
      if (isCombat) {
        icon = new Sprite();
        icon.width = COMBAT_ICON_SIZE;
        icon.height = COMBAT_ICON_SIZE;
        this.container.addChild(icon);
        applyIcon(icon, "cmdsword", ICON_COLORS["cmdsword"] ?? "#6b7280");
      }
      const text = new Text({ text: isCombat ? "" : p.short, style: normalStyle });
      text.anchor.set(0.5, 0.5);
      this.container.addChild(text);
      // Main cell hit area (for hover) — added first so indicators sit on top
      const hitArea = new Graphics();
      hitArea.eventMode = "static";
      hitArea.cursor = "pointer";
      hitArea.on("pointerover", () => { hoverBg.visible = true; });
      hitArea.on("pointerout", () => { hoverBg.visible = false; });
      this.container.addChild(hitArea);

      // Self indicator (bottom — my turn toggle)
      const selfIndicator = new Graphics();
      this.container.addChild(selfIndicator);
      const selfHitArea = new Graphics();
      selfHitArea.eventMode = "static";
      selfHitArea.cursor = "pointer";
      selfHitArea.on("pointerdown", () => {
        const phases = p.subPhases ?? [p.id];
        for (const ph of phases) this.callbacks.onToggleSelfPhase?.(ph);
      });
      this.container.addChild(selfHitArea);

      // Opponent indicators (top — per-opponent turn toggles)
      const oppIndicators = new Graphics();
      this.container.addChild(oppIndicators);
      const oppHitAreas: Graphics[] = [];
      for (let oi = 0; oi < 3; oi++) {
        const oha = new Graphics();
        oha.eventMode = "static";
        oha.cursor = "pointer";
        oha.visible = false;
        oha.on("pointerdown", () => {
          const oppState = this.lastState;
          if (!oppState) return;
          const opp = oppState.opponents[oi];
          if (!opp) return;
          const phases = p.subPhases ?? [p.id];
          for (const ph of phases) this.callbacks.onToggleOpponentPhase?.(opp.id, ph);
        });
        this.container.addChild(oha);
        oppHitAreas.push(oha);
      }

      const cell: PhaseCell = {
        bg, flashGfx, hoverBg, hitArea, text, icon,
        id: p.id, defaultLabel: p.short, subPhases: p.subPhases, flashStart: 0,
        selfIndicator, selfHitArea, oppIndicators, oppHitAreas,
      };
      this.cells.push(cell);
    }
  }

  setTheme(theme: PixiThemeColors): void {
    this.theme = theme;
  }

  setCallbacks(cb: PhaseStripCallbacks): void {
    this.callbacks = cb;
  }

  resize(width: number, height: number): void {
    this.canvasWidth = width;
    this.canvasHeight = height;
  }

  update(state: PhaseStripState): void {
    this.lastState = state;
    const t = this.theme;
    const y = (this.canvasHeight / 2) - CELL_H / 2;
    const centerX = this.canvasWidth / 2;

    // Find combat cell index
    const combatIdx = this.cells.findIndex((c) => !!c.subPhases);
    const leftCells = this.cells.slice(0, combatIdx);
    const rightCells = this.cells.slice(combatIdx + 1);

    // Combat cell centered
    const combatX = centerX - COMBAT_CELL_W / 2;

    // Left cells expand leftward from combat
    const cellPositions: number[] = new Array(this.cells.length);
    cellPositions[combatIdx] = combatX;
    let lx = combatX - CELL_GAP;
    for (let i = leftCells.length - 1; i >= 0; i--) {
      lx -= CELL_W;
      cellPositions[i] = lx;
      lx -= CELL_GAP;
    }
    // Right cells expand rightward from combat
    let rx = combatX + COMBAT_CELL_W + CELL_GAP;
    for (let i = 0; i < rightCells.length; i++) {
      cellPositions[combatIdx + 1 + i] = rx;
      rx += CELL_W + CELL_GAP;
    }

    // Divider line — only the edges outside all cells
    const lineY = this.canvasHeight / 2;
    const stripLeft = cellPositions[0]! - CELL_GAP;
    const stripRight = rx - CELL_GAP + CELL_GAP;
    this.lineGfx.clear();
    // Left segment
    this.lineGfx.moveTo(0, lineY);
    this.lineGfx.lineTo(stripLeft, lineY);
    this.lineGfx.stroke({ color: 0xffffff, width: 2, alpha: 0.12 });
    // Right segment
    this.lineGfx.moveTo(stripRight, lineY);
    this.lineGfx.lineTo(this.canvasWidth, lineY);
    this.lineGfx.stroke({ color: 0xffffff, width: 2, alpha: 0.12 });

    // Detect phase change for flash
    const turnJustStarted = state.isActiveTurn && !this.prevIsActiveTurn;
    this.prevIsActiveTurn = state.isActiveTurn;
    let stepChanged = false;
    if (turnJustStarted) {
      this.prevStep = state.currentStep;
    } else if (this.prevStep !== null && this.prevStep !== state.currentStep) {
      stepChanged = true;
    }
    this.prevStep = state.currentStep;

    const combatActiveColor = t.promptAction.attackAction;

    // Determine the active player's color for the bar tint
    const pc = t.playerColors;
    const selfColor = pc.self;
    const oppColors = [pc.opponent1, pc.opponent2, pc.opponent3];

    // Only update the displayed active player on non-cleanup phases
    // so the bar color doesn't flip early during cleanup.
    if (state.currentStep !== "cleanup") {
      this.displayActivePlayerId = state.activePlayerId;
    }
    const displayActive = this.displayActivePlayerId ?? state.activePlayerId;

    const isMeActive = displayActive === state.myPlayerId;
    const activeOppIdx = state.opponents.findIndex((o) => o.id === displayActive);
    const turnColor = isMeActive
      ? selfColor
      : activeOppIdx >= 0
        ? oppColors[activeOppIdx]!
        : 0x555555;

    const count = this.cells.length;
    for (let i = 0; i < count; i++) {
      const cell = this.cells[i]!;
      const isCombatCell = !!cell.subPhases;
      const cellW = isCombatCell ? COMBAT_CELL_W : CELL_W;
      const cx = cellPositions[i]!;

      const combatSubActive = isCombatCell && cell.subPhases!.includes(state.currentStep);
      const isCurrentPhase = isCombatCell ? combatSubActive : state.currentStep === cell.id;
      const isActive = isCurrentPhase; // highlight current phase regardless of whose turn
      const isEnabled = isCombatCell
        ? cell.subPhases!.some((s) => state.selfEnabledPhases.has(s))
        : state.selfEnabledPhases.has(cell.id);

      // Combat label: show sub-phase when active, icon-only otherwise
      if (isCombatCell) {
        cell.text.text = combatSubActive ? (COMBAT_LABELS[state.currentStep] ?? "") : "";
      }

      // Combat icon position + tint
      if (cell.icon) {
        const iconTint = isActive ? "#ffffff" : (ICON_COLORS["cmdsword"] ?? "#6b7280");
        applyIcon(cell.icon, "cmdsword", iconTint);
        cell.icon.width = COMBAT_ICON_SIZE;
        cell.icon.height = COMBAT_ICON_SIZE;
        if (combatSubActive) {
          // Icon left, label right
          cell.icon.x = cx + (cellW - COMBAT_ICON_SIZE - cell.text.width - 3) / 2;
          cell.icon.y = y + (CELL_H - COMBAT_ICON_SIZE) / 2;
          cell.text.x = cell.icon.x + COMBAT_ICON_SIZE + 3 + cell.text.width / 2;
        } else {
          // Icon centered
          cell.icon.x = cx + (cellW - COMBAT_ICON_SIZE) / 2;
          cell.icon.y = y + (CELL_H - COMBAT_ICON_SIZE) / 2;
        }
      }

      // Trigger flash
      if (stepChanged && isActive) {
        cell.flashStart = performance.now();
      }

      // Hit area
      cell.hitArea.clear();
      cell.hitArea.rect(cx, y, cellW, CELL_H);
      cell.hitArea.fill({ color: 0x000000, alpha: 0.001 });
      cell.hitArea.eventMode = state.isInteractive ? "static" : "none";
      cell.hitArea.cursor = state.isInteractive ? "pointer" : "default";

      // Background — opaque base, tinted with turn color, current phase brighter
      const phaseColor = isCombatCell && combatSubActive ? combatActiveColor : turnColor;
      cell.bg.clear();
      cell.bg.roundRect(cx, y, cellW, CELL_H, CELL_R);
      cell.bg.fill({ color: 0x0c0c0c });
      // All cells get a subtle tint of the turn color
      cell.bg.roundRect(cx, y, cellW, CELL_H, CELL_R);
      cell.bg.fill({ color: turnColor, alpha: 0.12 });
      // Current phase gets a stronger highlight
      if (isActive) {
        cell.bg.roundRect(cx, y, cellW, CELL_H, CELL_R);
        cell.bg.fill({ color: phaseColor, alpha: 0.7 });
      }

      // Hover overlay
      cell.hoverBg.clear();
      cell.hoverBg.roundRect(cx, y, cellW, CELL_H, CELL_R);
      cell.hoverBg.fill({ color: 0x2a2a2a, alpha: 0.5 });

      // Text position (non-combat cells; combat text is positioned with the icon above)
      cell.text.style = isActive
        ? (isCombatCell ? combatActiveStyle : activeStyle)
        : isEnabled ? enabledStyle : normalStyle;
      if (!isCombatCell) {
        cell.text.x = cx + cellW / 2;
        cell.text.y = y + CELL_H / 2;
      } else if (!combatSubActive) {
        // No text when showing icon only — position offscreen
        cell.text.x = -999;
      } else {
        cell.text.y = y + CELL_H / 2;
      }

      // ── Self indicator (bottom: triangle pointing down) ──
      const phaseIds = cell.subPhases ?? [cell.id];
      const selfEnabled = phaseIds.some((ph) => state.selfEnabledPhases.has(ph));
      const selfIndCx = cx + cellW / 2;
      const selfIndCy = y + CELL_H + INDICATOR_MARGIN + INDICATOR_SIZE / 2;

      cell.selfIndicator.clear();
      drawShape(cell.selfIndicator, "triangle", selfIndCx, selfIndCy, INDICATOR_SIZE, selfColor, selfEnabled, false);
      cell.selfIndicator.alpha = isMeActive ? 1 : 0.3;

      cell.selfHitArea.clear();
      cell.selfHitArea.rect(selfIndCx - INDICATOR_SIZE, selfIndCy - INDICATOR_SIZE, INDICATOR_SIZE * 2, INDICATOR_SIZE * 2);
      cell.selfHitArea.fill({ color: 0x000000, alpha: 0.001 });

      // ── Opponent indicators (top: shapes pointing up) ──
      const oppCount = state.opponents.length;
      cell.oppIndicators.clear();
      const oppRowCx = cx + cellW / 2;
      const oppRowCy = y - INDICATOR_MARGIN - INDICATOR_SIZE / 2;
      const oppTotalW = oppCount * INDICATOR_SIZE + Math.max(0, oppCount - 1) * INDICATOR_GAP;
      const oppStartX = oppRowCx - oppTotalW / 2 + INDICATOR_SIZE / 2;

      for (let oi = 0; oi < 3; oi++) {
        const oha = cell.oppHitAreas[oi]!;
        if (oi >= oppCount) {
          oha.visible = false;
          continue;
        }
        oha.visible = true;
        const opp = state.opponents[oi]!;
        const isThisOppTurn = displayActive === opp.id;
        const oppStops = state.opponentEnabledPhases.get(opp.id);
        const oppEnabled = phaseIds.some((ph) => oppStops?.has(ph));
        const shape = OPPONENT_SHAPES[oi]!;
        const color = oppColors[oi]!;
        const shapeX = oppStartX + oi * (INDICATOR_SIZE + INDICATOR_GAP);
        const dimAlpha = isThisOppTurn ? 1 : 0.35;

        drawShapeWithAlpha(cell.oppIndicators, shape, shapeX, oppRowCy, INDICATOR_SIZE, color, oppEnabled, true, dimAlpha);

        oha.clear();
        oha.rect(shapeX - INDICATOR_SIZE, oppRowCy - INDICATOR_SIZE, INDICATOR_SIZE * 2, INDICATOR_SIZE * 2);
        oha.fill({ color: 0x000000, alpha: 0.001 });
      }

      // Store geometry for flash tick
      (cell as any)._fx = cx;
      (cell as any)._fy = y;
      (cell as any)._fw = cellW;
      (cell as any)._fc = isCombatCell && combatSubActive ? combatActiveColor : turnColor;
    }
  }

  tick(): void {
    const now = performance.now();
    for (const cell of this.cells) {
      cell.flashGfx.clear();
      if (cell.flashStart === 0) continue;
      const elapsed = now - cell.flashStart;
      if (elapsed >= FLASH_DURATION_MS) { cell.flashStart = 0; continue; }

      const p = elapsed / FLASH_DURATION_MS;
      const e = easeOut(p);
      const fade = 1 - e;
      const cx = (cell as any)._fx as number;
      const cy = (cell as any)._fy as number;
      const color = (cell as any)._fc as number;
      if (cx === undefined) continue;

      const cw = ((cell as any)._fw as number) ?? CELL_W;
      const expand = fade * FLASH_MAX_EXPAND;
      cell.flashGfx.roundRect(cx - expand, cy - expand, cw + expand * 2, CELL_H + expand * 2, CELL_R + expand * 0.5);
      cell.flashGfx.stroke({ color, width: 1.5 + fade * 2, alpha: fade * 0.85, alignment: 0.5 });
      cell.flashGfx.roundRect(cx, cy, cw, CELL_H, CELL_R);
      cell.flashGfx.fill({ color, alpha: fade * fade * 0.25 });
    }
  }

  destroy(): void {
    try { this.container.destroy(); } catch { /* pixi teardown */ }
  }
}
