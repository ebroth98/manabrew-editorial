import { useGameStore } from "@/stores/useGameStore";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import type { Card as XMageCard, Player } from "@/types/xmage";
import { Card } from "@/components/game/Card";
import { FreeBattlefield } from "@/components/game/FreeBattlefield";
import { CardPreview } from "@/components/game/CardPreview";
import { ZoneViewer } from "@/components/game/ZoneViewer";
import { ZoneTargetSelector } from "@/components/game/ZoneTargetSelector";
import { LibraryPeekModal, type LibraryPeekMode } from "@/components/game/LibraryPeekModal";
import { SpellStackModal } from "@/components/game/SpellStackModal";
import { ChooseModeModal } from "@/components/game/ChooseModeModal";
import { ChooseOptionalTriggerModal } from "@/components/game/ChooseOptionalTriggerModal";
import { KickerModal, BuybackModal, MultikickerModal, ReplicateModal, AlternativeCostModal } from "@/components/game/CostModal";
import { ChooseColorModal } from "@/components/game/ChooseColorModal";
import { ChooseCardsModal } from "@/components/game/ChooseCardsModal";
import { RightActionPanel } from "@/components/game/RightActionPanel";
import { ZoneActionColumn } from "@/components/game/ZoneActionColumn";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { CardOverlayButton } from "@/components/game/CardOverlayButton";
import { ArrowOverlay } from "@/components/game/ArrowOverlay";
import { useGameArrows } from "@/components/game/useGameArrows";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { cn } from "@/lib/utils";
import {
  Heart,
  Layers,
  Sword,
} from "lucide-react";
import { Navigate, useLocation } from "react-router-dom";

const AVATAR_COLORS = [
  "bg-blue-600 text-white",
  "bg-purple-600 text-white",
  "bg-red-600 text-white",
  "bg-green-700 text-white",
  "bg-orange-500 text-white",
  "bg-pink-600 text-white",
  "bg-teal-600 text-white",
  "bg-indigo-600 text-white",
];

function getAvatarColor(name: string): string {
  const hash = name.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0);
  return AVATAR_COLORS[hash % AVATAR_COLORS.length];
}

function getInitials(name: string): string {
  return name
    .split(" ")
    .map((w) => w[0] ?? "")
    .join("")
    .toUpperCase()
    .slice(0, 2);
}

// Phase bar definitions — must match phase_to_step() in src-tauri/src/game_view_dto.rs
const PHASES = [
  { id: "untap", label: "Untap", short: "UNT" },
  { id: "upkeep", label: "Upkeep", short: "UP" },
  { id: "draw", label: "Draw", short: "DR" },
  { id: "main1", label: "Main 1", short: "M1" },
  { id: "begin_combat", label: "Begin Combat", short: "BC" },
  { id: "declare_attackers", label: "Attackers", short: "ATK" },
  { id: "declare_blockers", label: "Blockers", short: "BLK" },
  { id: "first_strike_damage", label: "1st Strike", short: "1ST" },
  { id: "combat_damage", label: "Damage", short: "DMG" },
  { id: "end_combat", label: "End Combat", short: "EC" },
  { id: "main2", label: "Main 2", short: "M2" },
  { id: "end", label: "End", short: "END" },
  { id: "cleanup", label: "Cleanup", short: "CL" },
];

const ZONE_COLUMN_RESERVED_PX = 68;

const MANA_COLORS = [
  { key: "W", bg: "bg-yellow-50 border-yellow-200", text: "text-yellow-800" },
  { key: "U", bg: "bg-blue-100 border-blue-300", text: "text-blue-800" },
  { key: "B", bg: "bg-gray-800 border-gray-600", text: "text-gray-100" },
  { key: "R", bg: "bg-red-100 border-red-300", text: "text-red-800" },
  { key: "G", bg: "bg-green-100 border-green-300", text: "text-green-800" },
  { key: "C", bg: "bg-gray-100 border-gray-300", text: "text-gray-700" },
];

function ManaPool({ pool }: { pool: Record<string, number> }) {
  const total = Object.values(pool).reduce((a, b) => a + b, 0);
  if (total === 0)
    return <span className="text-xs text-muted-foreground italic">Empty</span>;
  return (
    <div className="flex gap-1.5 flex-wrap items-center">
      {MANA_COLORS.flatMap(({ key }, i, arr) => {
        const count = pool[key] ?? 0;
        if (count === 0) return [];
        const items = [
          <span key={key} className="inline-flex items-center gap-0.5">
            <ManaSymbols cost={key} size="md" />
            {count > 1 && (
              <span className="text-xs font-bold text-foreground">{count}</span>
            )}
          </span>,
        ];
        // Add separator if there are more non-zero colors after this one
        const hasMore = arr.slice(i + 1).some(({ key: k }) => (pool[k] ?? 0) > 0);
        if (hasMore) {
          items.push(
            <span key={`sep-${key}`} className="text-muted-foreground/40 text-xs select-none">|</span>
          );
        }
        return items;
      })}
    </div>
  );
}

function PlayerPanel({
  player,
  isOpponent,
  isActiveTurn,
  isPriorityPlayer,
  isTargetable,
  onTarget,
  isFlashing,
  onOpenCommandZone,
  commandZoneCount = 0,
}: {
  player: Player;
  isOpponent: boolean;
  isActiveTurn?: boolean;
  /** True when this player currently holds priority (can play spells/abilities). */
  isPriorityPlayer?: boolean;
  isTargetable?: boolean;
  onTarget?: () => void;
  isFlashing?: boolean;
  onOpenCommandZone?: () => void;
  commandZoneCount?: number;
}) {
  const totalCmdDmg = Object.values(player.commanderDamage ?? {}).reduce(
    (a, b) => a + b,
    0,
  );

  return (
    <div
      data-player-id={player.id}
      className={cn(
        "flex items-center gap-3 px-3 py-2 rounded-lg border border-border bg-card text-sm transition-colors",
        isPriorityPlayer &&
          !isTargetable &&
          "bg-purple-50/50 dark:bg-purple-950/15 shadow-[inset_0_0_0_2px_rgba(168,85,247,0.95)]",
        isActiveTurn &&
          !isTargetable &&
          !isPriorityPlayer &&
          (isOpponent
            ? "bg-orange-50/40 dark:bg-orange-950/10 shadow-[inset_0_0_0_2px_rgba(251,146,60,0.95)]"
            : "bg-green-50/40 dark:bg-green-950/10 shadow-[inset_0_0_0_2px_rgba(34,197,94,0.95)]"),
        isTargetable &&
          "cursor-pointer hover:bg-red-50 dark:hover:bg-red-950/30 shadow-[inset_0_0_0_2px_rgba(248,113,113,0.95)]",
        isFlashing && "animate-player-turn-flash",
      )}
      onClick={isTargetable ? onTarget : undefined}
      title={isTargetable ? `Target ${player.name}` : undefined}
    >
      <Avatar className="h-8 w-8 shrink-0">
        <AvatarFallback
          className={cn("text-xs font-bold", getAvatarColor(player.name))}
        >
          {getInitials(player.name)}
        </AvatarFallback>
      </Avatar>
      <div className="flex items-center gap-1 shrink-0">
        <Heart className="h-3.5 w-3.5 text-red-500" />
        <span className="font-bold">{player.life}</span>
      </div>
      <div className="font-semibold truncate min-w-0">{player.name}</div>
      {isOpponent && isActiveTurn && (
        <span
          className={cn(
            "text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0",
            isOpponent
              ? "bg-orange-100 text-orange-700 dark:bg-orange-950/40 dark:text-orange-400"
              : "bg-green-100 text-green-700 dark:bg-green-950/40 dark:text-green-400",
          )}
        >
          {isOpponent ? "THEIR TURN" : "YOUR TURN"}
        </span>
      )}
      {isOpponent && isPriorityPlayer && (
        <span className="text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 bg-purple-100 text-purple-700 dark:bg-purple-950/40 dark:text-purple-300 animate-pulse">
          PRIORITY
        </span>
      )}
      {isTargetable && (
        <Badge
          variant="destructive"
          className="text-xs h-5 px-1 animate-pulse shrink-0"
        >
          TARGET
        </Badge>
      )}
      {player.poison > 0 && (
        <Badge variant="destructive" className="text-xs h-5 px-1 shrink-0">
          {player.poison} ☠
        </Badge>
      )}
      {totalCmdDmg > 0 && (
        <Badge
          variant="outline"
          className="text-xs h-5 px-1 text-orange-600 border-orange-400 shrink-0"
          title={`Commander damage received: ${totalCmdDmg}`}
        >
          ⚔{totalCmdDmg} CMD
        </Badge>
      )}
      {commandZoneCount > 0 && (
        <button
          className={cn(
            "inline-flex items-center gap-1 px-1.5 py-0.5 rounded border text-xs shrink-0",
            onOpenCommandZone
              ? "border-border text-muted-foreground hover:text-foreground hover:bg-muted/40"
              : "border-border text-muted-foreground/80",
          )}
          onClick={onOpenCommandZone}
          disabled={!onOpenCommandZone}
          title="Command Zone"
        >
          <Sword className="h-3 w-3" />
          <span>{commandZoneCount}</span>
        </button>
      )}
      <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
        <Layers className="h-3 w-3" />
        <span>{player.handCount}</span>
      </div>
      {!isOpponent && (
        <div className="ml-auto">
          <ManaPool pool={player.manaPool} />
        </div>
      )}
    </div>
  );
}

function BattlefieldZone({
  cards,
  label,
  emptyLabel,
  className,
  zoneBg,
  minHeight = 100,
  onClickCard,
  onClickAnyCard,
  onHoverCard,
  onFlipCard,
  showBackFace,
  pendingCardIds,
  attackingCardIds,
  tappableLandIds,
  onTapLand,
  untappableLandIds,
  onUntapLand,
  leftReserved = 0,
  rightReserved = 0,
}: {
  cards: XMageCard[];
  label: string;
  emptyLabel: string;
  className?: string;
  /** Override the cards area background/border classes (default: bg-muted/20) */
  zoneBg?: string;
  /** Minimum height of the cards area in px (default: 100) */
  minHeight?: number;
  /** Called when clicking a card with isChoosable=true */
  onClickCard?: (card: XMageCard) => void;
  /** Called when clicking any card (used for assigning attackers during blocking) */
  onClickAnyCard?: (card: XMageCard) => void;
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
  /** Cards highlighted as selected/pending (orange ring) */
  pendingCardIds?: string[];
  /** Cards highlighted as currently attacking (red ring) */
  attackingCardIds?: string[];
  /** Untapped lands the player can click to tap for mana (gold ring) */
  tappableLandIds?: string[];
  onTapLand?: (card: XMageCard) => void;
  /** Tapped lands whose mana is still in the pool (can be untapped) */
  untappableLandIds?: string[];
  onUntapLand?: (card: XMageCard) => void;
  /** Reserve horizontal space inside the battlefield canvas for overlays. */
  leftReserved?: number;
  rightReserved?: number;
}) {
  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);

  return (
    <div className={cn("flex flex-col gap-1 min-h-0", className)}>
      {label && (
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">
          {label}
        </span>
      )}
      <div
        className={cn(
          "flex flex-wrap gap-2 p-2 border rounded-lg flex-1 content-start",
          zoneBg ?? "bg-muted/20",
        )}
        style={{
          minHeight: `${minHeight}px`,
          paddingLeft: `${8 + leftReserved}px`,
          paddingRight: `${8 + rightReserved}px`,
        }}
      >
        {cards.length === 0 ? (
          <span className="text-xs text-muted-foreground italic self-center mx-auto">
            {emptyLabel}
          </span>
        ) : (
          cards.map((card) => {
            const isPending = pendingCardIds?.includes(card.id);
            const isAttacking = attackingCardIds?.includes(card.id);
            const isTappable = tappableLandIds?.includes(card.id);
            const isUntappable = untappableLandIds?.includes(card.id);
            const isChoosableClick =
              (card.isChoosable && !!onClickCard) ||
              (isAttacking && !!onClickAnyCard);
            return (
              <div
                key={card.id}
                data-card-id={card.id}
                className="relative group shrink-0 p-px"
                onMouseEnter={(e) => {
                  setHoveredCardId(card.id);
                  onHoverCard?.(card, e);
                }}
                onMouseLeave={() => {
                  setHoveredCardId(null);
                  onHoverCard?.(null);
                }}
              >
                <Card
                  card={card}
                  isTapped={card.tapped}
                  isHovered={hoveredCardId === card.id}
                  onFlip={onFlipCard}
                  showBackFace={showBackFace}
                  className={cn(
                    "w-[70px] h-[98px] shrink-0 hover:z-10",
                    card.isChoosable &&
                      onClickCard &&
                      "ring-2 ring-blue-400 cursor-pointer",
                    isPending && "ring-2 ring-orange-400 cursor-pointer",
                    isAttacking && "ring-2 ring-red-500 cursor-pointer",
                    isTappable &&
                      !isAttacking &&
                      "ring-2 ring-yellow-400 cursor-pointer",
                    isUntappable &&
                      !isAttacking &&
                      !isTappable &&
                      "ring-2 ring-cyan-400 cursor-pointer",
                  )}
                />
                {/* Tap-for-mana overlay — shown only during chooseAction */}
                {isTappable && onTapLand && (
                  <CardOverlayButton
                    variant="tap"
                    label="TAP"
                    onClick={() => onTapLand(card)}
                    title={`Tap ${card.name} for mana`}
                  />
                )}
                {/* Untap overlay — shown for tapped lands with unspent mana */}
                {isUntappable && onUntapLand && (
                  <CardOverlayButton
                    variant="untap"
                    label="UNTAP"
                    onClick={() => onUntapLand(card)}
                    title={`Untap ${card.name} (undo mana)`}
                  />
                )}
                {/* Choosable / attacker overlay */}
                {!isTappable && isChoosableClick && (
                  <CardOverlayButton
                    variant={isPending ? "pending" : isAttacking ? "attacking" : "choosable"}
                    onClick={() => {
                      if (card.isChoosable && onClickCard) onClickCard(card);
                      else if (isAttacking && onClickAnyCard)
                        onClickAnyCard(card);
                    }}
                    title={
                      isPending
                        ? `Deselect ${card.name}`
                        : isAttacking
                          ? `Block ${card.name}`
                          : `Select ${card.name}`
                    }
                  />
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

function HandDisplay({
  cards,
  onHoverCard,
  onStartDrag,
  onFlipCard,
  showBackFace,
}: {
  cards: XMageCard[];
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent) => void;
  onStartDrag?: (card: XMageCard, e: React.MouseEvent) => void;
  onFlipCard?: () => void;
  showBackFace?: boolean;
}) {
  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-1 shrink-0">
      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">
        Hand ({cards.length})
      </span>
      <div className="overflow-x-auto">
        <div className="flex gap-2 pb-2 px-1 min-h-[120px] items-end">
          {cards.map((card) => (
            <div
              key={card.id}
              className={cn(
                "relative group shrink-0",
                card.isPlayable && "cursor-grab",
              )}
              onMouseDown={
                card.isPlayable
                  ? (e) => { e.preventDefault(); onStartDrag?.(card, e); }
                  : undefined
              }
              onMouseEnter={(e) => {
                setHoveredCardId(card.id);
                onHoverCard?.(card, e);
              }}
              onMouseLeave={() => {
                setHoveredCardId(null);
                onHoverCard?.(null);
              }}
            >
              <Card
                card={card}
                className={cn(
                  "w-[80px] h-[112px] transition-transform group-hover:-translate-y-3",
                  !card.isPlayable && "opacity-60 grayscale",
                )}
                isHovered={hoveredCardId === card.id}
                onFlip={onFlipCard}
                showBackFace={showBackFace}
              />
              {card.isPlayable && (
                <div
                  className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 bg-primary/20 border-2 border-primary transition-opacity pointer-events-none"
                  title={`Play ${card.name}`}
                />
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

interface OpponentHalfProps {
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
  promptType: string | undefined;
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

function OpponentHalf({
  player,
  permanents,
  graveyard,
  exile,
  commandZone,
  isTargetable,
  onTarget,
  isFlashing,
  activePlayerId,
  priorityPlayerId,
  promptType,
  pendingAttacker,
  attackerIds,
  onClickCard,
  onClickAnyCard,
  onHoverCard,
  onFlipCard,
  showBackFace,
  onOpenZone,
  zonePanelSide,
  zonePanelOrder,
}: OpponentHalfProps) {
  return (
    <div className="flex flex-col gap-1 h-full overflow-hidden">
      <PlayerPanel
        player={player}
        isOpponent
        isActiveTurn={activePlayerId === player.id}
        isPriorityPlayer={priorityPlayerId === player.id}
        isTargetable={isTargetable}
        onTarget={onTarget}
        isFlashing={isFlashing}
        onOpenCommandZone={
          (commandZone?.length ?? 0) > 0
            ? () => onOpenZone(`${player.name}'s Command Zone`, commandZone!)
            : undefined
        }
        commandZoneCount={commandZone?.length ?? 0}
      />
      <div className="flex gap-2 flex-1 min-h-0 overflow-hidden">
        <div className="relative flex flex-col gap-1 flex-1 min-w-0 overflow-hidden">
          <div
            className={cn(
              "absolute bottom-1 z-20",
              zonePanelSide === "left" ? "left-1" : "right-1",
            )}
          >
            <ZoneActionColumn
              libraryCount={player.libraryCount}
              graveyardCount={graveyard.length}
              exileCount={exile.length}
              order={zonePanelOrder}
              onOpenGraveyard={() => onOpenZone(`${player.name}'s Graveyard`, graveyard)}
              onOpenExile={() => onOpenZone(`${player.name}'s Exile`, exile)}
            />
          </div>
          <BattlefieldZone
            cards={permanents}
            label=""
            emptyLabel="No permanents"
            onFlipCard={onFlipCard}
            showBackFace={showBackFace}
            className="flex-1"
            minHeight={60}
            leftReserved={zonePanelSide === "left" ? ZONE_COLUMN_RESERVED_PX : 0}
            rightReserved={zonePanelSide === "right" ? ZONE_COLUMN_RESERVED_PX : 0}
            onClickCard={
              promptType === "chooseTargetCard" ||
              promptType === "chooseTargetAny"
                ? onClickCard
                : undefined
            }
            onClickAnyCard={
              promptType === "chooseBlockers" ? onClickAnyCard : undefined
            }
            onHoverCard={onHoverCard}
            pendingCardIds={
              promptType === "chooseBlockers" && pendingAttacker
                ? [pendingAttacker]
                : undefined
            }
            attackingCardIds={
              promptType === "chooseBlockers" ? (attackerIds ?? []) : undefined
            }
          />
        </div>
      </div>
    </div>
  );
}

function MidPhaseStrip({ currentStep }: { currentStep: string }) {
  return (
    <div className="pointer-events-none flex items-center justify-center gap-1 px-2 py-0.5 overflow-x-auto max-w-full">
      {PHASES.map((phase) => (
        <span
          key={phase.id}
          className={cn(
            "text-[10px] px-1.5 py-0.5 rounded border leading-none shrink-0",
            currentStep === phase.id
              ? "bg-primary text-primary-foreground border-primary font-semibold"
              : "bg-background/90 border-border text-muted-foreground",
          )}
          title={phase.label}
        >
          {phase.short}
        </span>
      ))}
    </div>
  );
}

export default function Game() {
  const {
    gameView,
    currentPrompt,
    isGameActive,
    isWaitingForResponse,
    gameLog,
    debugInfo,
    passPriority,
    castSpell,
    declareAttackers,
    declareBlockers,
    targetPlayer,
    targetCard,
    targetAny,
    mulliganDecision,
    tapLand,
    untapLand,
    scryDecision,
    surveilDecision,
    digDecision,
    discardDecision,
    targetSpell,
    modeDecision,
    optionalTriggerDecision,
    colorDecision,
    chooseCardsDecision,
    respond,
    concede,
    endGame,
    setupListeners,
    deferredQueue,
  } = useGameStore();
  const flashDurationMs = usePreferencesStore((s) => s.flashDurationMs);
  const zonePanelSide = usePreferencesStore((s) => s.zonePanelSide);
  const zonePanelOrder = usePreferencesStore((s) => s.zonePanelOrder);
  const location = useLocation();
  const devExtraOpponents = ((location.state as { devExtraOpponents?: number } | null)?.devExtraOpponents ?? 0);
  const containerRef = useRef<HTMLDivElement>(null);

  const [hoveredCard, setHoveredCard] = useState<XMageCard | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [showBackFace, setShowBackFace] = useState(false);

  // Hand drag-to-play state
  const [draggingHandCard, setDraggingHandCard] = useState<XMageCard | null>(null);
  const [ghostPos, setGhostPos] = useState({ x: 0, y: 0 });
  const [isOverBattlefield, setIsOverBattlefield] = useState(false);
  const battlefieldContainerRef = useRef<HTMLDivElement>(null);
  const isOverBattlefieldRef = useRef(false);

  // Display flash queue — sequential visual-only flashes.
  type FlashItem =
    | { kind: "card"; cardId: string; cardName: string; setCode: string }
    | { kind: "turn"; playerId: string; playerName: string };
  const [activeFlash, setActiveFlash] = useState<FlashItem | null>(null);
  const flashQueueRef = useRef<FlashItem[]>([]);
  const isFlashingRef = useRef(false);
  // Hold the deferred gameView + prompt until all flashes for the current snapshot finish.
  const deferredStateRef = useRef<{ gameView: unknown; prompt: unknown } | null>(null);


  // Combat state
  const [pendingAttackers, setPendingAttackers] = useState<string[]>([]);
  /** The attacker card ID the player has selected to assign a blocker to (attacker-first flow) */
  const [pendingAttacker, setPendingAttacker] = useState<string | null>(null);
  const [blockAssignments, setBlockAssignments] = useState<
    { blockerId: string; attackerId: string }[]
  >([]);

  // Zone viewer
  const [viewingZone, setViewingZone] = useState<{
    title: string;
    cards: XMageCard[];
    onClickCard?: (cardId: string) => void;
  } | null>(null);
  function openZone(title: string, cards: XMageCard[], onClickCard?: (cardId: string) => void) {
    setViewingZone({ title, cards, onClickCard });
  }
  function closeZone() {
    setViewingZone(null);
  }

  // Zone target selector (for selecting cards from graveyard, exile, etc.)
  const [zoneTargetSelector, setZoneTargetSelector] = useState<{
    title: string;
    cards: XMageCard[];
    validCardIds: string[];
  } | null>(null);
  function openZoneTargetSelector(title: string, cards: XMageCard[], validCardIds: string[]) {
    setZoneTargetSelector({ title, cards, validCardIds });
  }
  function closeZoneTargetSelector() {
    setZoneTargetSelector(null);
  }

  // Library peek modal (Scry / Surveil / Dig)
  const [libraryPeekModal, setLibraryPeekModal] = useState<{
    mode: LibraryPeekMode;
    cards: XMageCard[];
    numToTake?: number;
    optional?: boolean;
  } | null>(null);

  // Spell stack modal (view stack / choose counter target)
  const [spellStackModalOpen, setSpellStackModalOpen] = useState(false);

  // Right-side prompt/action panel collapse state
  const [isActionPanelCollapsed, setIsActionPanelCollapsed] = useState(false);

  const promptType = currentPrompt?.type;

  /** Cancel any pending hover timer and clear the visible preview. */
  function dismissHover() {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    setHoveredCard(null);
  }

  function handleFlipCard() {
    setShowBackFace(prev => !prev);
  }

  function handleHoverCard(card: XMageCard | null, e?: React.MouseEvent) {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    if (!card) {
      setHoveredCard(null);
      setShowBackFace(false);
      return;
    }
    if (e) setMousePos({ x: e.clientX, y: e.clientY });
    hoverTimerRef.current = setTimeout(() => {
      setHoveredCard(card);
      setShowBackFace(false); // Reset to front face when hovering new card
      hoverTimerRef.current = null;
    }, 500);
  }

  // Set up event listeners on mount
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    setupListeners().then((fn) => {
      cleanup = fn;
    });
    return () => {
      cleanup?.();
    };
  }, [setupListeners]);


  // Keyboard shortcuts — passPriority already checks isWaitingForResponse internally
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      )
        return;
      if (e.code === "Space") {
        e.preventDefault();
        passPriority();
      }
      if (e.code === "F6") {
        e.preventDefault();
        passPriority();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [passPriority]);

  // Auto-pass: respond automatically when no legal actions available
  const autoPassEnabled = usePreferencesStore((s) => s.autoPassEnabled);
  const [isAutoPassing, setIsAutoPassing] = useState(false);

  useEffect(() => {
    setIsAutoPassing(false);

    if (!currentPrompt || isWaitingForResponse) return;
    if (!autoPassEnabled) return;

    let shouldAutoPass = false;

    if (currentPrompt.type === "chooseAction") {
      const ids = currentPrompt.playableCardIds ?? [];
      shouldAutoPass = ids.length === 0;
    } else if (currentPrompt.type === "chooseAttackers") {
      const ids = currentPrompt.availableAttackerIds ?? [];
      shouldAutoPass = ids.length === 0;
    } else if (currentPrompt.type === "chooseBlockers") {
      const ids = currentPrompt.availableBlockerIds ?? [];
      shouldAutoPass = ids.length === 0;
    }

    if (!shouldAutoPass) return;

    setIsAutoPassing(true);
    const delay = 300 + Math.floor(Math.random() * 500); // 300-800ms
    const timer = setTimeout(() => {
      passPriority();
    }, delay);

    return () => clearTimeout(timer);
  }, [currentPrompt, isWaitingForResponse, autoPassEnabled, passPriority]);

  // Reset combat state whenever the prompt type changes
  useEffect(() => {
    setPendingAttackers([]);
    setPendingAttacker(null);
    setBlockAssignments([]);
  }, [currentPrompt?.type]);

  // Dismiss card hover preview whenever any modal opens or the active prompt changes.
  // This prevents a stale preview from lingering behind a modal or after closing one.
  useEffect(() => {
    dismissHover();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [viewingZone, zoneTargetSelector, libraryPeekModal, spellStackModalOpen, currentPrompt?.type]);

  // Apply deferred gameView + prompt from the ref into the store.
  function applyDeferredState() {
    const deferred = deferredStateRef.current;
    if (!deferred) return;
    deferredStateRef.current = null;
    const updates: Record<string, unknown> = {
      gameView: deferred.gameView,
      isWaitingForResponse: false,
      currentPrompt: deferred.prompt ?? null,
    };
    useGameStore.setState(updates);
  }

  // Pop the next snapshot from the queue and start its flashes.
  function startNextSnapshot() {
    const queue = useGameStore.getState().deferredQueue;
    if (queue.length === 0) {
      isFlashingRef.current = false;
      useGameStore.setState({ isFlashing: false });
      return;
    }

    const [snapshot, ...rest] = queue;
    useGameStore.setState({ deferredQueue: rest });

    if (snapshot.displayEvents.length === 0) {
      // No flashes — apply state immediately and continue to next in queue.
      const updates: Record<string, unknown> = {
        gameView: snapshot.gameView,
        isWaitingForResponse: false,
        currentPrompt: snapshot.prompt ?? null,
      };
      useGameStore.setState(updates);
      if (rest.length > 0) {
        setTimeout(startNextSnapshot, 0);
      } else {
        isFlashingRef.current = false;
        useGameStore.setState({ isFlashing: false });
      }
      return;
    }

    // Defer the gameView + prompt — will be applied after all flashes finish.
    deferredStateRef.current = { gameView: snapshot.gameView, prompt: snapshot.prompt };

    // Enqueue flash items for this snapshot's display events.
    for (const evt of snapshot.displayEvents) {
      if (evt.kind === "cardPlayed") {
        flashQueueRef.current.push({
          kind: "card",
          cardId: evt.cardId!,
          cardName: evt.cardName!,
          setCode: evt.setCode ?? "",
        });
      } else if (evt.kind === "turnChanged") {
        flashQueueRef.current.push({
          kind: "turn",
          playerId: evt.activePlayerId!,
          playerName: evt.activePlayerName!,
        });
      }
    }

    // Kick off the first flash
    const first = flashQueueRef.current.shift();
    if (first) {
      isFlashingRef.current = true;
      useGameStore.setState({ isFlashing: true });
      setActiveFlash(first);
    }
  }

  // Watch the deferred queue — when entries arrive and we're idle, start processing.
  useEffect(() => {
    if (deferredQueue.length > 0 && !isFlashingRef.current) {
      startNextSnapshot();
    }
  }, [deferredQueue]);

  // Process flash queue: when current flash ends, show next or apply deferred state.
  useEffect(() => {
    if (!activeFlash) {
      // Check if there are more flashes in the current snapshot's batch
      const next = flashQueueRef.current.shift();
      if (next) {
        isFlashingRef.current = true;
        setActiveFlash(next);
      } else {
        // All flashes done — now apply the deferred gameView + prompt.
        applyDeferredState();
        const queue = useGameStore.getState().deferredQueue;
        if (queue.length > 0) {
          setTimeout(startNextSnapshot, 10);
        } else {
          isFlashingRef.current = false;
          useGameStore.setState({ isFlashing: false });
        }
      }
      return;
    }
    const timer = setTimeout(() => {
      setActiveFlash(null);
    }, flashDurationMs);
    return () => clearTimeout(timer);
  }, [activeFlash, flashDurationMs]);

  // Targeting / combat arrows — must be called unconditionally (Rules of Hooks)
  // Player IDs are empty strings when gameView is not yet available; the hook
  // will safely produce no arrows in that case.
  const me = gameView?.players.find((p) => p.isHuman) ?? gameView?.players[0];
  const opponents = gameView?.players.filter((p) => !p.isHuman) ?? [];
  const opponent = opponents[0]; // alias for arrows hook + game-over screen
  // DEV: pad with simulated opponents to test multi-player layout
  const displayOpponents = [
    ...opponents,
    ...Array.from({ length: devExtraOpponents }, (_, i) => ({
      id: `dev-fake-${i}`,
      name: `Dev Opp ${opponents.length + i + 1}`,
      isHuman: false,
      life: 20,
      poison: 0,
      handCount: 7,
      libraryCount: 40,
      graveyardCount: 0,
      exileCount: 0,
      manaPool: {} as Record<string, number>,
    } as Player)),
  ];
  // Stabilize attackerIds so useGameArrows' useEffect doesn't re-run every render
  // when the prompt has no attackerIds (the ?? [] fallback would create a new array each time).
  const attackerIds = useMemo(
    () => currentPrompt?.attackerIds ?? [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [currentPrompt?.attackerIds?.join(",")],
  );
  const combatAssignments = useMemo(
    () => gameView?.combatAssignments ?? [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [gameView?.combatAssignments?.map((a) => `${a.blockerId}:${a.attackerId}`).join(",")],
  );

  const arrows = useGameArrows({
    containerRef,
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId: me?.id ?? "",
    opponentPlayerId: opponent?.id ?? "",
  });

  // Auto-return to play menu when game is over
  useEffect(() => {
    if (!gameView?.gameOver && currentPrompt?.type !== "gameOver") return;
    const timer = setTimeout(() => endGame(), 3000);
    return () => clearTimeout(timer);
  }, [gameView?.gameOver, currentPrompt?.type]);

  // Open library-peek modal for Scry / Surveil / Dig / Discard prompts
  useEffect(() => {
    if (
      (promptType === "scry" || promptType === "surveil" || promptType === "dig") &&
      currentPrompt?.cards &&
      currentPrompt.cards.length > 0
    ) {
      setLibraryPeekModal({
        mode: promptType as LibraryPeekMode,
        cards: currentPrompt.cards as XMageCard[],
        numToTake: promptType === "dig" ? currentPrompt.numToTake : undefined,
        optional: promptType === "dig" ? currentPrompt.optional : undefined,
      });
    } else if (promptType === "chooseDiscard" && currentPrompt) {
      // Build card objects from hand card IDs — hand is visible in gameView.myHand
      const handCards = (currentPrompt.handCardIds ?? [])
        .map((id) => gameView?.myHand.find((c) => c.id === id))
        .filter((c): c is XMageCard => c !== undefined);
      if (handCards.length > 0) {
        setLibraryPeekModal({
          mode: "discard",
          cards: handCards,
          numToTake: currentPrompt.numToDiscard,
        });
      }
    } else if (
      promptType !== "scry" &&
      promptType !== "surveil" &&
      promptType !== "dig" &&
      promptType !== "chooseDiscard"
    ) {
      setLibraryPeekModal(null);
    }
  }, [promptType, currentPrompt, gameView?.myHand]);

  // Handle zone-based targeting prompts (e.g., selecting from graveyard)
  useEffect(() => {
    if (promptType === "chooseTargetCardFromZone" && currentPrompt) {
      const zone = currentPrompt.zone;
      const validCardIds = currentPrompt.validCardIds || [];
      const zoneCards = currentPrompt.zoneCards || [];

      if (zone && zoneCards.length > 0) {
        const zoneNames: Record<string, string> = {
          Graveyard: "Graveyard",
          Exile: "Exile",
          Hand: "Hand",
        };
        const zoneName = zoneNames[zone] || zone;
        const title = `Choose from ${zoneName}`;

        openZoneTargetSelector(title, zoneCards, validCardIds);
      }
    } else {
      // Close the zone selector if prompt type changes
      closeZoneTargetSelector();
    }
  }, [promptType, currentPrompt]);

  // Auto-open spell-stack modal for counter-targeting prompts
  useEffect(() => {
    setSpellStackModalOpen(promptType === "chooseTargetSpell");
  }, [promptType]);

  if (!isGameActive) return <Navigate to="/lobby" replace />;

  // Loading
  if (!gameView) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <p className="text-muted-foreground">Waiting for game state...</p>
        {debugInfo && (
          <p className="text-xs text-muted-foreground font-mono">{debugInfo}</p>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={async () => {
            try {
              const raw = await invoke<any>("get_prompt");
              useGameStore.setState({
                debugInfo: `Manual poll: ${JSON.stringify(raw)?.slice(0, 200)}`,
              });
            } catch (e) {
              useGameStore.setState({ debugInfo: `Poll error: ${e}` });
            }
          }}
        >
          Debug: Poll Now
        </Button>
      </div>
    );
  }

  // me / opponent are already derived above (before the early returns).
  // Re-assert non-null: if we reach here, gameView is defined.
  const myPermanents = gameView.battlefield.filter(
    (c) => c.controllerId === me!.id,
  );
  const opponentPermanentsByPlayer = new Map(
    opponents.map((op) => [
      op.id,
      gameView.battlefield.filter((c) => c.controllerId === op.id),
    ]),
  );
  const battlefieldCardNameById = new Map(
    gameView.battlefield.map((card) => [card.id, card.name] as const),
  );

  // Game over overlay
  if (gameView.gameOver || promptType === "gameOver") {
    const winnerId = gameView.winnerId;
    const didWin = winnerId === me!.id;
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <h2
          className={cn(
            "text-3xl font-bold",
            didWin ? "text-green-600" : "text-red-600",
          )}
        >
          {didWin ? "You Win!" : "You Lose!"}
        </h2>
        <p className="text-muted-foreground">
          Final life: You {me!.life} — {opponents.map((op) => `${op.name} ${op.life}`).join(" · ")}
        </p>
        <p className="text-sm text-muted-foreground">Turn {gameView.turn}</p>
        <p className="text-xs text-muted-foreground italic">
          Returning to menu…
        </p>
        <Button variant="outline" size="sm" onClick={() => endGame()}>
          Return to Menu
        </Button>
      </div>
    );
  }

  const playerIsTargetable =
    promptType === "chooseTargetPlayer" || promptType === "chooseTargetAny"
      ? (pid: string) => currentPrompt?.validPlayerIds?.includes(pid) ?? false
      : () => false;

  function handleTargetPlayer(pid: string) {
    if (promptType === "chooseTargetAny") {
      targetAny({ kind: "player", playerId: pid });
    } else {
      targetPlayer(pid);
    }
  }

  function handleBattlefieldClick(card: XMageCard) {
    if (!currentPrompt || !card.isChoosable) return;

    if (promptType === "chooseAttackers") {
      // Toggle this creature in/out of the pending attacker set
      setPendingAttackers((prev) =>
        prev.includes(card.id)
          ? prev.filter((id) => id !== card.id)
          : [...prev, card.id],
      );
    } else if (promptType === "chooseBlockers") {
      // Attacker-first flow: if an attacker is already selected, assign this creature as its blocker
      if (pendingAttacker) {
        setBlockAssignments((prev) => {
          const rest = prev.filter((a) => a.attackerId !== pendingAttacker);
          return [...rest, { blockerId: card.id, attackerId: pendingAttacker }];
        });
        setPendingAttacker(null);
      }
    } else if (promptType === "chooseTargetCard") {
      targetCard(card.id);
    } else if (promptType === "chooseTargetAny") {
      targetAny({ kind: "card", cardId: card.id });
    }
  }

  /** Called when clicking one of the opponent's attacking creatures during block assignment (attacker-first flow) */
  function handleAttackerClick(card: XMageCard) {
    // Toggle selection: clicking the same attacker deselects it
    setPendingAttacker((prev) => (prev === card.id ? null : card.id));
  }

  /** Initiate a drag from the hand; on drop over the battlefield, plays the card */
  function startHandCardDrag(card: XMageCard, e: React.MouseEvent) {
    if (!card.isPlayable) return;
    // Cancel any pending hover timer but keep the preview visible until movement begins
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    setDraggingHandCard(card);
    setGhostPos({ x: e.clientX, y: e.clientY });

    let moved = false;
    const handleMouseMove = (me: MouseEvent) => {
      if (!moved) setHoveredCard(null); // dismiss preview on first movement
      moved = true;
      setGhostPos({ x: me.clientX, y: me.clientY });
      if (battlefieldContainerRef.current) {
        const rect = battlefieldContainerRef.current.getBoundingClientRect();
        const over =
          me.clientX >= rect.left &&
          me.clientX <= rect.right &&
          me.clientY >= rect.top &&
          me.clientY <= rect.bottom;
        isOverBattlefieldRef.current = over;
        setIsOverBattlefield(over);
      }
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);

      if (!moved) {
        castSpell(card.id);
      } else if (isOverBattlefieldRef.current) {
        castSpell(card.id);
      }

      setDraggingHandCard(null);
      setIsOverBattlefield(false);
      isOverBattlefieldRef.current = false;
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }

  const turnFlashPlayerId =
    activeFlash?.kind === "turn" ? activeFlash.playerId : null;

  return (
    <div
      ref={containerRef}
      className="relative flex flex-col h-full gap-1.5 p-1.5 overflow-hidden"
      style={
        { "--flash-duration": `${flashDurationMs}ms` } as React.CSSProperties
      }
    >
      <ArrowOverlay arrows={arrows} />

      <div className="flex gap-1 min-h-0 flex-1 overflow-hidden">
        <div className="flex flex-col gap-1 min-h-0 flex-1 overflow-hidden">
          {/* ── Resizable split: opponent (top) / me (bottom) ─── */}
          <ResizablePanelGroup orientation="vertical" className="flex-1 min-h-0">
            <ResizablePanel defaultSize={45} minSize={20}>
              {displayOpponents.length <= 1 ? (
                <OpponentHalf
                  player={displayOpponents[0]!}
                  permanents={opponentPermanentsByPlayer.get(displayOpponents[0]!.id) ?? []}
                  graveyard={gameView.opponentGraveyard ?? []}
                  exile={gameView.opponentExile ?? []}
                  commandZone={gameView.opponentCommandZone ?? undefined}
                  isTargetable={playerIsTargetable(displayOpponents[0]!.id)}
                  onTarget={() => handleTargetPlayer(displayOpponents[0]!.id)}
                  isFlashing={turnFlashPlayerId === displayOpponents[0]?.id}
                  activePlayerId={gameView.activePlayerId}
                  priorityPlayerId={gameView.priorityPlayerId}
                  promptType={promptType}
                  pendingAttacker={pendingAttacker}
                  attackerIds={currentPrompt?.attackerIds}
                  onClickCard={handleBattlefieldClick}
                  onClickAnyCard={handleAttackerClick}
                  onHoverCard={handleHoverCard}
                  onFlipCard={handleFlipCard}
                  showBackFace={showBackFace}
                  onOpenZone={openZone}
                  zonePanelSide={zonePanelSide}
                  zonePanelOrder={zonePanelOrder}
                />
              ) : (
                <ResizablePanelGroup orientation="horizontal">
                  {displayOpponents.map((op, i) => (
                    <Fragment key={op.id}>
                      {i > 0 && <ResizableHandle />}
                      <ResizablePanel>
                        <OpponentHalf
                          player={op}
                          permanents={opponentPermanentsByPlayer.get(op.id) ?? []}
                          graveyard={i === 0 ? (gameView.opponentGraveyard ?? []) : []}
                          exile={i === 0 ? (gameView.opponentExile ?? []) : []}
                          commandZone={i === 0 ? (gameView.opponentCommandZone ?? undefined) : undefined}
                          isTargetable={playerIsTargetable(op.id)}
                          onTarget={() => handleTargetPlayer(op.id)}
                          isFlashing={turnFlashPlayerId === op.id}
                          activePlayerId={gameView.activePlayerId}
                          priorityPlayerId={gameView.priorityPlayerId}
                          promptType={promptType}
                          pendingAttacker={pendingAttacker}
                          attackerIds={currentPrompt?.attackerIds}
                          onClickCard={handleBattlefieldClick}
                          onClickAnyCard={handleAttackerClick}
                          onHoverCard={handleHoverCard}
                          onFlipCard={handleFlipCard}
                          showBackFace={showBackFace}
                          onOpenZone={openZone}
                          zonePanelSide={zonePanelSide}
                          zonePanelOrder={zonePanelOrder}
                        />
                      </ResizablePanel>
                    </Fragment>
                  ))}
                </ResizablePanelGroup>
              )}
            </ResizablePanel>

            <ResizableHandle
              withHandle={false}
              gripOnly
              className="h-8 w-full my-0 flex items-center justify-center overflow-visible"
            >
              <MidPhaseStrip currentStep={gameView.step} />
            </ResizableHandle>

            <ResizablePanel defaultSize={60} minSize={35}>
              <div className="flex flex-col gap-1 h-full overflow-hidden">
                <div className="flex gap-2 flex-1 min-h-0 overflow-hidden">
                  <div
                    ref={battlefieldContainerRef}
                    className="relative flex flex-col flex-1 min-w-0 overflow-hidden"
                  >
                    <div
                      className={cn(
                        "absolute bottom-1 z-20",
                        zonePanelSide === "left" ? "left-1" : "right-1",
                      )}
                    >
                      <ZoneActionColumn
                        libraryCount={me!.libraryCount}
                        graveyardCount={gameView.graveyard.length}
                        exileCount={gameView.exile.length}
                        order={zonePanelOrder}
                        onOpenGraveyard={() => {
                          const hasPlayable = gameView.graveyard.some((c) => c.isPlayable);
                          openZone(
                            "Your Graveyard",
                            gameView.graveyard,
                            hasPlayable && promptType === "chooseAction"
                              ? (cardId) => {
                                  closeZone();
                                  castSpell(cardId);
                                }
                              : undefined,
                          );
                        }}
                        onOpenExile={() => {
                          const hasPlayable = gameView.exile.some((c) => c.isPlayable);
                          openZone(
                            "Your Exile",
                            gameView.exile,
                            hasPlayable && promptType === "chooseAction"
                              ? (cardId) => {
                                  closeZone();
                                  castSpell(cardId);
                                }
                              : undefined,
                          );
                        }}
                        hasPlayableInGraveyard={promptType === "chooseAction" && gameView.graveyard.some((c) => c.isPlayable)}
                        hasPlayableInExile={promptType === "chooseAction" && gameView.exile.some((c) => c.isPlayable)}
                      />
                    </div>
                    <FreeBattlefield
                      cards={myPermanents}
                      className="flex-1"
                      onClickCard={
                        promptType === "chooseAttackers" ||
                        promptType === "chooseBlockers" ||
                        promptType === "chooseTargetCard" ||
                        promptType === "chooseTargetAny"
                          ? handleBattlefieldClick
                          : undefined
                      }
                      onHoverCard={handleHoverCard}
                      onFlipCard={handleFlipCard}
                      showBackFace={showBackFace}
                      pendingCardIds={
                        promptType === "chooseAttackers"
                          ? pendingAttackers
                          : promptType === "chooseBlockers"
                            ? blockAssignments.map((a) => a.blockerId)
                            : undefined
                      }
                      tappableLandIds={
                        promptType === "chooseAction"
                          ? (currentPrompt?.tappableLandIds ?? [])
                          : undefined
                      }
                      onTapLand={
                        promptType === "chooseAction"
                          ? (card) => tapLand(card.id)
                          : undefined
                      }
                      untappableLandIds={
                        promptType === "chooseAction"
                          ? (currentPrompt?.untappableLandIds ?? [])
                          : undefined
                      }
                      onUntapLand={
                        promptType === "chooseAction"
                          ? (card) => untapLand(card.id)
                          : undefined
                      }
                      bottomReserved={130}
                      leftReserved={zonePanelSide === "left" ? ZONE_COLUMN_RESERVED_PX : 0}
                      rightReserved={zonePanelSide === "right" ? ZONE_COLUMN_RESERVED_PX : 0}
                      isDropActive={isOverBattlefield}
                    />
                    <div className="absolute bottom-0 left-1/2 -translate-x-1/2 z-20 w-max max-w-full">
                      <HandDisplay
                        cards={gameView.myHand}
                        onHoverCard={handleHoverCard}
                        onFlipCard={handleFlipCard}
                        showBackFace={showBackFace}
                        onStartDrag={startHandCardDrag}
                      />
                    </div>
                  </div>
                </div>
              </div>
            </ResizablePanel>
          </ResizablePanelGroup>

          <div className="flex items-center gap-2 shrink-0">
            <div className="flex-1 min-w-0">
              <PlayerPanel
                player={me!}
                isOpponent={false}
                isActiveTurn={gameView.activePlayerId === me!.id}
                isPriorityPlayer={gameView.priorityPlayerId === me!.id}
                isTargetable={playerIsTargetable(me!.id)}
                onTarget={() => handleTargetPlayer(me!.id)}
                isFlashing={turnFlashPlayerId === me!.id}
                onOpenCommandZone={() => {
                  if ((gameView.myCommandZone?.length ?? 0) > 0) {
                    openZone("Your Command Zone", gameView.myCommandZone!);
                  }
                }}
                commandZoneCount={gameView.myCommandZone?.length ?? 0}
              />
            </div>
          </div>

        </div>

        <RightActionPanel
          collapsed={isActionPanelCollapsed}
          onToggleCollapse={() => setIsActionPanelCollapsed((prev) => !prev)}
          promptType={promptType}
          isWaitingForResponse={isWaitingForResponse}
          isAutoPassing={isAutoPassing}
          availableAttackerIds={currentPrompt?.availableAttackerIds ?? []}
          pendingAttackers={pendingAttackers}
          onPassPriority={passPriority}
          onDeclareAttackers={declareAttackers}
          pendingAttacker={pendingAttacker}
          attackerIds={currentPrompt?.attackerIds ?? []}
          blockAssignments={blockAssignments}
          onDeclareBlockers={declareBlockers}
          onMulliganDecision={mulliganDecision}
          stack={gameView.stack}
          onOpenStack={() => setSpellStackModalOpen(true)}
          onConcede={concede}
          resolveCardName={(cardId) => battlefieldCardNameById.get(cardId) ?? cardId}
          isMyPriority={gameView.priorityPlayerId === me!.id}
          turn={gameView.turn}
          activePlayerName={
            gameView.players.find((p) => p.id === gameView.activePlayerId)?.name ??
            "Unknown"
          }
          isMyTurn={gameView.activePlayerId === me!.id}
          gameLog={gameLog}
        />
      </div>

      {/* ── Zone viewer modal ─────────────────────────────── */}
      {viewingZone && (
        <ZoneViewer
          title={viewingZone.title}
          cards={viewingZone.cards}
          onClose={closeZone}
          onClickCard={viewingZone.onClickCard}
        />
      )}

      {/* ── Zone target selector modal (for graveyard/exile targeting) ─────────── */}
      {zoneTargetSelector && (
        <ZoneTargetSelector
          title={zoneTargetSelector.title}
          cards={zoneTargetSelector.cards}
          validCardIds={zoneTargetSelector.validCardIds}
          onSelect={(cardId) => {
            targetCard(cardId);
            closeZoneTargetSelector();
          }}
          onCancel={closeZoneTargetSelector}
        />
      )}

      {/* ── Library peek modal (Scry / Surveil / Dig) ────── */}
      {libraryPeekModal && (
        <LibraryPeekModal
          mode={libraryPeekModal.mode}
          cards={libraryPeekModal.cards}
          numToTake={libraryPeekModal.numToTake}
          optional={libraryPeekModal.optional}
          onConfirm={(selectedIds) => {
            if (libraryPeekModal.mode === "scry") {
              scryDecision(selectedIds);
            } else if (libraryPeekModal.mode === "surveil") {
              surveilDecision(selectedIds);
            } else if (libraryPeekModal.mode === "discard") {
              discardDecision(selectedIds);
            } else {
              digDecision(selectedIds);
            }
            setLibraryPeekModal(null);
          }}
        />
      )}

      {/* ── Spell stack modal (view stack / choose counter target) ─── */}
      {spellStackModalOpen && gameView.stack.length > 0 && (
        <SpellStackModal
          stack={gameView.stack}
          validSpellIds={
            promptType === "chooseTargetSpell"
              ? (currentPrompt?.validSpellIds ?? [])
              : []
          }
          onTarget={(spellId) => {
            targetSpell(spellId);
            setSpellStackModalOpen(false);
          }}
          onCancel={() => setSpellStackModalOpen(false)}
        />
      )}

      {/* ── Choose mode modal (SP$ Charm) ────────────────── */}
      {promptType === "chooseMode" && currentPrompt?.options && (
        <ChooseModeModal
          options={currentPrompt.options}
          minChoices={currentPrompt.minChoices ?? 1}
          maxChoices={currentPrompt.maxChoices ?? 1}
          cardName={currentPrompt.sourceCardName}
          onConfirm={(chosenIndices) => modeDecision(chosenIndices)}
        />
      )}

      {/* ── Optional trigger modal (OptionalDecider$) ──── */}
      {promptType === "chooseOptionalTrigger" && currentPrompt?.description != null && (
        <ChooseOptionalTriggerModal
          description={currentPrompt.description}
          cardName={currentPrompt.sourceCardName}
          onConfirm={(accept) => optionalTriggerDecision(accept)}
        />
      )}

      {/* ── Cost modals (kicker, buyback, multikicker, replicate, alt cost) ── */}
      {promptType === "chooseKicker" && currentPrompt?.kickerCost != null && (
        <KickerModal
          kickerCost={currentPrompt.kickerCost}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={(kicked) => respond({ type: "kickerDecision", kicked })}
        />
      )}

      {promptType === "chooseBuyback" && currentPrompt?.buybackCost != null && (
        <BuybackModal
          buybackCost={currentPrompt.buybackCost}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={(paid) => respond({ type: "buybackDecision", buybackPaid: paid })}
        />
      )}

      {promptType === "chooseMultikicker" && currentPrompt?.cost != null && (
        <MultikickerModal
          cost={currentPrompt.cost}
          maxKicks={currentPrompt.maxKicks ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={(kickCount) => respond({ type: "multikickerDecision", kickCount })}
        />
      )}

      {promptType === "chooseReplicate" && currentPrompt?.cost != null && (
        <ReplicateModal
          cost={currentPrompt.cost}
          maxReplicates={currentPrompt.maxReplicates ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={(replicateCount) => respond({ type: "replicateDecision", replicateCount })}
        />
      )}

      {promptType === "chooseAlternativeCost" && currentPrompt?.options != null && (
        <AlternativeCostModal
          options={currentPrompt.options as string[]}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={(chosenIndex) => respond({ type: "alternativeCostDecision", chosenIndex })}
        />
      )}

      {/* ── Choose Color modal (ChooseColorEffect) ──── */}
      {promptType === "chooseColor" && currentPrompt?.validColors != null && (
        <ChooseColorModal
          validColors={currentPrompt.validColors}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={(color) => colorDecision(color)}
        />
      )}

      {/* ── Choose Cards modal (ChooseCardEffect / CloneEffect) ──── */}
      {promptType === "chooseCardsForEffect" && currentPrompt?.zoneCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.zoneCards}
          minChoices={currentPrompt.minChoices ?? 1}
          maxChoices={currentPrompt.maxChoices ?? 1}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={(chosenCardIds) => chooseCardsDecision(chosenCardIds)}
        />
      )}

      {/* ── Card-play flash overlay ───────────────────────── */}
      {activeFlash?.kind === "card" &&
        createPortal(
          <div
            className="fixed inset-0 z-[10000] flex items-center justify-center pointer-events-none bg-black/30 animate-card-flash-backdrop"
            style={
              {
                "--flash-duration": `${flashDurationMs}ms`,
              } as React.CSSProperties
            }
          >
            <div className="animate-card-flash">
              <Card
                card={{
                  id: activeFlash.cardId,
                  name: activeFlash.cardName,
                  setCode: activeFlash.setCode,
                  cardNumber: "",
                  color: "",
                  manaCost: "",
                  types: [],
                  subtypes: [],
                  supertypes: [],
                  text: "",
                  isPlayable: false,
                  isSelected: false,
                  isChoosable: false,
                  controllerId: "",
                  ownerId: "",
                  zoneId: "",
                }}
                className="w-[240px] h-[336px]"
              />
            </div>
          </div>,
          document.body,
        )}

      {/* ── Ghost card while dragging from hand ───────────── */}
      {draggingHandCard &&
        createPortal(
          <div
            className="fixed pointer-events-none z-[9999]"
            style={{ left: ghostPos.x - 35, top: ghostPos.y - 49 }}
          >
            <Card
              card={draggingHandCard}
              className="w-[70px] h-[98px] opacity-70 shadow-2xl ring-2 ring-primary"
            />
          </div>,
          document.body,
        )}

      {/* ── Hover card preview ────────────────────────────── */}
      {hoveredCard && !viewingZone && !zoneTargetSelector && !libraryPeekModal && !spellStackModalOpen &&
       promptType !== "chooseKicker" && promptType !== "chooseBuyback" &&
       promptType !== "chooseMultikicker" && promptType !== "chooseReplicate" &&
       promptType !== "chooseAlternativeCost" && promptType !== "chooseMode" &&
       promptType !== "chooseOptionalTrigger" && (
        <CardPreview
          card={hoveredCard}
          mouseX={mousePos.x}
          mouseY={mousePos.y}
          showBackFace={showBackFace}
        />
      )}
    </div>
  );
}
