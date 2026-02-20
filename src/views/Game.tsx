import { useGameStore } from "@/stores/useGameStore";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Card as XMageCard, Player } from "@/types/xmage";
import { Card } from "@/components/game/Card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { BookOpen, Heart, Layers, Archive, Sword } from "lucide-react";

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

function PlayerPanel({ player, isOpponent }: { player: Player; isOpponent: boolean }) {
  return (
    <div className={cn(
      "flex items-center gap-3 px-3 py-2 border rounded-lg bg-card text-sm",
      isOpponent ? "flex-row" : "flex-row"
    )}>
      <div className="font-semibold truncate min-w-0">{player.name}</div>
      <div className="flex items-center gap-1 shrink-0">
        <Heart className="h-3.5 w-3.5 text-red-500" />
        <span className="font-bold">{player.life}</span>
      </div>
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
}: {
  cards: XMageCard[];
  label: string;
  emptyLabel: string;
  className?: string;
  onClickCard?: (card: XMageCard) => void;
}) {
  return (
    <div className={cn("flex flex-col gap-1 min-h-0", className)}>
      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">{label}</span>
      <div className="flex flex-wrap gap-2 p-2 min-h-[100px] border rounded-lg bg-muted/20 flex-1 content-start">
        {cards.length === 0 ? (
          <span className="text-xs text-muted-foreground italic self-center mx-auto">{emptyLabel}</span>
        ) : (
          cards.map((card) => (
            <div key={card.id} className="relative group shrink-0">
              <Card
                card={card}
                className={cn("w-[70px] h-[98px] shrink-0 hover:z-10",
                  card.isChoosable && onClickCard && "ring-2 ring-blue-400 cursor-pointer"
                )}
              />
              {card.isChoosable && onClickCard && (
                <button
                  className="absolute inset-0 rounded-lg opacity-0 group-hover:opacity-100 bg-blue-500/20 border-2 border-blue-400 transition-opacity"
                  onClick={() => onClickCard(card)}
                  title={`Target ${card.name}`}
                />
              )}
            </div>
          ))
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

function HandDisplay({ cards, onPlayCard }: { cards: XMageCard[]; onPlayCard: (card: XMageCard) => void }) {
  return (
    <div className="flex flex-col gap-1 shrink-0">
      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">Hand ({cards.length})</span>
      <div className="w-full overflow-x-auto">
        <div className="flex gap-2 pb-2 px-1 min-h-[120px] items-end">
          {cards.map((card) => (
            <div key={card.id} className="relative group shrink-0">
              <Card
                card={card}
                className={cn(
                  "w-[80px] h-[112px] transition-transform group-hover:-translate-y-3",
                  !card.isPlayable && "opacity-60 grayscale"
                )}
              />
              {card.isPlayable && (
                <button
                  className="absolute inset-0 rounded-lg opacity-0 group-hover:opacity-100 bg-primary/20 border-2 border-primary transition-opacity"
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

function GraveyardPeek({ count, label }: { count: number; label: string }) {
  return (
    <div className="flex flex-col items-center gap-0.5 cursor-pointer group">
      <div className="w-12 h-16 rounded border-2 border-dashed border-muted-foreground/40 flex items-center justify-center group-hover:border-primary transition-colors bg-muted/20">
        <div className="text-center">
          <Archive className="h-4 w-4 mx-auto text-muted-foreground" />
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
    chooseAction: "Choose a card to play or pass priority",
    chooseAttackers: "Declare attackers",
    chooseBlockers: "Declare blockers",
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
    targetPlayer,
    targetCard,
    targetAny,
    mulliganDecision,
    startGame,
    setupListeners,
  } = useGameStore();
  const [selectedCard, setSelectedCard] = useState<XMageCard | null>(null);

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
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [passPriority]);

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

  const me = gameView.players.find((p) => p.isHuman) ?? gameView.players[0];
  const opponent = gameView.players.find((p) => !p.isHuman) ?? gameView.players[1];

  const myPermanents = gameView.battlefield.filter((c) => c.controllerId === me.id);
  const opponentPermanents = gameView.battlefield.filter((c) => c.controllerId === opponent.id);

  const promptType = currentPrompt?.type;

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
        <Button onClick={() => window.location.reload()}>Play Again</Button>
      </div>
    );
  }

  function handlePlayCard(card: XMageCard) {
    if (!card.isPlayable) return;
    castSpell(card.id);
  }

  function handleBattlefieldClick(card: XMageCard) {
    if (!currentPrompt || !card.isChoosable) return;

    if (promptType === "chooseAttackers") {
      // For attackers: toggle selection (simplified: just send this one)
      declareAttackers([card.id]);
    } else if (promptType === "chooseTargetCard") {
      targetCard(card.id);
    } else if (promptType === "chooseTargetAny") {
      targetAny({ kind: "card", cardId: card.id });
    }
  }

  return (
    <div className="flex flex-col h-full gap-2 overflow-hidden">
      {/* Opponent panel */}
      <PlayerPanel player={opponent} isOpponent />

      {/* Opponent graveyard */}
      <div className="flex gap-2 shrink-0 px-1">
        <GraveyardPeek count={opponent.graveyardCount} label="GY" />
        <BattlefieldZone
          cards={opponentPermanents}
          label={`${opponent.name}'s Battlefield`}
          emptyLabel="No permanents"
          className="flex-1"
          onClickCard={promptType === "chooseTargetCard" || promptType === "chooseTargetAny" ? handleBattlefieldClick : undefined}
        />
      </div>

      {/* Stack */}
      {gameView.stack.length > 0 && (
        <div className="shrink-0 border rounded-lg p-2 bg-yellow-50 dark:bg-yellow-950/20">
          <p className="text-xs font-semibold text-yellow-700 dark:text-yellow-400 mb-1">Stack ({gameView.stack.length})</p>
          <div className="flex gap-2">
            {gameView.stack.map((obj) => (
              <Badge key={obj.id} variant="outline" className="text-xs">
                {obj.name}
              </Badge>
            ))}
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
        <GraveyardPeek count={me.graveyardCount} label="GY" />
        <BattlefieldZone
          cards={myPermanents}
          label="Your Battlefield"
          emptyLabel="No permanents"
          className="flex-1"
          onClickCard={promptType === "chooseAttackers" || promptType === "chooseTargetCard" || promptType === "chooseTargetAny" ? handleBattlefieldClick : undefined}
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
      <HandDisplay cards={gameView.myHand} onPlayCard={handlePlayCard} />

      {/* My panel + actions */}
      <div className="flex items-center gap-2 shrink-0">
        <div className="flex-1 min-w-0">
          <PlayerPanel player={me} isOpponent={false} />
        </div>
        <div className="flex gap-2 shrink-0">
          {promptType === "chooseAction" && (
            <Button size="sm" variant="outline" onClick={passPriority}>
              Pass (Space)
            </Button>
          )}
          {promptType === "chooseAttackers" && (
            <>
              <Button size="sm" variant="outline" onClick={passPriority}>
                No Attackers
              </Button>
              <Button size="sm" variant="secondary" className="flex items-center gap-1"
                onClick={() => {
                  // Attack with all available
                  const ids = currentPrompt?.availableAttackerIds ?? [];
                  declareAttackers(ids);
                }}
              >
                <Sword className="h-3.5 w-3.5" />
                Attack All
              </Button>
            </>
          )}
          {promptType === "chooseBlockers" && (
            <Button size="sm" variant="outline" onClick={passPriority}>
              No Blockers
            </Button>
          )}
          {(promptType === "chooseTargetPlayer" || promptType === "chooseTargetAny") && (
            <>
              {currentPrompt?.validPlayerIds?.map((pid) => {
                const player = gameView.players.find((p) => p.id === pid);
                return (
                  <Button key={pid} size="sm" variant="secondary"
                    onClick={() => {
                      if (promptType === "chooseTargetAny") {
                        targetAny({ kind: "player", playerId: pid });
                      } else {
                        targetPlayer(pid);
                      }
                    }}
                  >
                    Target {player?.name ?? pid} ({player?.life})
                  </Button>
                );
              })}
            </>
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
    </div>
  );
}
