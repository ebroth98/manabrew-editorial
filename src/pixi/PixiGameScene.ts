import {
  Application,
  Container,
  Graphics,
  Text,
  FederatedPointerEvent,
  Sprite,
  type Texture,
} from "pixi.js";
import type { Card } from "@/types/openmagic";
import type {
  GameCanvasCallbacks,
  BattlefieldState,
  HandState,
  HandSize,
  ScreenPos,
  ScreenBounds,
} from "./types";
import type { PixiThemeColors } from "./themeAdapter";
import { CardSprite } from "./CardSprite";
import {
  CARD_W,
  CARD_H,
  ZONE_COLUMN_RESERVED_PX,
} from "@/components/game/game.constants";
import { MarqueeHandler } from "./MarqueeHandler";
import { DragHandler } from "./DragHandler";
import { computeHandLayout, SIZE_PARAMS } from "./HandLayout";
import { ArrowLayer, type ArrowDef } from "./ArrowLayer";
import { HAND_CARD_BASES } from "@/components/game/game.styles";
import {
  extractManaLetters,
  getExpandedManaAbilities,
} from "@/components/game/manaUtils";
import {
  getManaSymbolTextureSync,
  loadManaSymbolTexture,
  prewarmManaSymbols,
} from "./manaSymbolCache";
import { manaColorFor } from "./manaColors";
import {
  ATTACH_OFFSET_Y,
  BATTLEFIELD_HOVER_HOLD_MS,
  BATTLEFIELD_LERP,
  BG_ALPHA_DROP,
  BG_ALPHA_IDLE,
  BG_COLOR,
  BLACK,
  CARD_RADIUS,
  DROP_STROKE_ALPHA,
  DROP_TINT_ALPHA,
  FALLBACK_GRAY,
  FALLBACK_ORANGE,
  GAP,
  GHOST_FILL_ALPHA,
  GHOST_STROKE_ALPHA,
  HAND_HOVER_HOLD_MS,
  HAND_LERP,
  HAND_MAX_SCALE,
  HAND_MIN_SCALE,
  HAND_REF_WIDTH,
  HOVER_SCALE,
  HOVER_SCALE_LERP,
  ICON_BG_ALPHA,
  ICON_HOVER_SCALE,
  MANA_BUTTON_ALPHA,
  MANA_BUTTON_HOVER_ALPHA,
  MANA_BUTTON_STROKE_ALPHA,
  MANA_BUTTON_STROKE_HOVER_ALPHA,
  MAX_GRID_SLOTS,
  MAX_LAND_SLOTS,
  OVERLAY_FADE_LERP,
  OVERLAY_LABEL_SELECT,
  OVERLAY_LABEL_TAP,
  OVERLAY_LABEL_UNTAP,
  SYMBOL_TAP,
  SYMBOL_UNTAP,
  PLAYABLE_HIGHLIGHT_ALPHA,
  PLAYABLE_RING_ALPHA,
  ACTION_BUTTON_ALPHA,
  ACTION_BUTTON_HOVER_ALPHA,
  SELECT_BUTTON_ALPHA,
  SELECT_BUTTON_HOVER_ALPHA,
  SNAP_ALPHA,
  SNAP_HAND_SCALE,
  SNAP_PX,
  SNAP_ROT,
  SNAP_SCALE,
  TABLE_RADIUS,
  WHITE,
  Z_HAND_CONTAINER,
  Z_HAND_HOVERED,
  Z_OVERLAY_OFFSET,
  Z_PLACEMENT_GHOST,
  Z_PLACEMENT_GHOST_TEXT,
  Z_SELECTION_BADGE,
} from "./constants";
import {
  EMPTY_LABEL_STYLE,
  GHOST_LABEL_STYLE,
  OVERLAY_LABEL_STYLE,
  SELECTION_BADGE_STYLE,
} from "./textStyles";

// ───── Shared types ─────
type Point = ScreenPos;

interface SpriteEntry {
  sprite: CardSprite;
  targetX: number;
  targetY: number;
  targetZIndex: number;
  overlay: Container | null;
}

interface HandTarget {
  x: number;
  y: number;
  rot: number;
  scale: number;
  zIndex: number;
}

interface ActionKind {
  isTappable: boolean;
  isUntappable: boolean;
  isChoosable: boolean;
}

// ───── Pure helpers ─────
const lerp = (
  current: number,
  target: number,
  speed: number,
  snap: number,
): number => {
  const d = target - current;
  return Math.abs(d) > snap ? current + d * speed : target;
};

export interface BlockingRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

const rectsOverlap = (a: Point, b: Point): boolean =>
  a.x < b.x + CARD_W + GAP / 2 &&
  a.x + CARD_W + GAP / 2 > b.x &&
  a.y < b.y + CARD_H + GAP / 2 &&
  a.y + CARD_H + GAP / 2 > b.y;

const cardIntersectsRect = (pos: Point, rect: BlockingRect): boolean =>
  pos.x < rect.x + rect.width &&
  pos.x + CARD_W > rect.x &&
  pos.y < rect.y + rect.height &&
  pos.y + CARD_H > rect.y;

const findFreeSlot = (
  occupied: Point[],
  blockers: BlockingRect[],
  xMin: number,
  cols: number,
  yForSlot: (slot: number) => number,
  maxSlots: number,
): Point => {
  for (let slot = 0; slot < maxSlots; slot++) {
    const pos: Point = {
      x: xMin + (slot % cols) * (CARD_W + GAP) + GAP,
      y: yForSlot(slot),
    };
    if (occupied.some((o) => rectsOverlap(pos, o))) continue;
    if (blockers.some((b) => cardIntersectsRect(pos, b))) continue;
    return pos;
  }
  return { x: xMin + GAP, y: yForSlot(0) };
};

export class PixiGameScene {
  app: Application;
  private root: Container;
  private myBattlefieldContainer: Container;
  private backgroundGfx: Graphics;
  private entries = new Map<string, SpriteEntry>();
  private callbacks: GameCanvasCallbacks;
  private theme: PixiThemeColors | null = null;
  private bottomReserved = 0;
  private leftReserved = ZONE_COLUMN_RESERVED_PX;
  private hoveredCardId: string | null = null;
  private battlefieldHoverClearTimer: number | null = null;
  /** Extra blocker rects (in canvas-local coords) — e.g. the PASS / phase-pass
   * button cluster at the bottom-right so lands aren't placed under it. */
  private externalBlockers: BlockingRect[] = [];
  /** Keep-out size anchored to the bottom-right of the canvas (recomputed
   * from current renderer dimensions so the rect stays valid after resize). */
  private bottomRightReserved: { width: number; height: number } | null = null;
  /**
   * Set in `destroy()` so any late-firing effects (React unmount races) that
   * still hold a reference to this instance short-circuit instead of touching
   * a partially-torn-down Pixi display tree.
   */
  private destroyed = false;
  private selectedCardIds = new Set<string>();
  private marquee: MarqueeHandler;
  private dragHandler: DragHandler;
  private customPositions = new Map<string, Point>();
  private lastState: BattlefieldState | null = null;
  private emptyText: Text;
  private selectionBadge: Text;

  private handContainer: Container;
  private handSprites = new Map<string, CardSprite>();
  private handTargets = new Map<string, HandTarget>();
  private hoveredHandIndex: number | null = null;
  private handHoverHoldTimer: number | null = null;
  private pendingHandHoverLeaveIndex: number | null = null;
  private lastHandState: HandState | null = null;
  private handSize: HandSize = "medium";
  private vScale = 1;
  private arrowLayer: ArrowLayer;
  private placementGhostGfx: Graphics | null = null;
  private placementGhostText: Text | null = null;

  constructor(app: Application, callbacks: GameCanvasCallbacks) {
    this.app = app;
    this.callbacks = callbacks;

    this.root = new Container();
    app.stage.addChild(this.root);

    this.backgroundGfx = new Graphics();
    this.root.addChild(this.backgroundGfx);

    this.myBattlefieldContainer = new Container();
    this.myBattlefieldContainer.label = "myBattlefield";
    this.myBattlefieldContainer.sortableChildren = true;
    this.root.addChild(this.myBattlefieldContainer);

    this.emptyText = new Text({
      text: "No permanents",
      style: EMPTY_LABEL_STYLE,
    });
    this.emptyText.anchor.set(0.5);
    this.emptyText.visible = false;
    this.root.addChild(this.emptyText);

    this.marquee = new MarqueeHandler();
    this.root.addChild(this.marquee.graphics);

    this.dragHandler = new DragHandler();

    this.selectionBadge = new Text({ text: "", style: SELECTION_BADGE_STYLE });
    this.selectionBadge.visible = false;
    this.selectionBadge.zIndex = Z_SELECTION_BADGE;
    this.root.addChild(this.selectionBadge);

    this.handContainer = new Container();
    this.handContainer.label = "hand";
    this.handContainer.sortableChildren = true;
    this.handContainer.zIndex = Z_HAND_CONTAINER;
    this.root.addChild(this.handContainer);

    this.arrowLayer = new ArrowLayer();
    this.root.addChild(this.arrowLayer.graphics);

    this.backgroundGfx.eventMode = "static";
    this.backgroundGfx.on("pointerdown", (e: FederatedPointerEvent) =>
      this.onBackgroundDown(e),
    );
    app.stage.on("pointermove", (e: FederatedPointerEvent) =>
      this.onGlobalMove(e),
    );
    app.stage.on("pointerup", () => this.onGlobalUp());
    app.stage.on("pointerupoutside", () => this.onGlobalUp());
    app.stage.eventMode = "static";

    app.ticker.add(this.tick, this);
    prewarmManaSymbols();
  }

  // ═══════════════════════════════════════════════════════════════
  // Public API
  // ═══════════════════════════════════════════════════════════════

  /** True once `destroy()` has run. Effects that fire late must bail. */
  get isDestroyed(): boolean {
    return this.destroyed;
  }

  setTheme(theme: PixiThemeColors): void {
    if (this.destroyed) return;
    this.theme = theme;
    this.arrowLayer.setTheme(theme);
    this.drawTableBackground();
  }

  setReserved(bottom: number, left: number): void {
    if (this.destroyed) return;
    this.bottomReserved = bottom;
    this.leftReserved = left;
    this.dragHandler.setReserved(left, 0, bottom);
  }

  /**
   * Register additional canvas regions that must stay clear of auto-placed
   * and dragged cards — typically UI overlays like the PASS / phase-pass
   * button cluster at the bottom-right.
   */
  setExternalBlockers(rects: BlockingRect[]): void {
    if (this.destroyed) return;
    this.externalBlockers = rects;
    this.syncDragBlockers();
    if (this.lastState) this.updateBattlefield(this.lastState);
  }

  /**
   * Reserve a fixed-size rectangle anchored to the canvas bottom-right for
   * a UI overlay (e.g. the PASS / phase-pass buttons). Pass `null` to clear.
   */
  setBottomRightReserved(size: { width: number; height: number } | null): void {
    if (this.destroyed) return;
    this.bottomRightReserved = size;
    this.syncDragBlockers();
    if (this.lastState) this.updateBattlefield(this.lastState);
  }

  private syncDragBlockers(): void {
    this.dragHandler.setExtraBlockers(this.collectOverlayBlockers());
  }

  /** External + bottom-right reserved rects resolved against current size. */
  private collectOverlayBlockers(): BlockingRect[] {
    const rects = [...this.externalBlockers];
    if (this.bottomRightReserved) {
      const { width, height } = this.app.renderer;
      const { width: w, height: h } = this.bottomRightReserved;
      rects.push({ x: width - w, y: height - h, width: w, height: h });
    }
    return rects;
  }

  setHandPreferences(size: HandSize, scale: number): void {
    if (this.destroyed) return;
    this.handSize = size;
    this.vScale = scale;
  }

  resize(width: number, height: number): void {
    if (this.destroyed) return;
    this.app.renderer.resize(width, height);
    this.drawTableBackground();
    this.emptyText.x = width / 2;
    this.emptyText.y = height / 2;
    this.dragHandler.setContainerSize(width, height);
    // Bottom-right reserved rect is anchored to canvas size — re-resolve
    // so the keep-out follows the resize.
    this.syncDragBlockers();
    if (this.lastHandState) this.updateHand(this.lastHandState);
  }

  updateArrows(arrows: ArrowDef[]): void {
    if (this.destroyed) return;
    this.arrowLayer.update(arrows);
  }

  setDropActive(active: boolean): void {
    if (this.destroyed || !this.theme) return;
    this.drawDropTargetBackground(active);
  }

  showPlacementGhost(cardName: string | null): void {
    if (this.destroyed) return;
    this.drawPlacementGhost(cardName);
  }

  updateBattlefield(state: BattlefieldState): void {
    if (this.destroyed || !state || !Array.isArray(state.cards)) return;
    this.lastState = state;
    const cardMap = new Map<string, Card>(state.cards.map((c) => [c.id, c]));
    const attachedIds = new Set<string>();
    for (const c of state.cards) {
      if (c.attachedTo && cardMap.has(c.attachedTo)) attachedIds.add(c.id);
    }
    const topLevelCards = state.cards.filter((c) => !attachedIds.has(c.id));
    const currentIds = new Set(state.cards.map((c) => c.id));

    this.pruneRemovedBattlefieldEntries(currentIds);
    const positions = this.computeBattlefieldGrid(topLevelCards);

    for (const card of topLevelCards) {
      const gridPos = positions.get(card.id) ?? { x: 0, y: 0 };
      // customPositions are stored as sprite *centers* (matching
      // sprite.x/y and dragHandler coords), but gridPos is top-left and
      // placeBattlefieldCard expects a top-left that it then offsets by
      // CARD_W/2 + CARD_H/2 to get the center. Convert the center back to
      // top-left so both paths line up — without this, a dragged card
      // jumps by half a card on the next battlefield state update.
      const customCenter = this.customPositions.get(card.id);
      const pos = customCenter
        ? { x: customCenter.x - CARD_W / 2, y: customCenter.y - CARD_H / 2 }
        : gridPos;
      const attachments = (card.attachmentIds ?? [])
        .map((id) => cardMap.get(id))
        .filter((c): c is Card => c !== undefined);
      const totalOffset = attachments.length * ATTACH_OFFSET_Y;

      for (let i = 0; i < attachments.length; i++) {
        const att = attachments[i]!;
        this.placeBattlefieldCard(
          att,
          pos.x + CARD_W / 2,
          pos.y +
            totalOffset -
            (attachments.length - i) * ATTACH_OFFSET_Y +
            CARD_H / 2,
          i + 1,
          state,
        );
      }

      this.placeBattlefieldCard(
        card,
        pos.x + CARD_W / 2,
        pos.y + totalOffset + CARD_H / 2,
        attachments.length + 1,
        state,
      );
    }

    this.emptyText.visible = state.cards.length === 0;
  }

  updateHand(state: HandState): void {
    if (this.destroyed || !state || !Array.isArray(state.cards)) return;
    this.lastHandState = state;
    this.pruneRemovedHandSprites(new Set(state.cards.map((c) => c.id)));
    this.dragHandler.setHandExclusion(this.collectHandBlockers()[0] ?? null);

    const dims = this.computeHandDimensions();
    const layout = computeHandLayout(
      state.cards.length,
      dims.cardW,
      dims.cardH,
      dims.maxSpread,
      dims.minSpread,
      dims.spreadWidth,
      this.hoveredHandIndex,
      dims.hoverLift,
      dims.neighborPush,
    );

    const centerX = this.app.renderer.width / 2;
    const bottomY = this.app.renderer.height;

    for (let i = 0; i < state.cards.length; i++) {
      const card = state.cards[i]!;
      const l = layout[i]!;
      const isHovered = this.hoveredHandIndex === i;

      let sprite = this.handSprites.get(card.id);
      if (!sprite) {
        sprite = this.createHandSprite(card);
        sprite.x = centerX + l.x;
        sprite.y = bottomY + l.y - l.scaleH / 2;
        sprite.scale.set(l.scaleW / CARD_W, l.scaleH / CARD_H);
      } else {
        // updateCardContent, not updateCard: the hand's animation tick owns
        // rotation (arc-fan angle) and alpha (dragging/casting); touching
        // them here would snap-jump back to defaults and re-lerp every tick.
        sprite.updateCardContent(card);
      }

      const isHidden =
        card.id === state.draggingCardId || card.id === state.castingCardId;
      sprite.alpha = isHidden ? 0 : 1;
      sprite.cursor = card.isPlayable ? "grab" : "default";

      const scaleX = l.scaleW / CARD_W;
      const scaleY = l.scaleH / CARD_H;
      this.handTargets.set(card.id, {
        x: centerX + l.x,
        y: bottomY + l.y - l.scaleH / 2,
        rot: (l.rotation * Math.PI) / 180,
        scale: scaleX,
        zIndex: isHovered ? Z_HAND_HOVERED : i + 1,
      });
      sprite.scale.set(scaleX, scaleY);

      this.applyHandCardHighlight(sprite, card, isHovered);
    }
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.cancelHandHoverHoldTimer();
    this.cancelBattlefieldHoverClear();
    this.app.ticker.remove(this.tick, this);
    this.marquee.destroy();
    this.dragHandler.destroy();
    this.arrowLayer.destroy();
    for (const sprite of this.handSprites.values())
      sprite.destroy({ children: true });
    this.handSprites.clear();
    for (const entry of this.entries.values()) {
      entry.sprite.destroy({ children: true });
      entry.overlay?.destroy({ children: true });
    }
    this.entries.clear();
    this.root.destroy({ children: true });
  }

  // ═══════════════════════════════════════════════════════════════
  // Background + drop target
  // ═══════════════════════════════════════════════════════════════

  private drawTableBackground(): void {
    const { width, height } = this.app.renderer;
    this.backgroundGfx.clear();
    this.backgroundGfx.roundRect(0, 0, width, height, TABLE_RADIUS);
    this.backgroundGfx.fill({ color: BG_COLOR, alpha: BG_ALPHA_IDLE });
  }

  private drawDropTargetBackground(active: boolean): void {
    const { width, height } = this.app.renderer;
    this.backgroundGfx.clear();
    this.backgroundGfx.roundRect(0, 0, width, height, TABLE_RADIUS);
    this.backgroundGfx.fill({ color: BG_COLOR, alpha: BG_ALPHA_DROP });
    if (!active || !this.theme) return;
    const tint = this.theme.activeAction.active;
    this.backgroundGfx.roundRect(2, 2, width - 4, height - 4, TABLE_RADIUS);
    this.backgroundGfx.stroke({
      color: tint,
      width: 2,
      alpha: DROP_STROKE_ALPHA,
    });
    this.backgroundGfx.roundRect(0, 0, width, height, TABLE_RADIUS);
    this.backgroundGfx.fill({ color: tint, alpha: DROP_TINT_ALPHA });
  }

  // ═══════════════════════════════════════════════════════════════
  // Placement ghost
  // ═══════════════════════════════════════════════════════════════

  private drawPlacementGhost(cardName: string | null): void {
    const { gfx, text } = this.ensurePlacementGhostLayers();
    if (!cardName) {
      gfx.visible = false;
      text.visible = false;
      return;
    }

    const slot = this.findFirstFreeBattlefieldSlot();
    const cx = slot.x + CARD_W / 2;
    const cy = slot.y + CARD_H / 2;
    const color = this.theme?.activeAction.active ?? FALLBACK_ORANGE;

    gfx.clear();
    gfx.roundRect(
      cx - CARD_W / 2,
      cy - CARD_H / 2,
      CARD_W,
      CARD_H,
      CARD_RADIUS,
    );
    gfx.stroke({ color, width: 2, alpha: GHOST_STROKE_ALPHA });
    gfx.fill({ color, alpha: GHOST_FILL_ALPHA });
    gfx.visible = true;

    text.text = cardName;
    text.x = cx;
    text.y = cy;
    text.visible = true;
  }

  private ensurePlacementGhostLayers(): { gfx: Graphics; text: Text } {
    if (this.placementGhostGfx && this.placementGhostText) {
      return { gfx: this.placementGhostGfx, text: this.placementGhostText };
    }
    const gfx = new Graphics();
    gfx.zIndex = Z_PLACEMENT_GHOST;
    this.myBattlefieldContainer.addChild(gfx);

    const text = new Text({ text: "", style: GHOST_LABEL_STYLE });
    text.anchor.set(0.5);
    text.zIndex = Z_PLACEMENT_GHOST_TEXT;
    this.myBattlefieldContainer.addChild(text);

    this.placementGhostGfx = gfx;
    this.placementGhostText = text;
    return { gfx, text };
  }

  // ═══════════════════════════════════════════════════════════════
  // Battlefield layout
  // ═══════════════════════════════════════════════════════════════

  private computeBattlefieldGrid(cards: Card[]): Map<string, Point> {
    const positions = new Map<string, Point>();
    const { width, height } = this.app.renderer;
    const xMin = Math.max(0, this.leftReserved);
    const usableW = width - xMin;
    const cols = Math.max(1, Math.floor((usableW + GAP) / (CARD_W + GAP)));
    // Land baseline matches React: sits above the `bottomReserved` strip so
    // newly-played lands land in the same row the React renderer uses.
    // Drag and the drag-target corners use the hand rect for a more forgiving
    // clamp, but auto-placement should stay visually identical to React.
    const landRowBaseY = Math.max(0, height - CARD_H - this.bottomReserved - GAP);
    const occupied: Point[] = [];
    const blockers = this.collectOverlayBlockers();

    const landY = (slot: number): number =>
      landRowBaseY - Math.floor(slot / cols) * (CARD_H + GAP);
    const nonLandY = (slot: number): number =>
      Math.floor(slot / cols) * (CARD_H + GAP) + GAP;

    for (const c of cards) {
      const isLand = c.types.includes("Land");
      const yFn = isLand ? landY : nonLandY;
      const max = isLand ? MAX_LAND_SLOTS : MAX_GRID_SLOTS;
      const pos = findFreeSlot(occupied, blockers, xMin, cols, yFn, max);
      positions.set(c.id, pos);
      occupied.push(pos);
    }

    return positions;
  }

  private findFirstFreeBattlefieldSlot(): Point {
    const { width } = this.app.renderer;
    const xMin = Math.max(0, this.leftReserved);
    const usableW = width - xMin;
    const cols = Math.max(1, Math.floor((usableW + GAP) / (CARD_W + GAP)));
    const occupied: Point[] = [...this.entries.values()].map((e) => ({
      x: e.targetX - CARD_W / 2,
      y: e.targetY - CARD_H / 2,
    }));
    return findFreeSlot(
      occupied,
      this.collectOverlayBlockers(),
      xMin,
      cols,
      (slot) => Math.floor(slot / cols) * (CARD_H + GAP) + GAP,
      MAX_GRID_SLOTS,
    );
  }

  /**
   * Approximate bounding rectangle of the hand fan in canvas coordinates.
   * The auto-placement grid treats this as a blocker so lands can fill the
   * (often generous) space to the left and right of the hand.
   */
  private collectHandBlockers(): BlockingRect[] {
    const count = this.lastHandState?.cards.length ?? 0;
    if (count === 0) return [];

    const dims = this.computeHandDimensions();
    const spread =
      count <= 1
        ? 0
        : Math.max(
            dims.minSpread,
            Math.min(
              dims.maxSpread,
              Math.floor((dims.spreadWidth - dims.cardW) / (count - 1)),
            ),
          );
    const totalSpread = count <= 1 ? 0 : (count - 1) * spread;
    const handW = totalSpread + dims.cardW;
    const handH = dims.cardH;
    const { width, height } = this.app.renderer;

    return [
      {
        x: width / 2 - handW / 2 - GAP,
        y: height - handH - GAP,
        width: handW + GAP * 2,
        height: handH + GAP,
      },
    ];
  }


  // ═══════════════════════════════════════════════════════════════
  // Battlefield entries
  // ═══════════════════════════════════════════════════════════════

  private pruneRemovedBattlefieldEntries(currentIds: Set<string>): void {
    for (const [id, entry] of this.entries) {
      if (currentIds.has(id)) continue;
      this.myBattlefieldContainer.removeChild(entry.sprite);
      if (entry.overlay) this.myBattlefieldContainer.removeChild(entry.overlay);
      entry.sprite.destroy({ children: true });
      entry.overlay?.destroy({ children: true });
      this.entries.delete(id);
    }
  }

  private placeBattlefieldCard(
    card: Card,
    centerX: number,
    centerY: number,
    zIndex: number,
    state: BattlefieldState,
  ): void {
    this.ensureBattlefieldEntry(card);
    const entry = this.entries.get(card.id)!;
    entry.targetX = centerX;
    entry.targetY = centerY;
    entry.targetZIndex = zIndex;
    entry.sprite.updateCard(card);
    this.applyBattlefieldRing(entry.sprite, state);
    this.rebuildBattlefieldOverlay(entry, state);
  }

  private ensureBattlefieldEntry(card: Card): void {
    if (this.entries.has(card.id)) return;
    const sprite = new CardSprite(card);
    this.wireBattlefieldCardEvents(sprite);
    this.myBattlefieldContainer.addChild(sprite);
    this.entries.set(card.id, {
      sprite,
      targetX: 0,
      targetY: 0,
      targetZIndex: 1,
      overlay: null,
    });
  }

  private wireBattlefieldCardEvents(sprite: CardSprite): void {
    sprite.on("pointerdown", (e: FederatedPointerEvent) => {
      e.stopPropagation();
      this.onBattlefieldCardDown(sprite, e);
    });

    sprite.on("pointertap", () => {
      if (this.dragHandler.justDraggedCardIds.has(sprite.card.id)) return;
      this.onBattlefieldCardTap(sprite.card);
    });

    sprite.on("pointerenter", () => this.setBattlefieldCardHovered(sprite));
    sprite.on("pointerleave", () => this.scheduleBattlefieldHoverClear(sprite.card.id));
  }

  private setBattlefieldCardHovered(sprite: CardSprite): void {
    this.cancelBattlefieldHoverClear();
    const wasHovered = this.hoveredCardId === sprite.card.id;
    this.hoveredCardId = sprite.card.id;
    if (wasHovered) return;
    const bounds = sprite.getBounds();
    const canvasRect = this.app.canvas.getBoundingClientRect();
    this.callbacks.onHoverCard?.(
      sprite.card,
      {
        x: bounds.x + canvasRect.left,
        y: bounds.y + canvasRect.top,
        width: bounds.width,
        height: bounds.height,
      },
      { useAnchor: true },
    );
  }

  private scheduleBattlefieldHoverClear(cardId: string): void {
    if (this.hoveredCardId !== cardId) return;
    this.cancelBattlefieldHoverClear();
    this.battlefieldHoverClearTimer = window.setTimeout(() => {
      this.battlefieldHoverClearTimer = null;
      if (this.destroyed) return;
      if (this.hoveredCardId !== cardId) return;
      this.hoveredCardId = null;
      this.callbacks.onHoverCard?.(null);
    }, BATTLEFIELD_HOVER_HOLD_MS);
  }

  private cancelBattlefieldHoverClear(): void {
    if (this.battlefieldHoverClearTimer !== null) {
      window.clearTimeout(this.battlefieldHoverClearTimer);
      this.battlefieldHoverClearTimer = null;
    }
  }

  private onBattlefieldCardTap(card: Card): void {
    const state = this.lastState;
    if (!state) {
      this.callbacks.onClickCard?.(card);
      return;
    }

    const kind: ActionKind = {
      isTappable: state.tappableLandIds?.includes(card.id) ?? false,
      isUntappable: state.untappableLandIds?.includes(card.id) ?? false,
      isChoosable: !!card.isChoosable,
    };

    if (kind.isTappable) {
      const expandedMana = state.manaAbilityOptions
        ? getExpandedManaAbilities(card.id, state.manaAbilityOptions)
        : [];
      if (expandedMana.length > 1) {
        this.callbacks.onClickCard?.(card);
        return;
      }
      this.dispatchBattlefieldAction(card, state, kind);
      return;
    }

    if (kind.isUntappable) {
      this.dispatchBattlefieldAction(card, state, kind);
      return;
    }

    this.callbacks.onClickCard?.(card);
  }

  // ═══════════════════════════════════════════════════════════════
  // Battlefield rings + overlays
  // ═══════════════════════════════════════════════════════════════

  private applyBattlefieldRing(
    sprite: CardSprite,
    state: BattlefieldState,
  ): void {
    if (!this.theme) {
      sprite.setRing(null);
      return;
    }

    const card = sprite.card;
    if (state.attackingCardIds?.includes(card.id)) {
      sprite.setRing(this.theme.promptAction.attackAction);
    } else if (state.pendingCardIds?.includes(card.id)) {
      sprite.setRing(this.theme.promptAction.passAction);
    } else if (state.tappableLandIds?.includes(card.id)) {
      sprite.setRing(this.theme.cardRing);
    } else if (state.untappableLandIds?.includes(card.id)) {
      sprite.setRing(this.theme.promptAction.cancel);
    } else if (card.isChoosable) {
      sprite.setRing(
        state.hostileTargeting
          ? this.theme.arrow.hostileTarget.color
          : this.theme.cardRing,
      );
    } else {
      sprite.setRing(null);
    }
  }

  private rebuildBattlefieldOverlay(
    entry: SpriteEntry,
    state: BattlefieldState,
  ): void {
    const card = entry.sprite.card;
    const kind: ActionKind = {
      isTappable: state.tappableLandIds?.includes(card.id) ?? false,
      isUntappable: state.untappableLandIds?.includes(card.id) ?? false,
      isChoosable: !!(card.isChoosable && this.callbacks.onClickCard),
    };

    if (!kind.isTappable && !kind.isUntappable && !kind.isChoosable) {
      if (entry.overlay) entry.overlay.visible = false;
      return;
    }

    const overlay = this.ensureOverlayContainer(entry);
    overlay.removeChildren().forEach((c) => c.destroy({ children: true }));

    const expandedMana =
      kind.isTappable && state.manaAbilityOptions
        ? getExpandedManaAbilities(card.id, state.manaAbilityOptions)
        : [];

    if (kind.isTappable && expandedMana.length > 1) {
      this.drawManaAbilityGrid(overlay, card, expandedMana);
    } else {
      this.drawSingleActionButton(overlay, card, state, kind);
    }

    overlay.visible = true;
  }

  private ensureOverlayContainer(entry: SpriteEntry): Container {
    if (entry.overlay) return entry.overlay;
    const overlay = new Container();
    // "passive" — the overlay container itself isn't hit-tested, but child
    // buttons with eventMode "static" can receive pointer events. "none"
    // would disable hit testing for the entire subtree.
    overlay.eventMode = "passive";
    overlay.alpha = 0;
    overlay.pivot.set(CARD_W / 2, CARD_H / 2);
    this.myBattlefieldContainer.addChild(overlay);
    entry.overlay = overlay;
    return overlay;
  }

  private drawManaAbilityGrid(
    overlay: Container,
    card: Card,
    abilities: ReturnType<typeof getExpandedManaAbilities>,
  ): void {
    const cols = abilities.length > 2 ? 2 : abilities.length;
    const rows = Math.ceil(abilities.length / cols);
    const btnW = CARD_W / cols;
    const btnH = CARD_H / rows;
    const isOddLast = abilities.length % 2 !== 0;

    abilities.forEach((ab, i) => {
      const col = i % cols;
      const row = Math.floor(i / cols);
      const shouldSpan = cols === 2 && i === abilities.length - 1 && isOddLast;
      const currentW = shouldSpan ? CARD_W : btnW;

      const letters = extractManaLetters(ab.description);
      const letter = letters[0];
      const color = manaColorFor(letter, BLACK);

      const btn = new Graphics();
      const paintBtn = (highlighted: boolean) => {
        btn.clear();
        btn.roundRect(col * btnW, row * btnH, currentW, btnH, CARD_RADIUS);
        btn.fill({
          color,
          alpha: highlighted ? MANA_BUTTON_HOVER_ALPHA : MANA_BUTTON_ALPHA,
        });
        btn.stroke({
          color: WHITE,
          width: 1,
          alpha: highlighted
            ? MANA_BUTTON_STROKE_HOVER_ALPHA
            : MANA_BUTTON_STROKE_ALPHA,
        });
      };
      paintBtn(false);
      overlay.addChild(btn);

      const icon = this.createManaIcon(
        letter ?? OVERLAY_LABEL_TAP,
        cols === 2 ? 10 : 12,
        cols === 2 ? 10 : 14,
      );
      icon.x = col * btnW + currentW / 2;
      icon.y = row * btnH + btnH / 2;
      overlay.addChild(icon);

      this.wireOverlayButton(
        btn,
        card.id,
        () =>
          this.callbacks.onTapLandAbility?.(card.id, ab.abilityIndex, letter),
        (highlighted) => {
          paintBtn(highlighted);
          icon.scale.set(highlighted ? ICON_HOVER_SCALE : 1);
        },
      );
    });
  }

  private drawSingleActionButton(
    overlay: Container,
    card: Card,
    state: BattlefieldState,
    kind: ActionKind,
  ): void {
    const ring = this.theme?.cardRing ?? FALLBACK_ORANGE;
    let label = OVERLAY_LABEL_SELECT;
    let symbol: string | null = null;
    let color = ring;
    let idleAlpha = SELECT_BUTTON_ALPHA;
    let hoverAlpha = SELECT_BUTTON_HOVER_ALPHA;

    if (kind.isTappable) {
      label = OVERLAY_LABEL_TAP;
      symbol = SYMBOL_TAP;
      idleAlpha = ACTION_BUTTON_ALPHA;
      hoverAlpha = ACTION_BUTTON_HOVER_ALPHA;
    } else if (kind.isUntappable) {
      label = OVERLAY_LABEL_UNTAP;
      symbol = SYMBOL_UNTAP;
      color = this.theme?.promptAction.cancel ?? FALLBACK_GRAY;
      idleAlpha = ACTION_BUTTON_ALPHA;
      hoverAlpha = ACTION_BUTTON_HOVER_ALPHA;
    }

    const btn = new Graphics();
    const paintBtn = (highlighted: boolean) => {
      btn.clear();
      btn.roundRect(0, 0, CARD_W, CARD_H, CARD_RADIUS);
      btn.fill({ color, alpha: highlighted ? hoverAlpha : idleAlpha });
    };
    paintBtn(false);
    overlay.addChild(btn);

    // Prefer the MTG card symbol (T / Q) when we have one — falls back to
    // the text label for generic SELECT or while the SVG is loading.
    const centerIcon = symbol
      ? this.createManaIcon(symbol, 14, 18)
      : this.createLabelIcon(label);
    centerIcon.x = CARD_W / 2;
    centerIcon.y = CARD_H / 2;
    overlay.addChild(centerIcon);

    this.wireOverlayButton(
      btn,
      card.id,
      () => this.dispatchBattlefieldAction(card, state, kind),
      (highlighted) => {
        paintBtn(highlighted);
        centerIcon.scale.set(highlighted ? ICON_HOVER_SCALE : 1);
      },
    );
  }

  private createLabelIcon(label: string): Container {
    const icon = new Container();
    icon.eventMode = "none";
    const txt = new Text({ text: label, style: OVERLAY_LABEL_STYLE });
    txt.anchor.set(0.5);
    icon.addChild(txt);
    return icon;
  }

  /**
   * Wires an overlay button's pointer events — tap (with drag-guard), hover
   * feedback, plus keeping the parent card's hover state alive while the
   * cursor is over the button (so the overlay doesn't fade out when the
   * cursor leaves the sprite's hit area to interact with the overlay).
   *
   * The button also forwards `pointerdown` to the sprite's drag-start
   * handler — without this, overlay buttons (which sit above the sprite
   * in the display tree) would swallow the press and the user could
   * never drag a tappable/choosable card. If the press turns into a
   * real drag, `pointertap` bails out via `justDraggedCardIds`.
   */
  private wireOverlayButton(
    btn: Graphics,
    cardId: string,
    onTap: () => void,
    onHoverChange?: (highlighted: boolean) => void,
  ): void {
    btn.eventMode = "static";
    btn.cursor = "pointer";
    btn.on("pointerover", () => {
      this.cancelBattlefieldHoverClear();
      const entry = this.entries.get(cardId);
      if (entry) this.setBattlefieldCardHovered(entry.sprite);
      onHoverChange?.(true);
    });
    btn.on("pointerout", () => {
      onHoverChange?.(false);
      this.scheduleBattlefieldHoverClear(cardId);
    });
    btn.on("pointerdown", (e: FederatedPointerEvent) => {
      e.stopPropagation();
      const entry = this.entries.get(cardId);
      if (entry) this.onBattlefieldCardDown(entry.sprite, e);
    });
    btn.on("pointertap", (e: FederatedPointerEvent) => {
      e.stopPropagation();
      if (this.dragHandler.justDraggedCardIds.has(cardId)) return;
      onTap();
    });
  }

  private createManaIcon(
    label: string,
    fontSize: number,
    radius: number,
  ): Container {
    const icon = new Container();
    // Let pointer events pass through to the button graphic underneath.
    icon.eventMode = "none";
    const circle = new Graphics();
    circle.circle(0, 0, radius);
    circle.fill({ color: BLACK, alpha: ICON_BG_ALPHA });
    icon.addChild(circle);

    const tex = getManaSymbolTextureSync(label);
    if (tex) {
      icon.addChild(this.createManaSprite(tex, radius));
    } else {
      const style = OVERLAY_LABEL_STYLE.clone();
      style.fontSize = fontSize;
      const txt = new Text({ text: label, style });
      txt.anchor.set(0.5);
      icon.addChild(txt);

      if (label.length === 1 && /^[WUBRGCXTQ]$/.test(label)) {
        // Kick off load; next overlay rebuild will pick up the cached texture.
        loadManaSymbolTexture(label)
          .then(() => this.refreshBattlefieldOverlays())
          .catch(() => {});
      }
    }
    return icon;
  }

  private createManaSprite(texture: Texture, radius: number): Sprite {
    const sprite = new Sprite(texture);
    sprite.anchor.set(0.5);
    const size = radius * 1.6;
    sprite.width = size;
    sprite.height = size;
    return sprite;
  }

  private refreshBattlefieldOverlays(): void {
    if (!this.lastState) return;
    for (const entry of this.entries.values()) {
      if (entry.overlay?.visible) {
        this.rebuildBattlefieldOverlay(entry, this.lastState);
      }
    }
  }

  private dispatchBattlefieldAction(
    card: Card,
    state: BattlefieldState,
    kind: ActionKind,
  ): void {
    if (kind.isTappable) {
      const batch = this.selectedBatch(state.tappableLandIds, card.id);
      if (batch.length > 1) this.callbacks.onTapLands?.(batch);
      else this.callbacks.onTapLand?.(card);
    } else if (kind.isUntappable) {
      const batch = this.selectedBatch(state.untappableLandIds, card.id);
      if (batch.length > 1) this.callbacks.onUntapLands?.(batch);
      else this.callbacks.onUntapLand?.(card);
    } else if (kind.isChoosable) {
      this.callbacks.onClickCard?.(card);
    }
  }

  private selectedBatch(
    eligibleIds: string[] | undefined,
    cardId: string,
  ): string[] {
    if (!this.selectedCardIds.has(cardId) || this.selectedCardIds.size <= 1)
      return [];
    return [...this.selectedCardIds].filter((id) => eligibleIds?.includes(id));
  }

  // ═══════════════════════════════════════════════════════════════
  // Interaction (battlefield)
  // ═══════════════════════════════════════════════════════════════

  private onBattlefieldCardDown(
    sprite: CardSprite,
    e: FederatedPointerEvent,
  ): void {
    this.callbacks.onHoverCard?.(null);
    const local = this.root.toLocal(e.global);
    this.selectedCardIds = this.dragHandler.start(
      sprite.card.id,
      local.x,
      local.y,
      this.selectedCardIds,
      this.snapshotCurrentPositions(),
      e.shiftKey,
    );
    this.drawSelectionBadge();
    this.refreshSelectionRings();
  }

  private onBackgroundDown(e: FederatedPointerEvent): void {
    const local = this.root.toLocal(e.global);
    if (!e.shiftKey) {
      this.selectedCardIds.clear();
      this.drawSelectionBadge();
      this.refreshSelectionRings();
    }
    this.marquee.start(local.x, local.y, e.shiftKey);
  }

  private onGlobalMove(e: FederatedPointerEvent): void {
    const local = this.root.toLocal(e.global);
    if (this.marquee.isActive) {
      this.marquee.move(local.x, local.y);
      return;
    }

    const newPositions = this.dragHandler.move(local.x, local.y);
    if (!newPositions) return;
    // Active drag just crossed the move threshold — kill any hover preview
    // so it doesn't linger under the cursor while the user drags. Done here
    // (not at pointerdown) so a simple click that never moves doesn't also
    // dismiss the preview.
    this.callbacks.onDismissHoverPreview?.();
    for (const [id, pos] of newPositions) {
      const entry = this.entries.get(id);
      if (!entry) continue;
      entry.targetX = pos.x;
      entry.targetY = pos.y;
      // Snap sprite (and its overlay) directly to the cursor so the card
      // tracks 1:1 during drag. Without this the battlefield lerp (0.15)
      // makes dragging feel draggy/sticky — the sprite chases the cursor
      // with ~150ms of visible lag.
      entry.sprite.x = pos.x;
      entry.sprite.y = pos.y;
      if (entry.overlay?.visible) {
        entry.overlay.x = pos.x;
        entry.overlay.y = pos.y;
      }
      this.customPositions.set(id, pos);
    }
  }

  private onGlobalUp(): void {
    if (this.marquee.isActive) {
      this.selectedCardIds = this.marquee.end(
        this.snapshotCurrentPositions(),
        this.selectedCardIds,
      );
      this.drawSelectionBadge();
      this.refreshSelectionRings();
      return;
    }

    const result = this.dragHandler.end();
    if (!result?.wasDrag) return;
    for (const [id, pos] of this.customPositions) {
      const entry = this.entries.get(id);
      if (!entry) continue;
      entry.targetX = pos.x;
      entry.targetY = pos.y;
    }
  }

  private snapshotCurrentPositions(): Map<string, Point> {
    const positions = new Map<string, Point>();
    for (const [id, entry] of this.entries) {
      positions.set(id, { x: entry.sprite.x, y: entry.sprite.y });
    }
    return positions;
  }

  private drawSelectionBadge(): void {
    if (this.selectedCardIds.size === 0) {
      this.selectionBadge.visible = false;
      return;
    }
    this.selectionBadge.text = `${this.selectedCardIds.size} selected`;
    this.selectionBadge.visible = true;
    this.selectionBadge.x =
      this.app.renderer.width - this.selectionBadge.width - 8;
    this.selectionBadge.y = 6;
  }

  private refreshSelectionRings(): void {
    if (!this.lastState) return;
    for (const entry of this.entries.values()) {
      if (this.selectedCardIds.has(entry.sprite.card.id)) {
        entry.sprite.setRing(this.theme?.cardRing ?? FALLBACK_ORANGE);
      } else {
        this.applyBattlefieldRing(entry.sprite, this.lastState);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════
  // Hand
  // ═══════════════════════════════════════════════════════════════

  private pruneRemovedHandSprites(currentIds: Set<string>): void {
    for (const [id, sprite] of this.handSprites) {
      if (currentIds.has(id)) continue;
      this.handContainer.removeChild(sprite);
      sprite.destroy({ children: true });
      this.handSprites.delete(id);
      this.handTargets.delete(id);
    }
  }

  private computeHandDimensions() {
    const base = HAND_CARD_BASES[this.handSize];
    const params = SIZE_PARAMS[this.handSize];
    const canvasScale = Math.min(
      HAND_MAX_SCALE,
      Math.max(HAND_MIN_SCALE, this.app.renderer.width / HAND_REF_WIDTH),
    );
    const scale = Math.min(this.vScale, canvasScale);
    return {
      cardW: Math.round(base.cardW * scale),
      cardH: Math.round(base.cardH * scale),
      hoverLift: Math.round(params.hoverLift * scale),
      neighborPush: Math.round(params.neighborPush * scale),
      maxSpread: Math.round(params.maxSpread * scale),
      minSpread: Math.round(params.minSpread * scale),
      spreadWidth: Math.round(params.spreadWidth * scale),
    };
  }

  private createHandSprite(card: Card): CardSprite {
    const sprite = new CardSprite(card);
    sprite.eventMode = "static";
    sprite.cursor = card.isPlayable ? "grab" : "default";

    sprite.on("pointerenter", () => this.onHandCardEnter(sprite));
    sprite.on("pointerleave", () => this.onHandCardLeave(sprite));
    sprite.on("pointerdown", (e: FederatedPointerEvent) => {
      e.stopPropagation();
      if (sprite.card.isPlayable) {
        this.callbacks.onStartDrag?.(sprite.card, {
          x: e.globalX,
          y: e.globalY,
        });
      } else {
        this.callbacks.onClickCard_Hand?.(sprite.card);
      }
    });

    this.handContainer.addChild(sprite);
    this.handSprites.set(card.id, sprite);
    return sprite;
  }

  private onHandCardEnter(sprite: CardSprite): void {
    const idx = this.handIndexOf(sprite.card.id);
    if (idx < 0) return;
    this.cancelHandHoverHoldTimer();
    this.hoveredHandIndex = idx;
    this.recalcHandTargets();
    const screenBounds = this.hoveredHandSpriteBounds(sprite);
    this.callbacks.onHoverCard?.(sprite.card, screenBounds, {
      useAnchor: true,
      placement: "top-center",
    });
    this.callbacks.onHoverHandCard?.(sprite.card, screenBounds);
  }

  /**
   * Analytical bounds for the hovered hand sprite in canvas coordinates.
   * `sprite.getBounds()` would use the pre-lerp position — the scale is set
   * synchronously in `updateHand`, but the position is animated, so using
   * raw bounds momentarily anchors overlays to the un-lifted location.
   */
  private hoveredHandSpriteBounds(sprite: CardSprite): ScreenBounds {
    const target = this.handTargets.get(sprite.card.id);
    const centerX = target?.x ?? sprite.x;
    const centerY = target?.y ?? sprite.y;
    const width = CARD_W * sprite.scale.x;
    const height = CARD_H * sprite.scale.y;
    return {
      x: centerX - width / 2,
      y: centerY - height / 2,
      width,
      height,
    };
  }

  private onHandCardLeave(sprite: CardSprite): void {
    const idx = this.handIndexOf(sprite.card.id);
    if (this.hoveredHandIndex !== idx) return;

    // Hide the big preview immediately, but defer the actual hand-sprite
    // unhover so the HTML action menu can cancel the clear if the cursor
    // moves onto it.
    this.callbacks.onHoverCard?.(null);
    this.callbacks.onHoverHandCard?.(null);
    this.scheduleHandHoverCommit(idx);
  }

  private scheduleHandHoverCommit(idx: number): void {
    this.cancelHandHoverHoldTimer();
    this.pendingHandHoverLeaveIndex = idx;
    this.handHoverHoldTimer = window.setTimeout(() => {
      this.commitHandHoverLeave();
    }, HAND_HOVER_HOLD_MS);
  }

  private commitHandHoverLeave(): void {
    this.handHoverHoldTimer = null;
    const idx = this.pendingHandHoverLeaveIndex;
    this.pendingHandHoverLeaveIndex = null;
    if (this.destroyed) return;
    if (idx === null || this.hoveredHandIndex !== idx) return;
    this.hoveredHandIndex = null;
    this.recalcHandTargets();
  }

  private cancelHandHoverHoldTimer(): void {
    if (this.handHoverHoldTimer !== null) {
      window.clearTimeout(this.handHoverHoldTimer);
      this.handHoverHoldTimer = null;
    }
    this.pendingHandHoverLeaveIndex = null;
  }

  /** Called when the HTML action menu receives the cursor. */
  holdHandHover(): void {
    this.cancelHandHoverHoldTimer();
  }

  /** Called when the cursor leaves the HTML action menu. */
  releaseHandHover(): void {
    if (this.hoveredHandIndex === null) return;
    this.scheduleHandHoverCommit(this.hoveredHandIndex);
  }

  private handIndexOf(cardId: string): number {
    return this.lastHandState?.cards.findIndex((c) => c.id === cardId) ?? -1;
  }

  private applyHandCardHighlight(
    sprite: CardSprite,
    card: Card,
    isHovered: boolean,
  ): void {
    if (!card.isPlayable) {
      sprite.setRing(null);
      return;
    }
    const ring = this.theme?.cardRing ?? FALLBACK_ORANGE;
    if (isHovered) sprite.setHighlight(true, ring, PLAYABLE_HIGHLIGHT_ALPHA);
    else sprite.setRing(ring, PLAYABLE_RING_ALPHA);
  }

  private recalcHandTargets(): void {
    if (this.lastHandState) this.updateHand(this.lastHandState);
  }

  // ═══════════════════════════════════════════════════════════════
  // Per-frame animation
  // ═══════════════════════════════════════════════════════════════

  private tick = (): void => {
    for (const entry of this.entries.values())
      this.animateBattlefieldSprite(entry);
    for (const [id, target] of this.handTargets) {
      const sprite = this.handSprites.get(id);
      if (sprite) this.animateHandSprite(sprite, target);
    }
  };

  private animateBattlefieldSprite(entry: SpriteEntry): void {
    const s = entry.sprite;
    s.x = lerp(s.x, entry.targetX, BATTLEFIELD_LERP, SNAP_PX);
    s.y = lerp(s.y, entry.targetY, BATTLEFIELD_LERP, SNAP_PX);
    s.zIndex = entry.targetZIndex;

    const isHovered = this.hoveredCardId === s.card.id;
    const targetScale = isHovered ? HOVER_SCALE : 1;
    const nextScale = lerp(
      s.scale.x,
      targetScale,
      HOVER_SCALE_LERP,
      SNAP_SCALE,
    );
    s.scale.set(nextScale);

    if (entry.overlay?.visible) {
      entry.overlay.x = s.x;
      entry.overlay.y = s.y;
      entry.overlay.zIndex = entry.targetZIndex + Z_OVERLAY_OFFSET;
      entry.overlay.alpha = lerp(
        entry.overlay.alpha,
        isHovered ? 1 : 0,
        OVERLAY_FADE_LERP,
        SNAP_ALPHA,
      );
    }
  }

  private animateHandSprite(sprite: CardSprite, target: HandTarget): void {
    sprite.x = lerp(sprite.x, target.x, HAND_LERP, SNAP_PX);
    sprite.y = lerp(sprite.y, target.y, HAND_LERP, SNAP_PX);
    sprite.rotation = lerp(sprite.rotation, target.rot, HAND_LERP, SNAP_ROT);

    const dsx = target.scale - sprite.scale.x;
    if (Math.abs(dsx) > SNAP_HAND_SCALE) {
      const s = sprite.scale.x + dsx * HAND_LERP;
      sprite.scale.set(s, s * (CARD_H / CARD_W));
    }

    sprite.zIndex = target.zIndex;
  }
}
