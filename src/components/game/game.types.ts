import type { Card as XMageCard, Player } from "@/types/xmage";
import type { GameLogEntry } from "@/types/gameLog";
import type { GameSnapshotEntry } from "@/types/gameSnapshot";
import type { PromptType } from "@/types/promptType";

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
  onHoverCard: (card: XMageCard | null, e?: React.MouseEvent) => void;
  onFlipCard: () => void;
  showBackFace: boolean;
  onOpenZone: (title: string, cards: XMageCard[], onClickCard?: (cardId: string) => void) => void;
  zonePanelSide: "left" | "right";
  zonePanelOrder: ("library" | "graveyard" | "exile")[];
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
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  pendingCardIds?: string[];
  attackingCardIds?: string[];
  tappableLandIds?: string[];
  onTapLand?: (card: XMageCard) => void;
  untappableLandIds?: string[];
  onUntapLand?: (card: XMageCard) => void;
  leftReserved?: number;
  rightReserved?: number;
  landsAtTop?: boolean;
}

export interface HandDisplayProps {
  cards: XMageCard[];
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent) => void;
  onStartDrag?: (card: XMageCard, e: React.MouseEvent) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  draggingCardId?: string;
}

export interface RightActionPanelProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  gameLog: GameLogEntry[];
  onHoverLogCard: (cardId: string | null, e?: React.MouseEvent) => void;
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
