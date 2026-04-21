import { Container, Sprite, Texture, Graphics, Text, TextStyle } from "pixi.js";
import type { Card } from "@/types/openmagic";
import { CARD_W, CARD_H } from "@/components/game/game.constants";
import { loadCardTexture } from "./textureCache";
import { adaptTheme, type PixiThemeColors } from "./themeAdapter";
import { getGameThemeColors } from "@/components/game/game.theme";

/**
 * Shared, mutable theme reference used by every `CardSprite` instance.
 * `PixiGameScene.setTheme` calls `setCardSpriteTheme` so every sprite
 * repaints against the active preset without needing to thread the
 * theme through the Container constructor.
 */
// Seeded from the active preset so every sprite can draw correctly from
// construction time; `setCardSpriteTheme` then keeps it in sync with live
// preset / overrides changes.
let activeTheme: PixiThemeColors = adaptTheme(getGameThemeColors());

/** TextStyle instances whose `fill` tracks the theme's `textOnTinted` colour.
 *  Each call to `setCardSpriteTheme` updates them in place so already-rendered
 *  Text objects repaint without needing to be replaced. */
const TINTED_TEXT_STYLES: TextStyle[] = [];

export function setCardSpriteTheme(theme: PixiThemeColors): void {
  activeTheme = theme;
  for (const style of TINTED_TEXT_STYLES) {
    style.fill = theme.textOnTinted.source;
  }
}

function registerTintedTextStyle(style: TextStyle): TextStyle {
  TINTED_TEXT_STYLES.push(style);
  return style;
}

// Hand cards render at up to ~3.25× base scale (medium hover) and ~4.3× (large
// hover). Rasterize text textures high enough that they remain sharp across
// that range on top of the 3× canvas backing.
const TEXT_RASTER_RESOLUTION = 5;

// `tintedTextFill` is recomputed whenever the active theme changes; each
// registered TextStyle has its `fill` rewritten in place so already-
// rendered Text objects re-tint without being replaced.
const tintedTextFill = (): string => activeTheme.textOnTinted.source;

const PT_STYLE = registerTintedTextStyle(new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 10,
  fontWeight: "bold",
  fill: tintedTextFill(),
}));

const BADGE_STYLE = registerTintedTextStyle(new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 6,
  fontWeight: "bold",
  fill: tintedTextFill(),
}));

const COUNTER_STYLE = registerTintedTextStyle(new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 8,
  fontWeight: "bold",
  fill: tintedTextFill(),
}));

const CHIP_STYLE = registerTintedTextStyle(new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 7,
  fontWeight: "bold",
  fill: tintedTextFill(),
}));

const NAME_STYLE = registerTintedTextStyle(new TextStyle({
  fontFamily: "system-ui, -apple-system, sans-serif",
  fontSize: 8,
  fill: tintedTextFill(),
  wordWrap: true,
  wordWrapWidth: CARD_W - 8,
  align: "center",
}));


// ── Geometry ─────────────────────────────────────────────────────
const CARD_RADIUS = 6;
const RING_RADIUS = 8;
const RING_INSET = 2;
const CHIP_RADIUS = 3;
const COUNTER_HEIGHT = 16;
const COUNTER_RADIUS = 8;
const KEYWORD_ROW_H = 12;
const MAX_VISIBLE_KEYWORDS = 4;
// Fraction of the card height occupied by the title line (card name +
// mana cost). Badges sit just below this band so the mana cost stays
// unobstructed regardless of hover scale.
const BADGE_TITLE_BAND_FRAC = 0.1;

type CardStatusKey = keyof PixiThemeColors["cardStatus"];

interface BadgeRule {
  label: string;
  test: (card: Card) => boolean;
  colorKey: CardStatusKey;
}

const BADGE_RULES: BadgeRule[] = [
  { label: "EXERTED",     test: (c) => !!c.exerted,        colorKey: "exerted" },
  { label: "MORPH",       test: (c) => !!c.isFaceDown,     colorKey: "morph" },
  { label: "BESTOW",      test: (c) => !!c.isBestowed,     colorKey: "bestow" },
  { label: "TRANSFORMED", test: (c) => !!c.isTransformed,  colorKey: "transformed" },
  { label: "PLOTTED",     test: (c) => !!c.isPlotted,      colorKey: "plotted" },
  { label: "MADNESS",     test: (c) => !!c.isMadnessExiled, colorKey: "madness" },
  { label: "WARPED",      test: (c) => !!c.isWarpExiled,   colorKey: "warped" },
  { label: "TOKEN",       test: (c) => !!c.isToken,        colorKey: "token" },
];

function badgeColor(key: CardStatusKey): number {
  return activeTheme.cardStatus[key];
}

/** Static mapping from counter-type string (as it appears on the card
 *  state) to the `PixiThemeColors.counter` key. Any type not listed here
 *  falls through to `counter.default`. */
const COUNTER_TYPE_KEYS: Record<string, keyof PixiThemeColors["counter"]> = {
  P1P1:      "p1p1",
  M1M1:      "m1m1",
  Loyalty:   "loyalty",
  Charge:    "charge",
  Quest:     "quest",
  Study:     "study",
  Lore:      "lore",
  Age:       "age",
  Time:      "time",
  Fade:      "fade",
  Level:     "level",
  Storage:   "storage",
  Mining:    "mining",
  Brick:     "brick",
  Depletion: "depletion",
  Page:      "page",
};

function getCounterColor(type: string): number {
  const palette = activeTheme.counter;
  const key = COUNTER_TYPE_KEYS[type];
  return key ? palette[key] : palette.default;
}

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
  const pt = activeTheme.pt;
  const toughness = parseStat(card.toughness);
  if (card.damage != null && card.damage >= toughness) return pt.lethal;
  if (card.basePower == null) return pt.neutral;

  const curP = parseStat(card.power);
  const curT = toughness;
  const buffed = curP > card.basePower || curT > (card.baseToughness ?? 0);
  const debuffed = curP < card.basePower || curT < (card.baseToughness ?? 0);
  if (buffed) return pt.buffed;
  if (debuffed) return pt.debuffed;
  return pt.neutral;
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
    this.placeholderGfx.fill({ color: activeTheme.cardPlaceholder.fill, alpha: 0.8 });
    this.placeholderGfx.stroke({ color: activeTheme.cardPlaceholder.stroke, width: 1 });
    this.addChild(this.placeholderGfx);

    this.nameText = new Text({ text: card.name, style: NAME_STYLE });
    this.nameText.resolution = TEXT_RASTER_RESOLUTION;
    this.nameText.anchor.set(0.5);
    this.nameText.x = CARD_W / 2;
    this.nameText.y = CARD_H / 2;
    this.addChild(this.nameText);

    this.imageMask = new Graphics();
    this.imageMask.roundRect(0, 0, CARD_W, CARD_H, CARD_RADIUS);
    this.imageMask.fill(activeTheme.canvas.neutral);
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

    const bw = this.badgeText.width + 5;
    const bh = this.badgeText.height + 2;
    this.badgeBg.clear();
    this.badgeBg.roundRect(0, 0, bw, bh, CHIP_RADIUS);
    this.badgeBg.fill({ color: badgeColor(rule.colorKey), alpha: 0.9 });

    this.badgeText.x = 2.5;
    this.badgeText.y = 1;
    // Sit the badge just below the MTG title line instead of on top of it.
    // A top-centered badge would otherwise cover the mana cost pip cluster
    // (top-right of the card frame) when the hand hover scales the card up,
    // and the mana cost is the piece of information the player most needs
    // to read at a glance.
    const titleBandY = Math.round(CARD_H * BADGE_TITLE_BAND_FRAC);
    this.badgeContainer.x = (CARD_W - bw) / 2;
    this.badgeContainer.y = titleBandY;
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
      bg.stroke({ color: activeTheme.canvas.shadow, width: 1, alpha: 0.2 });

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
    // Start the keyword chip strip just under the MTG title line — matches
    // the badge band (BADGE_TITLE_BAND_FRAC) so card name + mana cost in
    // the top-right stay readable at every hover scale.
    let offsetY = Math.round(CARD_H * BADGE_TITLE_BAND_FRAC);
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
      bg.fill({ color: activeTheme.canvas.shadow, alpha: 0.6 });

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

  setHighlight(active: boolean, color = activeTheme.cardRing, alpha = 0.3): void {
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
