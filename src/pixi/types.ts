import type { Card, ActivatableAbilityInfo } from "@/types/openmagic";
import type { HAND_CARD_BASES } from "@/components/game/game.styles";

export type HandSize = keyof typeof HAND_CARD_BASES;

export interface ScreenBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Sub-rectangle of the canvas used for auto-placement + hand layout.
 *  The canvas itself can be larger (covering both halves of the board so
 *  arrows span everything) while gameplay sprites stay inside this rect. */
export type PlayZoneRect = ScreenBounds;

export interface ScreenPos {
  x: number;
  y: number;
}

export type HoverPlacement = "auto" | "top-center";

/**
 * Arrow endpoint referenced by game-entity identity so the Pixi scene can
 * resolve the current position from its own sprite maps (canvas-local) or
 * fall back to DOM query (player panels, stack cards).
 */
export type ArrowEndpoint =
  | { kind: "card"; id: string }
  | { kind: "player"; id: string }
  | { kind: "stack"; id: string }
  | { kind: "placement-ghost" };

export type ArrowType =
  | "attack"
  | "block"
  | "hostile-target"
  | "friendly-target"
  | "placement";

export interface ArrowSpec {
  from: ArrowEndpoint;
  to: ArrowEndpoint;
  type: ArrowType;
}

/**
 * Cursor-following arrow shown during target selection. Source is a
 * React-rendered element with `data-casting-card={id}` (StackDisplay).
 * Target is either a specific card/player (locked target) or the cursor.
 */
export interface CastingArrowSpec {
  castingCardId: string;
  /** When set, the arrow locks onto this card or player id. */
  targetId?: string | null;
  hostile: boolean;
}

export interface GameCanvasCallbacks {
  onClickCard?: (card: Card) => void;
  onClickAnyCard?: (card: Card) => void;
  onHoverCard?: (
    card: Card | null,
    screenBounds?: ScreenBounds,
    options?: { useAnchor?: boolean; placement?: HoverPlacement },
  ) => void;
  onFlipCard?: () => void;
  onStartDrag?: (card: Card, screenPos: ScreenPos) => void;
  onClickCard_Hand?: (card: Card) => void;
  onHoverHandCard?: (card: Card | null, screenBounds?: ScreenBounds) => void;
  onTargetPlayer?: (playerId: string) => void;
  onTapLand?: (card: Card) => void;
  onTapLands?: (cardIds: string[]) => void;
  onTapLandAbility?: (cardId: string, abilityIndex: number, color?: string) => void;
  onUntapLand?: (card: Card) => void;
  onUntapLands?: (cardIds: string[]) => void;
  onAttackerClick?: (card: Card) => void;
  onCastSpell?: (cardId: string) => void;
  /**
   * Dismiss the hover preview immediately (no 250ms grace). Used when
   * the scene begins a drag so the preview doesn't linger on the cursor.
   */
  onDismissHoverPreview?: () => void;
}

export interface BattlefieldState {
  cards: Card[];
  pendingCardIds?: string[];
  attackingCardIds?: string[];
  tappableLandIds?: string[];
  untappableLandIds?: string[];
  manaAbilityOptions?: ActivatableAbilityInfo[];
  hostileTargeting?: boolean;
}

export interface HandState {
  cards: Card[];
  draggingCardId?: string;
  castingCardId?: string | null;
}

export interface CardSpriteData {
  card: Card;
  x: number;
  y: number;
  tapped: boolean;
  ringColor: number | null;
  ringAlpha: number;
  phasedOut: boolean;
  summoningSick: boolean;
}
