import { useParams } from "react-router-dom";
import { useGameStore } from "@/stores/useGameStore";
import { useEffect, useState } from "react";
import type { GameView, Card as XMageCard, Player } from "@/types/xmage";
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

// Mock game state for development
function buildMockGameView(gameId: string): GameView {
  return {
    gameId,
    turn: 4,
    step: "main1",
    activePlayerId: "player-1",
    priorityPlayerId: "player-1",
    players: [
      {
        id: "player-1",
        name: "You",
        isHuman: true,
        life: 18,
        poison: 0,
        handCount: 5,
        libraryCount: 49,
        graveyardCount: 2,
        exileCount: 0,
        manaPool: { W: 0, U: 2, B: 0, R: 0, G: 0, C: 1 },
      },
      {
        id: "player-2",
        name: "Opponent",
        isHuman: false,
        life: 14,
        poison: 0,
        handCount: 3,
        libraryCount: 52,
        graveyardCount: 1,
        exileCount: 0,
        manaPool: { W: 0, U: 0, B: 0, R: 0, G: 0, C: 0 },
      },
    ],
    myHand: [
      { id: "h1", name: "Counterspell", setCode: "2ED", cardNumber: "56", color: "U", manaCost: "{U}{U}", cmc: 2, types: ["Instant"], subtypes: [], supertypes: [], text: "Counter target spell.", isPlayable: true, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "hand" },
      { id: "h2", name: "Lightning Bolt", setCode: "LEA", cardNumber: "161", color: "R", manaCost: "{R}", cmc: 1, types: ["Instant"], subtypes: [], supertypes: [], text: "Deal 3 damage to any target.", isPlayable: true, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "hand" },
      { id: "h3", name: "Island", setCode: "LEA", cardNumber: "287", color: "", manaCost: "", cmc: 0, types: ["Basic", "Land"], subtypes: ["Island"], supertypes: ["Basic"], text: "", isPlayable: true, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "hand" },
      { id: "h4", name: "Jace, the Mind Sculptor", setCode: "WWK", cardNumber: "31", color: "U", manaCost: "{2}{U}{U}", cmc: 4, types: ["Legendary", "Planeswalker"], subtypes: ["Jace"], supertypes: ["Legendary"], text: "+2: Look at the top card...", isPlayable: false, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "hand" },
      { id: "h5", name: "Force of Will", setCode: "ALL", cardNumber: "28", color: "U", manaCost: "{3}{U}{U}", cmc: 5, types: ["Instant"], subtypes: [], supertypes: [], text: "You may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.", isPlayable: false, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "hand" },
    ],
    battlefield: [
      { id: "bf1", name: "Island", setCode: "LEA", cardNumber: "287", color: "", manaCost: "", cmc: 0, types: ["Basic", "Land"], subtypes: ["Island"], supertypes: ["Basic"], text: "", isPlayable: true, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "battlefield" },
      { id: "bf2", name: "Island", setCode: "LEA", cardNumber: "287", color: "", manaCost: "", cmc: 0, types: ["Basic", "Land"], subtypes: ["Island"], supertypes: ["Basic"], text: "", isPlayable: true, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "battlefield" },
      { id: "bf3", name: "Island", setCode: "LEA", cardNumber: "287", color: "", manaCost: "", cmc: 0, types: ["Basic", "Land"], subtypes: ["Island"], supertypes: ["Basic"], text: "", isPlayable: true, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "battlefield" },
      { id: "bf4", name: "Snapcaster Mage", setCode: "ISD", cardNumber: "78", color: "U", manaCost: "{1}{U}", cmc: 2, types: ["Creature"], subtypes: ["Human", "Wizard"], supertypes: [], text: "Flash\nWhen Snapcaster Mage enters the battlefield, target instant or sorcery card in your graveyard gains flashback until end of turn.", power: "2", toughness: "1", isPlayable: true, isSelected: false, isChoosable: true, controllerId: "player-1", ownerId: "player-1", zoneId: "battlefield" },
      // Opponent's permanents (controller = player-2)
      { id: "bf5", name: "Tarmogoyf", setCode: "FUT", cardNumber: "153", color: "G", manaCost: "{1}{G}", cmc: 2, types: ["Creature"], subtypes: ["Lhurgoyf"], supertypes: [], text: "Tarmogoyf's power is equal to the number of card types among cards in all graveyards...", power: "*", toughness: "1+*", isPlayable: false, isSelected: false, isChoosable: true, controllerId: "player-2", ownerId: "player-2", zoneId: "battlefield" },
      { id: "bf6", name: "Mountain", setCode: "LEA", cardNumber: "289", color: "", manaCost: "", cmc: 0, types: ["Basic", "Land"], subtypes: ["Mountain"], supertypes: ["Basic"], text: "", isPlayable: false, isSelected: false, isChoosable: true, controllerId: "player-2", ownerId: "player-2", zoneId: "battlefield" },
    ],
    stack: [],
    exile: [],
    graveyard: [],
  };
}

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
}: {
  cards: XMageCard[];
  label: string;
  emptyLabel: string;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-col gap-1 min-h-0", className)}>
      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">{label}</span>
      <div className="flex flex-wrap gap-2 p-2 min-h-[100px] border rounded-lg bg-muted/20 flex-1 content-start">
        {cards.length === 0 ? (
          <span className="text-xs text-muted-foreground italic self-center mx-auto">{emptyLabel}</span>
        ) : (
          cards.map((card) => (
            <Card
              key={card.id}
              card={card}
              className="w-[70px] h-[98px] shrink-0 hover:z-10"
            />
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

export default function Game() {
  const { gameId } = useParams<{ gameId: string }>();
  const { gameView, updateGameView, passPriority } = useGameStore();
  const [selectedCard, setSelectedCard] = useState<XMageCard | null>(null);

  // Load mock data on mount
  useEffect(() => {
    if (!gameView && gameId) {
      updateGameView(buildMockGameView(gameId));
    }
  }, [gameId]);

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

  if (!gameView) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">Loading game...</p>
      </div>
    );
  }

  const me = gameView.players.find((p) => p.isHuman) ?? gameView.players[0];
  const opponent = gameView.players.find((p) => !p.isHuman) ?? gameView.players[1];

  const myPermanents = gameView.battlefield.filter((c) => c.controllerId === me.id);
  const opponentPermanents = gameView.battlefield.filter((c) => c.controllerId === opponent.id);

  const hasPriority = gameView.priorityPlayerId === me.id;

  function handlePlayCard(card: XMageCard) {
    // TODO: wire to game engine
    setSelectedCard(card);
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
        />
      </div>

      {/* Hand */}
      <HandDisplay cards={gameView.myHand} onPlayCard={handlePlayCard} />

      {/* My panel + actions */}
      <div className="flex items-center gap-2 shrink-0">
        <div className="flex-1 min-w-0">
          <PlayerPanel player={me} isOpponent={false} />
        </div>
        <div className="flex gap-2 shrink-0">
          {hasPriority && (
            <>
              <Button size="sm" variant="outline" onClick={passPriority}>
                Pass (Space)
              </Button>
              <Button size="sm" variant="secondary" className="flex items-center gap-1">
                <Sword className="h-3.5 w-3.5" />
                Attack
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Selected card info */}
      {selectedCard && (
        <div className="shrink-0 text-xs text-muted-foreground border-t pt-1">
          Selected: <span className="font-semibold text-foreground">{selectedCard.name}</span>
          {" "}— {selectedCard.text}
          <Button size="sm" variant="ghost" className="ml-2 h-5 text-xs" onClick={() => setSelectedCard(null)}>
            Clear
          </Button>
        </div>
      )}
    </div>
  );
}
