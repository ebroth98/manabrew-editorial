import { useGameStore } from "@/stores/useGameStore";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Card as XMageCard, Player } from "@/types/xmage";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { ZoneViewer } from "@/components/game/ZoneViewer";
import { ArrowOverlay } from "@/components/game/ArrowOverlay";
import { useGameArrows } from "@/components/game/useGameArrows";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { cn } from "@/lib/utils";
import { BookOpen, Heart, Layers, Archive, Sword, Skull, TimerOff } from "lucide-react";

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

// Phase bar definitions
const PHASES = [
  { id: "untap", label: "Untap", short: "UNT" },
  { id: "upkeep", label: "Upkeep", short: "UP" },
  { id: "draw", label: "Draw", short: "DR" },
  { id: "main1", label: "Main 1", short: "M1" },
  { id: "begin_combat", label: "Begin Combat", short: "BC" },
  { id: "declare_attackers", label: "Attackers", short: "ATK" },
  { id: "declare_blockers", label: "Blockers", short: "BLK" },
  { id: "combat_damage", label: "Damage", short: "DMG" },
  { id: "end_combat", label: "End Combat", short: "EC" },
  { id: "main2", label: "Main 2", short: "M2" },
  { id: "end", label: "End", short: "END" },
  { id: "cleanup", label: "Cleanup", short: "CL" },
];

const MANA_COLORS = [
  { key: "W", bg: "bg-yellow-50 border-yellow-200", text: "text-yellow-800" },
  { key: "U", bg: "bg-blue-100 border-blue-300", text: "text-blue-800" },
  { key: "B", bg: "bg-gray-800 border-gray-600", text: "text-gray-100" },
  { key: "R", bg: "bg-red-100 border-red-300", text: "text-red-800" },
  { key: "G", bg: "bg-green-100 border-green-300", text: "text-green-800" },
  { key: "C", bg: "bg-gray-100 border-gray-300", text: "text-gray-700" },
];

const DECK_OPTIONS = [
  { id: "red_burn", label: "Red Burn", desc: "Bolts + Shocks + Ogres + Giants", color: "text-red-500" },
  { id: "green_stompy", label: "Green Stompy", desc: "Giant Growth + Trample + Reach + Wurms", color: "text-green-500" },
  { id: "white_aggro", label: "White Aggro", desc: "Savannah Lions + First Strike + Flying", color: "text-yellow-500" },
  { id: "black_control", label: "Black Control", desc: "Doom Blade + Divination + Deathtouch", color: "text-purple-500" },
];

function ManaPool({ pool }: { pool: Record<string, number> }) {
  const total = Object.values(pool).reduce((a, b) => a + b, 0);
  if (total === 0) return <span className="text-xs text-muted-foreground italic">Empty</span>;
  return (
    <div className="flex gap-0.5 flex-wrap">
      {MANA_COLORS.map(({ key, bg, text }) =>
        (pool[key] ?? 0) > 0 ? (
          <span
            key={key}
            className={cn("inline-flex items-center justify-center w-5 h-5 rounded-full border text-xs font-bold", bg, text)}
            title={`${pool[key]} ${key}`}
          >
            {pool[key]}
          </span>
        ) : null
      )}
    </div>
  );
}

function PlayerPanel({
  player,
  isOpponent,
  isTargetable,
  onTarget,
}: {
  player: Player;
  isOpponent: boolean;
  isTargetable?: boolean;
  onTarget?: () => void;
}) {
  return (
    <div
      data-player-id={player.id}
      className={cn(
        "flex items-center gap-3 px-3 py-2 border rounded-lg bg-card text-sm transition-colors",
        isTargetable && "ring-2 ring-red-400 border-red-400 cursor-pointer hover:bg-red-50 dark:hover:bg-red-950/30"
      )}
      onClick={isTargetable ? onTarget : undefined}
      title={isTargetable ? `Target ${player.name}` : undefined}
    >
      <Avatar className={cn("h-8 w-8 shrink-0", isTargetable && "ring-2 ring-red-400")}>
        <AvatarFallback className={cn("text-xs font-bold", getAvatarColor(player.name))}>
          {getInitials(player.name)}
        </AvatarFallback>
      </Avatar>
      <div className="font-semibold truncate min-w-0">{player.name}</div>
      <div className="flex items-center gap-1 shrink-0">
        <Heart className="h-3.5 w-3.5 text-red-500" />
        <span className="font-bold">{player.life}</span>
      </div>
      {isTargetable && (
        <Badge variant="destructive" className="text-xs h-5 px-1 animate-pulse shrink-0">
          TARGET
        </Badge>
      )}
      {player.poison > 0 && (
        <Badge variant="destructive" className="text-xs h-5 px-1">
          {player.poison} poison
        </Badge>
      )}
      <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
        <BookOpen className="h-3 w-3" />
        <span>{player.libraryCount}</span>
      </div>
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
  onClickCard,
  onClickAnyCard,
  onHoverCard,
  pendingCardIds,
  attackingCardIds,
  tappableLandIds,
  onTapLand,
  untappableLandIds,
  onUntapLand,
}: {
  cards: XMageCard[];
  label: string;
  emptyLabel: string;
  className?: string;
  /** Called when clicking a card with isChoosable=true */
  onClickCard?: (card: XMageCard) => void;
  /** Called when clicking any card (used for assigning attackers during blocking) */
  onClickAnyCard?: (card: XMageCard) => void;
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent) => void;
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
}) {
  return (
    <div className={cn("flex flex-col gap-1 min-h-0", className)}>
      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">{label}</span>
      <div className="flex flex-wrap gap-2 p-2 min-h-[100px] border rounded-lg bg-muted/20 flex-1 content-start">
        {cards.length === 0 ? (
          <span className="text-xs text-muted-foreground italic self-center mx-auto">{emptyLabel}</span>
        ) : (
          cards.map((card) => {
            const isPending = pendingCardIds?.includes(card.id);
            const isAttacking = attackingCardIds?.includes(card.id);
            const isTappable = tappableLandIds?.includes(card.id);
            const isUntappable = untappableLandIds?.includes(card.id);
            const isChoosableClick = (card.isChoosable && !!onClickCard) || (isAttacking && !!onClickAnyCard);
            return (
              <div
                key={card.id}
                data-card-id={card.id}
                className="relative group shrink-0"
                onMouseEnter={(e) => onHoverCard?.(card, e)}
                onMouseLeave={() => onHoverCard?.(null)}
              >
                <Card
                  card={card}
                  isTapped={card.tapped}
                  className={cn("w-[70px] h-[98px] shrink-0 hover:z-10",
                    card.isChoosable && onClickCard && "ring-2 ring-blue-400 cursor-pointer",
                    isPending && "ring-2 ring-orange-400 cursor-pointer",
                    isAttacking && "ring-2 ring-red-500 cursor-pointer",
                    isTappable && !isAttacking && "ring-2 ring-yellow-400 cursor-pointer",
                    isUntappable && !isAttacking && !isTappable && "ring-2 ring-cyan-400 cursor-pointer",
                  )}
                />
                {/* Tap-for-mana overlay — shown only during chooseAction */}
                {isTappable && onTapLand && (
                  <button
                    className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 bg-yellow-400/20 border-2 border-yellow-400 transition-opacity flex items-end justify-center pb-1"
                    onClick={() => onTapLand(card)}
                    title={`Tap ${card.name} for mana`}
                  >
                    <span className="text-[9px] font-bold text-yellow-800 bg-yellow-200/90 px-1 rounded leading-none">TAP</span>
                  </button>
                )}
                {/* Untap overlay — shown for tapped lands with unspent mana */}
                {isUntappable && onUntapLand && (
                  <button
                    className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 bg-cyan-400/20 border-2 border-cyan-400 transition-opacity flex items-end justify-center pb-1"
                    onClick={() => onUntapLand(card)}
                    title={`Untap ${card.name} (undo mana)`}
                  >
                    <span className="text-[9px] font-bold text-cyan-900 bg-cyan-200/90 px-1 rounded leading-none">UNTAP</span>
                  </button>
                )}
                {/* Choosable / attacker overlay */}
                {!isTappable && isChoosableClick && (
                  <button
                    className={cn(
                      "absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 border-2 transition-opacity",
                      isPending
                        ? "bg-orange-500/20 border-orange-400"
                        : isAttacking
                          ? "bg-red-500/20 border-red-500"
                          : "bg-blue-500/20 border-blue-400"
                    )}
                    onClick={() => {
                      if (card.isChoosable && onClickCard) onClickCard(card);
                      else if (isAttacking && onClickAnyCard) onClickAnyCard(card);
                    }}
                    title={isPending ? `Deselect ${card.name}` : isAttacking ? `Block ${card.name}` : `Select ${card.name}`}
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

function PhaseBar({ currentStep, activePlayerId, myPlayerId }: { currentStep: string; activePlayerId: string; myPlayerId: string }) {
  const isMyTurn = activePlayerId === myPlayerId;
  return (
    <div className="flex items-center gap-1 overflow-x-auto py-1 px-2 bg-muted/30 border rounded-lg shrink-0">
      <span className={cn("text-xs font-semibold shrink-0 mr-1", isMyTurn ? "text-green-600" : "text-orange-500")}>
        {isMyTurn ? "Your Turn" : "Opp Turn"}
      </span>
      {PHASES.map((phase) => (
        <div
          key={phase.id}
          className={cn(
            "text-xs px-1.5 py-0.5 rounded shrink-0 border transition-colors",
            currentStep === phase.id
              ? "bg-primary text-primary-foreground border-primary font-semibold"
              : "bg-background border-border text-muted-foreground"
          )}
          title={phase.label}
        >
          {phase.short}
        </div>
      ))}
    </div>
  );
}

function HandDisplay({
  cards,
  onPlayCard,
  onHoverCard,
}: {
  cards: XMageCard[];
  onPlayCard: (card: XMageCard) => void;
  onHoverCard?: (card: XMageCard | null, e?: React.MouseEvent) => void;
}) {
  return (
    <div className="flex flex-col gap-1 shrink-0">
      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">Hand ({cards.length})</span>
      <div className="w-full overflow-x-auto">
        <div className="flex gap-2 pb-2 px-1 min-h-[120px] items-end">
          {cards.map((card) => (
            <div
              key={card.id}
              className="relative group shrink-0"
              onMouseEnter={(e) => onHoverCard?.(card, e)}
              onMouseLeave={() => onHoverCard?.(null)}
            >
              <Card
                card={card}
                className={cn(
                  "w-[80px] h-[112px] transition-transform group-hover:-translate-y-3",
                  !card.isPlayable && "opacity-60 grayscale"
                )}
              />
              {card.isPlayable && (
                <button
                  className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 bg-primary/20 border-2 border-primary transition-opacity"
                  onClick={() => onPlayCard(card)}
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

function ZonePeek({
  count,
  label,
  icon: Icon = Archive,
  onClick,
}: {
  count: number;
  label: string;
  icon?: React.ElementType;
  onClick?: () => void;
}) {
  return (
    <div
      className="flex flex-col items-center gap-0.5 cursor-pointer group"
      onClick={onClick}
    >
      <div className="w-12 h-16 rounded border-2 border-dashed border-muted-foreground/40 flex items-center justify-center group-hover:border-primary transition-colors bg-muted/20">
        <div className="text-center">
          <Icon className="h-4 w-4 mx-auto text-muted-foreground" />
          <span className="text-xs font-bold text-muted-foreground">{count}</span>
        </div>
      </div>
      <span className="text-xs text-muted-foreground">{label}</span>
    </div>
  );
}

function PromptBanner({ promptType }: { promptType: string }) {
  const labels: Record<string, string> = {
    mulligan: "Keep this hand?",
    chooseAction: "Choose a card to play or pass priority (Space / F6)",
    chooseAttackers: "Declare attackers — click creatures to toggle, then Attack or Attack All",
    chooseBlockers: "Declare blockers — click an attacking creature, then click your blocker",
    chooseTargetPlayer: "Choose a target player",
    chooseTargetCard: "Choose a target creature",
    chooseTargetAny: "Choose a target (player or creature)",
    gameOver: "Game Over",
  };
  const label = labels[promptType] ?? promptType;
  return (
    <div className="shrink-0 border rounded-lg p-2 bg-blue-50 dark:bg-blue-950/20 text-center">
      <p className="text-sm font-semibold text-blue-700 dark:text-blue-400">{label}</p>
    </div>
  );
}

function DeckPicker({ onPick }: { onPick: (id: string) => void }) {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-6">
      <h2 className="text-2xl font-bold">Choose Your Deck</h2>
      <div className="grid grid-cols-2 gap-4 max-w-md">
        {DECK_OPTIONS.map((deck) => (
          <button
            key={deck.id}
            className="border rounded-lg p-4 hover:bg-muted/50 transition-colors text-left"
            onClick={() => onPick(deck.id)}
          >
            <p className={cn("font-semibold", deck.color)}>{deck.label}</p>
            <p className="text-xs text-muted-foreground mt-1">{deck.desc}</p>
          </button>
        ))}
      </div>
    </div>
  );
}

export default function Game() {
  const {
    gameView,
    currentPrompt,
    isGameActive,
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
    concede,
    endGame,
    startGame,
    setupListeners,
  } = useGameStore();
  const containerRef = useRef<HTMLDivElement>(null);

  const [selectedCard, setSelectedCard] = useState<XMageCard | null>(null);
  const [hoveredCard, setHoveredCard] = useState<XMageCard | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  // Combat state
  const [pendingAttackers, setPendingAttackers] = useState<string[]>([]);
  /** The attacker card ID the player has selected to assign a blocker to (attacker-first flow) */
  const [pendingAttacker, setPendingAttacker] = useState<string | null>(null);
  const [blockAssignments, setBlockAssignments] = useState<{ blockerId: string; attackerId: string }[]>([]);

  // Zone viewer
  const [viewingZone, setViewingZone] = useState<{ title: string; cards: XMageCard[] } | null>(null);
  function openZone(title: string, cards: XMageCard[]) { setViewingZone({ title, cards }); }
  function closeZone() { setViewingZone(null); }

  // Concede confirmation
  const [confirmConcede, setConfirmConcede] = useState(false);

  const promptType = currentPrompt?.type;

  function handleHoverCard(card: XMageCard | null, e?: React.MouseEvent) {
    setHoveredCard(card);
    if (e && card) setMousePos({ x: e.clientX, y: e.clientY });
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

  // Poll for game state as fallback (in case events don't work)
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  useEffect(() => {
    if (!isGameActive) return;
    // Poll every 300ms when we don't have a gameView yet, or after responding
    pollRef.current = setInterval(async () => {
      // Never overwrite a game-over state — the dying thread keeps emitting prompts
      if (useGameStore.getState().gameView?.gameOver) return;
      try {
        const prompt = await invoke<any>('get_prompt');
        if (prompt) {
          const gv = prompt.gameView || prompt.game_view;
          if (gv) {
            useGameStore.setState({
              gameView: gv,
              currentPrompt: prompt,
              debugInfo: `Poll OK: ${prompt.type}`,
            });
          }
        }
      } catch (e) {
        // ignore poll errors
      }
    }, 300);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [isGameActive]);

  // Keyboard shortcuts
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
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

  // Reset combat state whenever the prompt type changes
  useEffect(() => {
    setPendingAttackers([]);
    setPendingAttacker(null);
    setBlockAssignments([]);
  }, [currentPrompt?.type]);

  // Targeting / combat arrows — must be called unconditionally (Rules of Hooks)
  // Player IDs are empty strings when gameView is not yet available; the hook
  // will safely produce no arrows in that case.
  const me = gameView?.players.find((p) => p.isHuman) ?? gameView?.players[0];
  const opponent = gameView?.players.find((p) => !p.isHuman) ?? gameView?.players[1];
  const arrows = useGameArrows({
    containerRef,
    promptType,
    attackerIds: currentPrompt?.attackerIds ?? [],
    blockAssignments,
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

  // Show deck picker if no active game
  if (!isGameActive) {
    return <DeckPicker onPick={(id) => startGame(id)} />;
  }

  // Loading
  if (!gameView) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <p className="text-muted-foreground">Waiting for game state...</p>
        {debugInfo && <p className="text-xs text-muted-foreground font-mono">{debugInfo}</p>}
        <Button variant="outline" size="sm" onClick={async () => {
          try {
            const raw = await invoke<any>('get_prompt');
            useGameStore.setState({ debugInfo: `Manual poll: ${JSON.stringify(raw)?.slice(0, 200)}` });
          } catch (e) {
            useGameStore.setState({ debugInfo: `Poll error: ${e}` });
          }
        }}>
          Debug: Poll Now
        </Button>
      </div>
    );
  }

  // me / opponent are already derived above (before the early returns).
  // Re-assert non-null: if we reach here, gameView is defined.
  const myPermanents = gameView.battlefield.filter((c) => c.controllerId === me!.id);
  const opponentPermanents = gameView.battlefield.filter((c) => c.controllerId === opponent!.id);

  // Game over overlay
  if (gameView.gameOver || promptType === "gameOver") {
    const winnerId = gameView.winnerId;
    const didWin = winnerId === me.id;
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <h2 className={cn("text-3xl font-bold", didWin ? "text-green-600" : "text-red-600")}>
          {didWin ? "You Win!" : "You Lose!"}
        </h2>
        <p className="text-muted-foreground">
          Final life: You {me.life} — {opponent.name} {opponent.life}
        </p>
        <p className="text-sm text-muted-foreground">Turn {gameView.turn}</p>
        <p className="text-xs text-muted-foreground italic">Returning to menu…</p>
        <Button variant="outline" size="sm" onClick={() => endGame()}>Return to Menu</Button>
      </div>
    );
  }

  function handlePlayCard(card: XMageCard) {
    if (!card.isPlayable) return;
    castSpell(card.id);
  }

  const playerIsTargetable = (promptType === "chooseTargetPlayer" || promptType === "chooseTargetAny")
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
        prev.includes(card.id) ? prev.filter((id) => id !== card.id) : [...prev, card.id]
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

  return (
    <div ref={containerRef} className="relative flex flex-col h-full gap-2 overflow-hidden">
      {/* Targeting / combat arrow overlay — sits on top of all game elements */}
      <ArrowOverlay arrows={arrows} />

      {/* Opponent panel */}
      <PlayerPanel
        player={opponent}
        isOpponent
        isTargetable={playerIsTargetable(opponent.id)}
        onTarget={() => handleTargetPlayer(opponent.id)}
      />

      {/* Opponent graveyard + exile + battlefield */}
      <div className="flex gap-2 shrink-0 px-1">
        <div className="flex flex-col gap-1">
          <ZonePeek
            count={opponent.graveyardCount}
            label="GY"
            onClick={() => openZone(`${opponent.name}'s Graveyard`, gameView.opponentGraveyard ?? [])}
          />
          <ZonePeek
            count={opponent.exileCount}
            label="Exile"
            onClick={() => openZone(`${opponent.name}'s Exile`, gameView.opponentExile ?? [])}
          />
        </div>
        <BattlefieldZone
          cards={opponentPermanents}
          label={`${opponent.name}'s Battlefield`}
          emptyLabel="No permanents"
          className="flex-1"
          onClickCard={promptType === "chooseTargetCard" || promptType === "chooseTargetAny" ? handleBattlefieldClick : undefined}
          onClickAnyCard={promptType === "chooseBlockers" ? handleAttackerClick : undefined}
          onHoverCard={handleHoverCard}
          pendingCardIds={promptType === "chooseBlockers" && pendingAttacker ? [pendingAttacker] : undefined}
          attackingCardIds={promptType === "chooseBlockers" ? (currentPrompt?.attackerIds ?? []) : undefined}
        />
      </div>

      {/* Stack */}
      {gameView.stack.length > 0 && (
        <div className="shrink-0 border rounded-lg p-2 bg-yellow-50 dark:bg-yellow-950/20">
          <p className="text-xs font-semibold text-yellow-700 dark:text-yellow-400 mb-1">Stack ({gameView.stack.length})</p>
          <div className="flex flex-col gap-1">
            {gameView.stack.map((obj) => {
              const srcCard = gameView.battlefield.find((c) => c.id === obj.sourceId)
                ?? gameView.myHand.find((c) => c.id === obj.sourceId);
              const color = srcCard?.color ?? "";
              const borderColor =
                color === "White" ? "border-yellow-300" :
                color === "Blue" ? "border-blue-400" :
                color === "Black" ? "border-gray-600" :
                color === "Red" ? "border-red-500" :
                color === "Green" ? "border-green-500" :
                "border-gray-300";
              return (
                <div key={obj.id} className={cn("flex flex-col border-l-4 pl-2 py-0.5", borderColor)}>
                  <span className="text-xs font-bold leading-tight">{obj.name}</span>
                  {obj.text && <span className="text-xs text-muted-foreground leading-tight">{obj.text}</span>}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Combat report during blocking */}
      {promptType === "chooseBlockers" && currentPrompt?.attackerIds && currentPrompt.attackerIds.length > 0 && (
        <div className="shrink-0 border rounded-lg p-2 bg-red-50 dark:bg-red-950/20">
          <p className="text-xs font-semibold text-red-700 dark:text-red-400 mb-1">Combat</p>
          <div className="flex flex-col gap-0.5">
            {currentPrompt.attackerIds.map((aid) => {
              const attacker = gameView.battlefield.find((c) => c.id === aid);
              const blockers = blockAssignments.filter((a) => a.attackerId === aid);
              const blockerNames = blockers.map((b) => {
                const bc = gameView.battlefield.find((c) => c.id === b.blockerId);
                return bc?.name ?? b.blockerId;
              });
              return (
                <div key={aid} className="text-xs flex gap-1 items-center">
                  <span className="font-semibold">{attacker?.name ?? aid}</span>
                  <span className="text-muted-foreground">→</span>
                  <span className={blockerNames.length === 0 ? "text-red-500 italic" : "text-muted-foreground"}>
                    {blockerNames.length === 0 ? "unblocked" : blockerNames.join(", ")}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Prompt banner */}
      {promptType && promptType !== "gameOver" && (
        <PromptBanner promptType={promptType} />
      )}

      {/* Phase bar */}
      <PhaseBar
        currentStep={gameView.step}
        activePlayerId={gameView.activePlayerId}
        myPlayerId={me.id}
      />

      {/* My battlefield */}
      <div className="flex gap-2 shrink-0 px-1 flex-1 min-h-0">
        <div className="flex flex-col gap-1">
          <ZonePeek
            count={me.graveyardCount}
            label="GY"
            onClick={() => openZone("Your Graveyard", gameView.graveyard)}
          />
          <ZonePeek
            count={me.exileCount}
            label="Exile"
            onClick={() => openZone("Your Exile", gameView.exile)}
          />
        </div>
        <BattlefieldZone
          cards={myPermanents}
          label="Your Battlefield"
          emptyLabel="No permanents"
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
          pendingCardIds={
            promptType === "chooseAttackers"
              ? pendingAttackers
              : promptType === "chooseBlockers"
                ? blockAssignments.map((a) => a.blockerId)
                : undefined
          }
          tappableLandIds={promptType === "chooseAction" ? (currentPrompt?.tappableLandIds ?? []) : undefined}
          onTapLand={promptType === "chooseAction" ? (card) => tapLand(card.id) : undefined}
          untappableLandIds={promptType === "chooseAction" ? (currentPrompt?.untappableLandIds ?? []) : undefined}
          onUntapLand={promptType === "chooseAction" ? (card) => untapLand(card.id) : undefined}
        />
      </div>

      {/* Mulligan UI */}
      {promptType === "mulligan" && (
        <div className="shrink-0 flex gap-2 justify-center p-2">
          <Button size="sm" onClick={() => mulliganDecision(true)}>Keep Hand</Button>
          <Button size="sm" variant="destructive" onClick={() => mulliganDecision(false)}>Mulligan</Button>
        </div>
      )}

      {/* Hand */}
      <HandDisplay cards={gameView.myHand} onPlayCard={handlePlayCard} onHoverCard={handleHoverCard} />

      {/* My panel + actions */}
      <div className="flex items-center gap-2 shrink-0">
        <div className="flex-1 min-w-0">
          <PlayerPanel
            player={me}
            isOpponent={false}
            isTargetable={playerIsTargetable(me.id)}
            onTarget={() => handleTargetPlayer(me.id)}
          />
        </div>
        <div className="flex gap-2 shrink-0 items-center">
          {promptType === "chooseAction" && (
            <>
              <Button size="sm" variant="outline" onClick={passPriority}>
                Pass (Space)
              </Button>
              <Button
                size="sm"
                variant="outline"
                className="flex items-center gap-1"
                onClick={passPriority}
                title="Pass priority to end of turn (F6)"
              >
                <TimerOff className="h-3.5 w-3.5" />
                End Turn (F6)
              </Button>
            </>
          )}
          {promptType === "chooseAttackers" && (
            <>
              <Button size="sm" variant="outline" onClick={passPriority}>
                No Attackers
              </Button>
              <Button
                size="sm"
                variant="secondary"
                className="flex items-center gap-1"
                onClick={() => declareAttackers(currentPrompt?.availableAttackerIds ?? [])}
              >
                <Sword className="h-3.5 w-3.5" />
                Attack All
              </Button>
              {pendingAttackers.length > 0 && (
                <Button
                  size="sm"
                  className="flex items-center gap-1 bg-orange-500 hover:bg-orange-600 text-white"
                  onClick={() => declareAttackers(pendingAttackers)}
                >
                  <Sword className="h-3.5 w-3.5" />
                  Attack ({pendingAttackers.length})
                </Button>
              )}
            </>
          )}
          {promptType === "chooseBlockers" && (
            <>
              <Button size="sm" variant="outline" onClick={passPriority}>
                No Blockers
              </Button>
              {pendingAttacker && (
                <span className="text-xs text-muted-foreground italic self-center">
                  Now click your blocker
                </span>
              )}
              {blockAssignments.length > 0 && (
                <Button
                  size="sm"
                  className="bg-blue-600 hover:bg-blue-700 text-white"
                  onClick={() => declareBlockers(blockAssignments)}
                >
                  Confirm Blocks ({blockAssignments.length})
                </Button>
              )}
            </>
          )}
          {/* Concede button — always visible */}
          {confirmConcede ? (
            <>
              <span className="text-xs text-muted-foreground italic self-center">Concede?</span>
              <Button
                size="sm"
                variant="destructive"
                onClick={() => { concede(); setConfirmConcede(false); }}
              >
                Yes, Concede
              </Button>
              <Button size="sm" variant="outline" onClick={() => setConfirmConcede(false)}>
                Cancel
              </Button>
            </>
          ) : (
            <Button
              size="sm"
              variant="ghost"
              className="flex items-center gap-1 text-muted-foreground hover:text-destructive"
              onClick={() => setConfirmConcede(true)}
              title="Concede the game"
            >
              <Skull className="h-3.5 w-3.5" />
              Concede
            </Button>
          )}
        </div>
      </div>

      {/* Game log */}
      {gameLog.length > 0 && (
        <div className="shrink-0 max-h-16 overflow-y-auto text-xs text-muted-foreground border-t pt-1 px-1">
          {gameLog.slice(-5).map((msg, i) => (
            <div key={i}>{msg}</div>
          ))}
        </div>
      )}

      {/* Selected card info */}
      {selectedCard && (
        <div className="shrink-0 text-xs text-muted-foreground border-t pt-1">
          Selected: <span className="font-semibold text-foreground">{selectedCard.name}</span>
          {" "}&mdash; {selectedCard.text}
          <Button size="sm" variant="ghost" className="ml-2 h-5 text-xs" onClick={() => setSelectedCard(null)}>
            Clear
          </Button>
        </div>
      )}

      {/* Zone viewer modal */}
      {viewingZone && (
        <ZoneViewer
          title={viewingZone.title}
          cards={viewingZone.cards}
          onClose={closeZone}
        />
      )}

      {/* Hover card preview */}
      {hoveredCard && (
        <CardPreview card={hoveredCard} mouseX={mousePos.x} mouseY={mousePos.y} />
      )}
    </div>
  );
}
