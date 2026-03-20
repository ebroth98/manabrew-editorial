import { useState, useEffect } from "react";
import { tauriApi, type PresetDeckInfo } from "@/api/tauri";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { Search, Shuffle, Swords, User, Bot } from "lucide-react";
import { useDeckStore } from "@/stores/useDeckStore";

interface SelectedDeck {
  id: string;
  name: string;
  desc?: string;
  color?: string;
  deckList: { name: string; setCode: string }[];
}

interface DeckVsSelectorProps {
  onStart: (
    playerDeck: { name: string; setCode: string }[],
    opponentDeck: { name: string; setCode: string }[]
  ) => void;
}

type PickingSide = "player" | "opponent";

export function DeckVsSelector({ onStart }: DeckVsSelectorProps) {
  const [presetDecks, setPresetDecks] = useState<PresetDeckInfo[]>([]);
  const [playerDeck, setPlayerDeck] = useState<SelectedDeck | null>(null);
  const [opponentDeck, setOpponentDeck] = useState<SelectedDeck | null>(null);
  const [pickingSide, setPickingSide] = useState<PickingSide>("player");
  const [deckSearch, setDeckSearch] = useState("");
  const { savedDecks, currentDeck } = useDeckStore();

  useEffect(() => {
    tauriApi.deck.getPresetDecks()
      .then(setPresetDecks)
      .catch((e) =>
        console.error("[DeckVsSelector] Failed to load preset decks:", e)
      );
  }, []);

  const searchLower = deckSearch.toLowerCase();
  const filteredDecks = searchLower
    ? presetDecks.filter(
        (d) =>
          d.label.toLowerCase().includes(searchLower) ||
          d.desc.toLowerCase().includes(searchLower)
      )
    : presetDecks;

  const userDeckEntries: SelectedDeck[] = [
    {
      id: "current",
      name: currentDeck.name,
      deckList: currentDeck.cards.map((c) => ({ name: c.name, setCode: c.setCode ?? "" })),
    },
    ...savedDecks.map((s) => ({
      id: s.id,
      name: s.deck.name,
      deckList: s.deck.cards.map((c) => ({ name: c.name, setCode: c.setCode ?? "" })),
    })),
  ];

  const filteredUserDecks = searchLower
    ? userDeckEntries.filter((d) => d.name.toLowerCase().includes(searchLower))
    : userDeckEntries;

  function assignDeck(selected: SelectedDeck) {
    if (pickingSide === "player") {
      setPlayerDeck(selected);
      if (!opponentDeck) setPickingSide("opponent");
    } else {
      setOpponentDeck(selected);
    }
  }

  function selectDeck(deck: PresetDeckInfo) {
    assignDeck({
      id: deck.id,
      name: deck.label,
      desc: deck.desc,
      color: deck.color,
      deckList: [{ name: deck.id, setCode: "" }],
    });
  }

  function selectUserDeck(entry: SelectedDeck) {
    assignDeck(entry);
  }

  function handleRandomOpponent() {
    // Pick a random preset for the opponent
    if (presetDecks.length === 0) return;
    const random = presetDecks[Math.floor(Math.random() * presetDecks.length)];
    setOpponentDeck({
      id: random.id,
      name: random.label,
      desc: random.desc,
      color: random.color,
      deckList: [{ name: random.id, setCode: "" }],
    });
  }

  function handleFight() {
    if (!playerDeck || !opponentDeck) return;
    onStart(playerDeck.deckList, opponentDeck.deckList);
  }

  const isReady = !!playerDeck && !!opponentDeck;

  return (
    <div className="flex flex-col h-full">
      {/* ── Top: VS panels ── */}
      <div className="flex items-stretch gap-0 border-b flex-shrink-0">
        {/* Player side */}
        <FighterPanel
          side="player"
          label="YOU"
          icon={<User className="h-4 w-4" />}
          deck={playerDeck}
          isActive={pickingSide === "player"}
          onClick={() => setPickingSide("player")}
          onClear={() => setPlayerDeck(null)}
        />

        {/* VS divider */}
        <div className="flex flex-col items-center justify-center px-4 flex-shrink-0 bg-muted/30 border-x">
          <span className="text-2xl font-black tracking-tighter text-muted-foreground/60">
            VS
          </span>
        </div>

        {/* Opponent side */}
        <FighterPanel
          side="opponent"
          label="AI"
          icon={<Bot className="h-4 w-4" />}
          deck={opponentDeck}
          isActive={pickingSide === "opponent"}
          onClick={() => setPickingSide("opponent")}
          onClear={() => setOpponentDeck(null)}
          extra={
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleRandomOpponent();
              }}
              className="inline-flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors mt-1"
            >
              <Shuffle className="h-3 w-3" />
              Random
            </button>
          }
        />
      </div>

      {/* ── Picking indicator ── */}
      <div className="px-4 py-2 border-b bg-muted/10 flex items-center gap-2 flex-shrink-0">
        <span className="text-xs text-muted-foreground">Picking for:</span>
        <div className="flex gap-1">
          <button
            type="button"
            onClick={() => setPickingSide("player")}
            className={cn(
              "px-2.5 py-1 rounded-md text-xs font-medium transition-colors",
              pickingSide === "player"
                ? "bg-blue-500/10 text-blue-600 dark:text-blue-400 ring-1 ring-blue-500/30"
                : "text-muted-foreground hover:bg-muted/60"
            )}
          >
            <User className="h-3 w-3 inline mr-1" />
            You
          </button>
          <button
            type="button"
            onClick={() => setPickingSide("opponent")}
            className={cn(
              "px-2.5 py-1 rounded-md text-xs font-medium transition-colors",
              pickingSide === "opponent"
                ? "bg-red-500/10 text-red-600 dark:text-red-400 ring-1 ring-red-500/30"
                : "text-muted-foreground hover:bg-muted/60"
            )}
          >
            <Bot className="h-3 w-3 inline mr-1" />
            AI
          </button>
        </div>
      </div>

      {/* ── Search bar ── */}
      <div className="px-4 pt-3 pb-2 flex-shrink-0">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground pointer-events-none" />
          <input
            type="text"
            placeholder="Filter decks..."
            value={deckSearch}
            onChange={(e) => setDeckSearch(e.target.value)}
            className="w-full pl-8 pr-3 py-1.5 rounded-md border bg-background text-sm focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
      </div>

      {/* ── Deck grid ── */}
      <div className="flex-1 overflow-y-auto px-4 pb-4 space-y-4">
        {/* User decks */}
        {filteredUserDecks.length > 0 && (
          <div>
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-semibold pt-2 pb-1">
              My Decks
            </p>
            <div className="grid grid-cols-4 gap-2">
              {filteredUserDecks.map((entry) => {
                const isPlayerDeck = playerDeck?.id === entry.id;
                const isOpponentDeck = opponentDeck?.id === entry.id;
                return (
                  <button
                    key={entry.id}
                    type="button"
                    onClick={() => selectUserDeck(entry)}
                    className={cn(
                      "rounded-lg border p-2.5 text-left transition-all relative",
                      isPlayerDeck && isOpponentDeck
                        ? "border-purple-500 bg-purple-500/5 ring-1 ring-purple-500"
                        : isPlayerDeck
                          ? "border-blue-500 bg-blue-500/5 ring-1 ring-blue-500"
                          : isOpponentDeck
                            ? "border-red-500 bg-red-500/5 ring-1 ring-red-500"
                            : "border-border hover:bg-muted/40 hover:shadow-sm"
                    )}
                  >
                    <div className="absolute top-1 right-1 flex gap-0.5">
                      {isPlayerDeck && (
                        <span className="w-4 h-4 rounded-full bg-blue-500 text-white flex items-center justify-center">
                          <User className="h-2.5 w-2.5" />
                        </span>
                      )}
                      {isOpponentDeck && (
                        <span className="w-4 h-4 rounded-full bg-red-500 text-white flex items-center justify-center">
                          <Bot className="h-2.5 w-2.5" />
                        </span>
                      )}
                    </div>
                    <span className="font-semibold text-xs leading-tight block pr-5 truncate">
                      {entry.name}
                    </span>
                    <p className="text-[10px] text-muted-foreground mt-0.5">
                      {entry.deckList.length} cards
                    </p>
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* Preset decks */}
        <div>
          {filteredUserDecks.length > 0 && (
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-semibold pb-1">
              Preset Decks
            </p>
          )}
          {filteredDecks.length === 0 ? (
            <p className="text-xs text-muted-foreground italic py-4">
              No decks match your search.
            </p>
          ) : (
            <div className="grid grid-cols-4 gap-2 pt-1">
              {filteredDecks.map((deck) => {
                const isPlayerDeck = playerDeck?.id === deck.id;
                const isOpponentDeck = opponentDeck?.id === deck.id;
                return (
                  <button
                    key={deck.id}
                    type="button"
                    onClick={() => selectDeck(deck)}
                    className={cn(
                      "rounded-lg border p-2.5 text-left transition-all relative",
                      isPlayerDeck && isOpponentDeck
                        ? "border-purple-500 bg-purple-500/5 ring-1 ring-purple-500"
                        : isPlayerDeck
                          ? "border-blue-500 bg-blue-500/5 ring-1 ring-blue-500"
                          : isOpponentDeck
                            ? "border-red-500 bg-red-500/5 ring-1 ring-red-500"
                            : "border-border hover:bg-muted/40 hover:shadow-sm"
                    )}
                  >
                    {/* Selection badges */}
                    <div className="absolute top-1 right-1 flex gap-0.5">
                      {isPlayerDeck && (
                        <span className="w-4 h-4 rounded-full bg-blue-500 text-white flex items-center justify-center">
                          <User className="h-2.5 w-2.5" />
                        </span>
                      )}
                      {isOpponentDeck && (
                        <span className="w-4 h-4 rounded-full bg-red-500 text-white flex items-center justify-center">
                          <Bot className="h-2.5 w-2.5" />
                        </span>
                      )}
                    </div>

                    <div className="flex items-start justify-between gap-1 mb-1 pr-5">
                      <span
                        className={cn(
                          "font-semibold text-xs leading-tight",
                          deck.color
                        )}
                      >
                        {deck.label}
                      </span>
                    </div>
                    <p className="text-[10px] text-muted-foreground leading-tight line-clamp-2">
                      {deck.desc}
                    </p>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* ── Footer: Fight button ── */}
      <div className="px-4 py-3 border-t flex items-center justify-between bg-muted/10 flex-shrink-0">
        <div className="flex items-center gap-2 text-sm min-w-0">
          {playerDeck ? (
            <span className="text-xs truncate">
              <span className="text-blue-600 dark:text-blue-400 font-medium">
                {playerDeck.name}
              </span>
              <span className="text-muted-foreground mx-1.5">vs</span>
              {opponentDeck ? (
                <span className="text-red-600 dark:text-red-400 font-medium">
                  {opponentDeck.name}
                </span>
              ) : (
                <span className="text-muted-foreground italic">
                  Pick opponent...
                </span>
              )}
            </span>
          ) : (
            <span className="text-xs text-muted-foreground italic">
              Pick your deck to start
            </span>
          )}
        </div>
        <Button
          size="sm"
          onClick={handleFight}
          disabled={!isReady}
          className="gap-1.5"
        >
          <Swords className="h-3.5 w-3.5" />
          Fight!
        </Button>
      </div>
    </div>
  );
}

// ── Fighter panel ──────────────────────────────────────────────────

interface FighterPanelProps {
  side: PickingSide;
  label: string;
  icon: React.ReactNode;
  deck: SelectedDeck | null;
  isActive: boolean;
  onClick: () => void;
  onClear: () => void;
  extra?: React.ReactNode;
}

function FighterPanel({
  side,
  label,
  icon,
  deck,
  isActive,
  onClick,
  onClear,
  extra,
}: FighterPanelProps) {
  const sideColor =
    side === "player"
      ? {
          activeBg: "bg-blue-500/5",
          activeRing: "ring-blue-500/30",
          badge: "bg-blue-500",
          text: "text-blue-600 dark:text-blue-400",
        }
      : {
          activeBg: "bg-red-500/5",
          activeRing: "ring-red-500/30",
          badge: "bg-red-500",
          text: "text-red-600 dark:text-red-400",
        };

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex-1 p-4 text-left transition-all min-h-[100px] flex flex-col justify-between",
        isActive
          ? `${sideColor.activeBg} ring-1 ${sideColor.activeRing}`
          : "hover:bg-muted/30"
      )}
    >
      <div className="flex items-center gap-2 mb-2">
        <Badge
          variant="outline"
          className={cn(
            "text-[10px] font-bold gap-1",
            sideColor.text
          )}
        >
          {icon}
          {label}
        </Badge>
        {isActive && (
          <span className="text-[9px] text-muted-foreground animate-pulse">
            selecting...
          </span>
        )}
      </div>

      {deck ? (
        <div>
          <p className={cn("font-semibold text-sm", deck.color)}>
            {deck.name}
          </p>
          <p className="text-[10px] text-muted-foreground leading-tight mt-0.5 line-clamp-2">
            {deck.desc}
          </p>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onClear();
            }}
            className="text-[10px] text-muted-foreground hover:text-destructive transition-colors mt-1 underline"
          >
            Clear
          </button>
        </div>
      ) : (
        <div>
          <p className="text-xs text-muted-foreground italic">
            No deck selected
          </p>
          {extra}
        </div>
      )}
    </button>
  );
}
