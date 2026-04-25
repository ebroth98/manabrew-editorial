import type { Card as XMageCard, Player, ActivatableAbilityInfo } from "@/types/openmagic";
import type { GameLogEntry } from "@/types/gameLog";
import type { GameSnapshotEntry } from "@/types/gameSnapshot";
import type { PromptType } from "@/types/promptType";
import type { PlacementGhost } from "@/components/game/zones/FreeBattlefield";
import type { HandActionOption } from "@/stores/useGameUIStore";
import type { PixiGameScene } from "@/pixi/PixiGameScene";

export type PromptActionType = PromptType;

export interface CombatAssignment {
  blockerId: string;
  attackerId: string;
}

export type FlashItem =
  | { kind: "card"; cardId: string; cardName: string; setCode: string }
  | { kind: "turn"; playerId: string; playerName: string };

/** Seat identifier used to resolve per-player theme colours. Source of
 *  truth for `playerColors.<seat>` theme keys and for `OPPONENT_SEATS`
 *  index → seat mapping. */
export type PlayerSeat = "self" | "opponent1" | "opponent2" | "opponent3";

/** Ordered list of opponent seats, indexed by `opponentIndex`. Keep in
 *  sync with `PlayerSeat` — TS will fail compilation if they diverge. */
export const OPPONENT_SEATS: readonly Exclude<PlayerSeat, "self">[] = [
  "opponent1",
  "opponent2",
  "opponent3",
] as const;

export interface OpponentHalfProps {
  player: Player;
  /** 0-based opponent index for seat color assignment. */
  opponentIndex: number;
  permanents: XMageCard[];
  graveyard: XMageCard[];
  exile: XMageCard[];
  commandZone?: XMageCard[];
  isTargetable: boolean;
  isSelectedTarget?: boolean;
  onTarget: () => void;
  isFlashing: boolean;
  activePlayerId: string;
  priorityPlayerId: string;
  step: string;
  promptType: PromptType | undefined;
  pendingAttacker: string | null;
  attackerIds?: string[];
  onClickCard: (card: XMageCard) => void;
  onClickAnyCard: (card: XMageCard) => void;
  onHoverCard: (
    card: XMageCard | null,
    e?: React.MouseEvent,
    options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect },
  ) => void;
  onFlipCard: () => void;
  showBackFace: boolean;
  onOpenZone: (title: string, cards: XMageCard[], onClickCard?: (cardId: string) => void) => void;
  zonePanelSide: "left" | "right";
  zonePanelOrder: ("library" | "graveyard" | "exile")[];
  isMonarch?: boolean;
  hasInitiative?: boolean;
  placementGhost?: PlacementGhost | null;
  hostileTargeting?: boolean;
  manaAbilityOptions?: ActivatableAbilityInfo[];
  onTapLandAbility?: (cardId: string, abilityIndex: number, color?: string) => void;
  /** Populated by the opponent's Pixi canvas so the full-board arrow
   *  layer can resolve sprite positions for opponent permanents
   *  without round-tripping through DOM queries. */
  pixiSceneRef?: React.MutableRefObject<PixiGameScene | null>;
}

export interface BattlefieldZoneProps {
  cards: XMageCard[];
  label: string;
  emptyLabel: string;
  className?: string;
  zoneBg?: string;
  minHeight?: number;
  topReserved?: number;
  onClickCard?: (card: XMageCard) => void;
  onClickAnyCard?: (card: XMageCard) => void;
  onHoverCard?: (
    card: XMageCard | null,
    e?: React.MouseEvent,
    options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect },
  ) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  pendingCardIds?: string[];
  attackingCardIds?: string[];
  tappableLandIds?: string[];
  onTapLand?: (card: XMageCard) => void;
  /** Mana ability options for tappable lands (per-color tap buttons on dual lands). */
  manaAbilityOptions?: ActivatableAbilityInfo[];
  /** Tap a land with a specific mana ability (dual land color choice). */
  onTapLandAbility?: (cardId: string, abilityIndex: number, color?: string) => void;
  untappableLandIds?: string[];
  onUntapLand?: (card: XMageCard) => void;
  leftReserved?: number;
  rightReserved?: number;
  landsAtTop?: boolean;
  placementGhost?: PlacementGhost | null;
  hostileTargeting?: boolean;
}

export interface HandDisplayProps {
  cards: XMageCard[];
  onHoverCard?: (
    card: XMageCard | null,
    e?: React.MouseEvent,
    options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect },
  ) => void;
  onStartDrag?: (card: XMageCard, e: React.MouseEvent) => void;
  onClickCard?: (card: XMageCard, e?: React.MouseEvent) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  draggingCardId?: string;
  castingCardId?: string | null;
  getActions?: (card: XMageCard) => HandActionOption[];
  onSelectAction?: (action: HandActionOption) => void;
  /**
   * Optional selection overlay — used by the mulligan flows to drive the
   * hand without spawning a separate modal. When `selectionMode` is on:
   *   - Clicking a card invokes `onCardToggle` instead of drag/cast.
   *   - Cards in `selectedIds` drop below the arc, un-tilt, and wear a
   *     red ring + "→ Library bottom" pill.
   *   - The normal drag / cast / tug-reject paths are disabled.
   */
  selectionMode?: boolean;
  selectedIds?: Set<string>;
  onCardToggle?: (cardId: string) => void;
}

export interface RightActionPanelProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  gameLog: GameLogEntry[];
  onHoverLogCard: (
    cardId: string | null,
    e?: React.MouseEvent,
    options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect },
  ) => void;
  resolveCardName: (cardId: string) => string;
  resolvePlayerName: (playerId: string) => string;
  snapshots: GameSnapshotEntry[];
  canRestoreSnapshots: boolean;
  onRestoreSnapshot: (checkpointId: number) => void;
}

export interface MainActionOverlayProps {
  promptType?: PromptActionType;
  isWaitingForResponse: boolean;
  isAutoPassing: boolean;
  isPassingUntilEot: boolean;
  availableAttackerIds: string[];
  pendingAttackers: string[];
  onPassPriority: () => void;
  onPassUntilEot: () => void;
  selectedAttackDefenderId?: string | null;
  selectedAttackDefenderLabel?: string | null;
  onDeclareAttackers: (attackerIds: string[], defenderId?: string) => void;
  pendingAttacker: string | null;
  attackerIds: string[];
  blockAssignments: CombatAssignment[];
  onDeclareBlockers: (assignments: CombatAssignment[]) => void;
  onOpenStack: () => void;
  onConcede: () => void;
  resolveCardName: (cardId: string) => string;
  isMyPriority: boolean;
  turn: number;
  activePlayerName: string;
  isMyTurn: boolean;
  step: string;
  payManaCostInfo: {
    cardName: string;
    manaCost: string;
    manaPool: Record<string, number>;
    canConfirmFromPool: boolean;
  } | null;
  onPayManaCost: () => void;
  onAutoManaCost: () => void;
  onCancelManaCost: () => void;
  // Mulligan (live inside the prompt slot with Pass Priority so the
  // player never leaves the board for a keep/mulligan decision).
  mulliganCount?: number;
  onMulliganKeep?: () => void;
  onMulliganDraw?: () => void;
  mulliganPutBackCount?: number;
  mulliganSelectedCount?: number;
  onMulliganPutBackConfirm?: () => void;
}
