import type { Card as XMageCard, Player, ActivatableAbilityInfo } from "@/types/openmagic";
import type { GameLogEntry } from "@/types/gameLog";
import type { GameSnapshotEntry } from "@/types/gameSnapshot";
import type { PromptType } from "@/types/promptType";
import type { PlacementGhost } from "@/components/game/zones/FreeBattlefield";
import type { HandActionOption } from "@/stores/useGameUIStore";

export type PromptActionType = PromptType;

export interface CombatAssignment {
  blockerId: string;
  attackerId: string;
}

export type FlashItem =
  | { kind: "card"; cardId: string; cardName: string; setCode: string }
  | { kind: "turn"; playerId: string; playerName: string };

export interface OpponentHalfProps {
  player: Player;
  permanents: XMageCard[];
  graveyard: XMageCard[];
  exile: XMageCard[];
  commandZone?: XMageCard[];
  isTargetable: boolean;
  onTarget: () => void;
  isFlashing: boolean;
  activePlayerId: string;
  priorityPlayerId: string;
  promptType: PromptType | undefined;
  pendingAttacker: string | null;
  attackerIds?: string[];
  onClickCard: (card: XMageCard) => void;
  onClickAnyCard: (card: XMageCard) => void;
  onHoverCard: (card: XMageCard | null, e?: React.MouseEvent, options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect }) => void;
  onFlipCard: () => void;
  showBackFace: boolean;
  onOpenZone: (title: string, cards: XMageCard[], onClickCard?: (cardId: string) => void) => void;
  zonePanelSide: "left" | "right";
  zonePanelOrder: ("library" | "graveyard" | "exile")[];
  placementGhost?: PlacementGhost | null;
  hostileTargeting?: boolean;
  manaAbilityOptions?: ActivatableAbilityInfo[];
  onTapLandAbility?: (cardId: string, abilityIndex: number, color?: string) => void;
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
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent, options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect }) => void;
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
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent, options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect }) => void;
  onStartDrag?: (card: XMageCard, e: React.MouseEvent) => void;
  onClickCard?: (card: XMageCard, e?: React.MouseEvent) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  draggingCardId?: string;
  castingCardId?: string | null;
  getActions?: (card: XMageCard) => HandActionOption[];
  onSelectAction?: (action: HandActionOption) => void;
}

export interface RightActionPanelProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  gameLog: GameLogEntry[];
  onHoverLogCard: (cardId: string | null, e?: React.MouseEvent, options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect }) => void;
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
  onDeclareAttackers: (attackerIds: string[]) => void;
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
  payManaCostInfo: { cardName: string; manaCost: string; manaPool: Record<string, number> } | null;
  onPayManaCost: () => void;
  onCancelManaCost: () => void;
}
