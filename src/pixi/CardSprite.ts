import { Container, Sprite, Texture, Graphics, Text, TextStyle } from "pixi.js";
import type { Card } from "@/types/openmagic";
import { CARD_W, CARD_H } from "@/components/game/game.constants";
import { loadCardTexture } from "./textureCache";
import { BLACK, WHITE } from "./constants";

// Hand cards render at up to ~3.25× base scale (medium hover) and ~4.3× (large
// hover). Rasterize text textures high enough that they remain sharp across
// that range on top of the 3× canvas backing.
const TEXT_RASTER_RESOLUTION = 5;

const PT_STYLE = new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 10,
  fontWeight: "bold",
  fill: "#ffffff",
});

const BADGE_STYLE = new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 7,
  fontWeight: "bold",
  fill: "#ffffff",
});

const COUNTER_STYLE = new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 8,
  fontWeight: "bold",
  fill: "#ffffff",
});

const CHIP_STYLE = new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 7,
  fontWeight: "bold",
  fill: "#ffffff",
});

const NAME_STYLE = new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 8,
  fill: "#ffffff",
  wordWrap: true,
  wordWrapWidth: CARD_W - 8,
  align: "center",
});

// ── Colors ───────────────────────────────────────────────────────
const PLACEHOLDER_FILL = 0x1a1a2e;
const PLACEHOLDER_STROKE = 0x444466;
const COUNTER_DEFAULT_COLOR = 0x4b5563;
const PT_COLOR_NEUTRAL = 0x6b7280;
const PT_COLOR_LETHAL = 0xdc2626;
const PT_COLOR_BUFFED = 0x22c55e;
const PT_COLOR_DEBUFFED = 0xdc2626;
const HIGHLIGHT_FALLBACK = 0xfb923c;

// ── Geometry ─────────────────────────────────────────────────────
const CARD_RADIUS = 6;
const RING_RADIUS = 8;
const RING_INSET = 2;
const CHIP_RADIUS = 3;
const COUNTER_HEIGHT = 16;
const COUNTER_RADIUS = 8;
const KEYWORD_ROW_H = 12;
const MAX_VISIBLE_KEYWORDS = 4;

const BADGE_COLORS: Record<string, number> = {
  EXERTED: 0xf97316,
  MORPH: 0x4b5563,
  BESTOW: 0x14b8a6,
  TOKEN: 0xfbbf24,
  TRANSFORMED: 0xa855f7,
  PLOTTED: 0x6366f1,
  MADNESS: 0xdc2626,
  WARPED: 0x0891b2,
};

interface BadgeRule {
  label: string;
  test: (card: Card) => boolean;
  colorKey: keyof typeof BADGE_COLORS;
}

const BADGE_RULES: BadgeRule[] = [
  { label: "EXERTED",     test: (c) => !!c.exerted,        colorKey: "EXERTED" },
  { label: "MORPH",       test: (c) => !!c.isFaceDown,     colorKey: "MORPH" },
  { label: "BESTOW",      test: (c) => !!c.isBestowed,     colorKey: "BESTOW" },
  { label: "TRANSFORMED", test: (c) => !!c.isTransformed,  colorKey: "TRANSFORMED" },
  { label: "PLOTTED",     test: (c) => !!c.isPlotted,      colorKey: "PLOTTED" },
  { label: "MADNESS",     test: (c) => !!c.isMadnessExiled, colorKey: "MADNESS" },
  { label: "WARPED",      test: (c) => !!c.isWarpExiled,   colorKey: "WARPED" },
  { label: "TOKEN",       test: (c) => !!c.isToken,        colorKey: "TOKEN" },
];

const COUNTER_COLORS: Record<string, number> = {
  P1P1:      0x22c55e,
  M1M1:      0xdc2626,
  Loyalty:   0x3b82f6,
  Charge:    0xa855f7,
  Quest:     0xfacc15,
  Study:     0x06b6d4,
  Lore:      0xf59e0b,
  Age:       0x78716c,
  Time:      0x6366f1,
  Fade:      0x64748b,
  Level:     0xf97316,
  Storage:   0x14b8a6,
  Mining:    0xa16207,
  Brick:     0x9a3412,
  Depletion: 0xbe123c,
  Page:      0xa1a1aa,
};

const getCounterColor = (type: string) => COUNTER_COLORS[type] ?? COUNTER_DEFAULT_COLOR;

const COUNTER_LABEL_OVERRIDES: Record<string, string> = {
  P1P1: "+1",
  M1M1: "−1",
  Loyalty: "♦",
  Charge: "⚡",
};

const getCounterLabel = (type: string): string =>
  COUNTER_LABEL_OVERRIDES[type] ?? type.slice(0, 3);

const parseStat = (value: string | undefined): number => {
  if (!value) return 0;
  const n = parseInt(value, 10);
  return Number.isNaN(n) ? 0 : n;
};

const resolvePTBgColor = (card: Card): number => {
  const toughness = parseStat(card.toughness);
  if (card.damage != null && card.damage >= toughness) return PT_COLOR_LETHAL;
  if (card.basePower == null) return PT_COLOR_NEUTRAL;

  const curP = parseStat(card.power);
  const curT = toughness;
  const buffed = curP > card.basePower || curT > (card.baseToughness ?? 0);
  const debuffed = curP < card.basePower || curT < (card.baseToughness ?? 0);
  if (buffed) return PT_COLOR_BUFFED;
  if (debuffed) return PT_COLOR_DEBUFFED;
  return PT_COLOR_NEUTRAL;
};

export class CardSprite extends Container {
  card: Card;

  private imageSpr: Sprite;
  private imageMask: Graphics;
  private ringGfx: Graphics;
  private ptContainer: Container;
  private ptBg: Graphics;
  private ptText: Text;
  private badgeContainer: Container;
  private badgeBg: Graphics;
  private badgeText: Text;
  private counterContainer: Container;
  private keywordsContainer: Container;
  private placeholderGfx: Graphics;
  private nameText: Text;
  private _imageLoaded = false;

  constructor(card: Card) {
    super();
    this.card = card;
    this.eventMode = "static";
    this.cursor = "pointer";

    this.ringGfx = new Graphics();
    this.addChild(this.ringGfx);

    this.placeholderGfx = new Graphics();
    this.placeholderGfx.roundRect(0, 0, CARD_W, CARD_H, CARD_RADIUS);
    this.placeholderGfx.fill({ color: PLACEHOLDER_FILL, alpha: 0.8 });
    this.placeholderGfx.stroke({ color: PLACEHOLDER_STROKE, width: 1 });
    this.addChild(this.placeholderGfx);

    this.nameText = new Text({ text: card.name, style: NAME_STYLE });
    this.nameText.resolution = TEXT_RASTER_RESOLUTION;
    this.nameText.anchor.set(0.5);
    this.nameText.x = CARD_W / 2;
    this.nameText.y = CARD_H / 2;
    this.addChild(this.nameText);

    this.imageMask = new Graphics();
    this.imageMask.roundRect(0, 0, CARD_W, CARD_H, CARD_RADIUS);
    this.imageMask.fill(WHITE);
    this.addChild(this.imageMask);

    this.imageSpr = new Sprite(Texture.EMPTY);
    this.imageSpr.setSize(CARD_W, CARD_H);
    this.imageSpr.mask = this.imageMask;
    this.addChild(this.imageSpr);

    this.badgeContainer = new Container();
    this.badgeBg = new Graphics();
    this.badgeText = new Text({ text: "", style: BADGE_STYLE });
    this.badgeText.resolution = TEXT_RASTER_RESOLUTION;
    this.badgeContainer.addChild(this.badgeBg);
    this.badgeContainer.addChild(this.badgeText);
    this.badgeContainer.visible = false;
    this.addChild(this.badgeContainer);

    this.counterContainer = new Container();
    this.addChild(this.counterContainer);

    this.keywordsContainer = new Container();
    this.addChild(this.keywordsContainer);

    this.ptContainer = new Container();
    this.ptBg = new Graphics();
    this.ptText = new Text({ text: "", style: PT_STYLE });
    this.ptText.resolution = TEXT_RASTER_RESOLUTION;
    this.ptContainer.addChild(this.ptBg);
    this.ptContainer.addChild(this.ptText);
    this.ptContainer.visible = false;
    this.addChild(this.ptContainer);

    this.hitArea = { contains: (x: number, y: number) => x >= 0 && x <= CARD_W && y >= 0 && y <= CARD_H };

    this.pivot.set(CARD_W / 2, CARD_H / 2);
    this.loadImage();
  }

  private async loadImage(): Promise<void> {
    const tex = await loadCardTexture(
      this.card.name,
      this.card.imageUrl,
      this.card.isToken,
      this.card.setCode,
      this.card.cardNumber,
      "normal",
      this.card.isFaceDown,
    );
    if (this.destroyed) return;
    if (tex !== Texture.EMPTY) {
      this.imageSpr.texture = tex;
      this.imageSpr.setSize(CARD_W, CARD_H);
      this.placeholderGfx.visible = false;
      this.nameText.visible = false;
      this._imageLoaded = true;
    }
  }

  get imageLoaded(): boolean {
    return this._imageLoaded;
  }

  /**
   * Full update including tapped rotation + phased-out alpha. Use this on the
   * battlefield, where `tapped` is what drives the 90° rotation.
   */
  updateCard(card: Card): void {
    this.updateCardContent(card);
    this.rotation = card.tapped ? Math.PI / 2 : 0;
    this.alpha = card.phasedOut ? 0.3 : 1;
  }

  /**
   * Updates the card's visible content (art, P/T, badges, counters, keywords)
   * but does NOT touch `rotation` or `alpha`. Use this when an external
   * animation owns those properties — e.g. the hand layout lerps rotation
   * to the arc-fan angle and sets alpha based on dragging/casting state.
   * Calling the full `updateCard` there would reset the rotation to 0 on
   * every state update, causing a bumpy re-lerp back to the fan angle.
   */
  updateCardContent(card: Card): void {
    const nameChanged = card.name !== this.card.name || card.imageUrl !== this.card.imageUrl || card.isFaceDown !== this.card.isFaceDown;
    this.card = card;

    if (nameChanged) {
      this._imageLoaded = false;
      this.placeholderGfx.visible = true;
      this.nameText.visible = true;
      this.nameText.text = card.name;
      this.loadImage();
    }

    this.updatePT();
    this.updateBadge();
    this.updateCounters();
    this.updateKeywords();
  }

  private updatePT(): void {
    const card = this.card;
    const isCreature = card.types?.some((t) => t.toLowerCase() === "creature");
    if (!isCreature || !card.power || !card.toughness) {
      this.ptContainer.visible = false;
      return;
    }

    this.ptContainer.visible = true;
    this.ptText.text = `${card.power}/${card.toughness}`;
    const bgColor = resolvePTBgColor(card);

    const tw = this.ptText.width + 6;
    const th = this.ptText.height + 4;
    this.ptBg.clear();
    this.ptBg.roundRect(0, 0, tw, th, CHIP_RADIUS);
    this.ptBg.fill({ color: bgColor, alpha: 0.85 });

    this.ptText.x = 3;
    this.ptText.y = 2;
    this.ptContainer.x = CARD_W - tw - 3;
    this.ptContainer.y = CARD_H - th - 3;
  }

  private updateBadge(): void {
    const rule = BADGE_RULES.find((r) => r.test(this.card));
    if (!rule) {
      this.badgeContainer.visible = false;
      return;
    }

    this.badgeContainer.visible = true;
    this.badgeText.text = rule.label;

    const bw = this.badgeText.width + 8;
    const bh = this.badgeText.height + 4;
    this.badgeBg.clear();
    this.badgeBg.roundRect(0, 0, bw, bh, CHIP_RADIUS);
    this.badgeBg.fill({ color: BADGE_COLORS[rule.colorKey]!, alpha: 0.9 });

    this.badgeText.x = 4;
    this.badgeText.y = 2;
    this.badgeContainer.x = (CARD_W - bw) / 2;
    this.badgeContainer.y = 3;
  }

  private updateCounters(): void {
    this.counterContainer.removeChildren().forEach(c => c.destroy({ children: true }));
    const counters = this.card.counters;
    if (!counters) return;

    const entries = Object.entries(counters).filter(([, n]) => n > 0);
    if (entries.length === 0) return;

    let offsetX = 3;
    for (const [type, count] of entries) {
      const color = getCounterColor(type);
      const label = getCounterLabel(type);
      const displayText = count > 1 ? `${label} ${count}` : label;

      const badge = new Container();
      const bg = new Graphics();
      const txt = new Text({ text: displayText, style: COUNTER_STYLE });
      txt.resolution = TEXT_RASTER_RESOLUTION;
      txt.anchor.set(0, 0.5);
      txt.x = 4;
      txt.y = 8;

      const bw = txt.width + 8;
      bg.roundRect(0, 0, bw, COUNTER_HEIGHT, COUNTER_RADIUS);
      bg.fill({ color, alpha: 0.9 });
      bg.stroke({ color: BLACK, width: 1, alpha: 0.2 });

      badge.addChild(bg);
      badge.addChild(txt);
      badge.x = offsetX;
      badge.y = CARD_H - COUNTER_HEIGHT - 3;
      this.counterContainer.addChild(badge);
      offsetX += bw + 2;
    }
  }

  private updateKeywords(): void {
    this.keywordsContainer.removeChildren().forEach(c => c.destroy({ children: true }));
    const keywords = this.card.keywords;
    if (!keywords || keywords.length === 0) return;

    const visible = keywords.slice(0, MAX_VISIBLE_KEYWORDS);
    const hiddenCount = keywords.length - visible.length;

    let offsetX = 3;
    let offsetY = 3;
    const rowH = KEYWORD_ROW_H;

    const addChip = (text: string) => {
      const chip = new Container();
      const bg = new Graphics();
      const txt = new Text({ text, style: CHIP_STYLE });
      txt.resolution = TEXT_RASTER_RESOLUTION;
      txt.anchor.set(0, 0.5);
      txt.x = 3;
      txt.y = rowH / 2;

      const cw = txt.width + 6;
      if (offsetX + cw > CARD_W - 6) {
        offsetX = 3;
        offsetY += rowH + 2;
      }

      bg.roundRect(0, 0, cw, rowH, CHIP_RADIUS);
      bg.fill({ color: BLACK, alpha: 0.6 });

      chip.addChild(bg);
      chip.addChild(txt);
      chip.x = offsetX;
      chip.y = offsetY;
      this.keywordsContainer.addChild(chip);
      offsetX += cw + 2;
    };

    visible.forEach(kw => addChip(kw.split(":")[0]!));
    if (hiddenCount > 0) addChip(`+${hiddenCount}`);
  }

  setRing(color: number | null, alpha = 1): void {
    this.ringGfx.clear();
    if (color == null) return;
    this.drawRingStroke(color, alpha);
  }

  setHighlight(active: boolean, color = HIGHLIGHT_FALLBACK, alpha = 0.3): void {
    this.ringGfx.clear();
    if (!active) return;
    this.drawRingStroke(color, 1);
    this.ringGfx.roundRect(0, 0, CARD_W, CARD_H, CARD_RADIUS);
    this.ringGfx.fill({ color, alpha });
  }

  private drawRingStroke(color: number, alpha: number): void {
    this.ringGfx.roundRect(
      -RING_INSET,
      -RING_INSET,
      CARD_W + RING_INSET * 2,
      CARD_H + RING_INSET * 2,
      RING_RADIUS,
    );
    this.ringGfx.stroke({ color, width: 2, alpha });
  }
}
