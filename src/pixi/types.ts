import type { Card, ActivatableAbilityInfo } from "@/types/openmagic";
import type { HAND_CARD_BASES } from "@/components/game/game.styles";

export type HandSize = keyof typeof HAND_CARD_BASES;

export interface ScreenBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ScreenPos {
  x: number;
  y: number;
}

export type HoverPlacement = "auto" | "top-center";

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
