import { useEffect, useState } from "react";
import { useLocation } from "react-router-dom";
import { useGameStore } from "@/stores/useGameStore";
import { useDeckStore } from "@/stores/useDeckStore";
import { getDefaultGameRuntime } from "@/game";
import { getDeckFingerprint, serializeDeck } from "@/lib/decks";
import { GAME_FORMATS } from "@/lib/formats";
import { cn } from "@/lib/utils";
import { FormatBadge } from "@/components/game/FormatBadge";
import { DeckSelectionCard } from "@/components/lobby/DeckSelectionCard";
import {
  resolveDeckCoverSource,
  resolvePresetDeckCoverSource,
} from "@/components/deck/deckCover.utils";
import { Button } from "@/components/ui/button";
import { Hand, Search } from "lucide-react";
import type { PresetDeckInfo } from "@/platform";
import type { CardIdentity } from "@/types/server";
import type { Deck, GameView } from "@/types/openmagic";
import Game from "./Game";

interface TabletopLocationState {
  manualTabletop: true;
  playerOrder: string[];
  isHost: boolean;
  startingLife: number;
  myPlayerSlot: string;
  initialGameView?: GameView;
}

interface SelectedDeck {
  id: string;
  name: string;
  desc?: string;
  color?: string;
  deckList: CardIdentity[];
  sourceDeck?: Deck;
  formatId?: string;
  commanderName?: string;
  coverCardName?: string;
}

export default function Tabletop() {
  const location = useLocation();
  const { isGameActive, startManualTabletopGame, startManualRoomClient, setMultiplayerState } =
    useGameStore();

  const { savedDecks, currentDeck } = useDeckStore();
  const [presetDecks, setPresetDecks] = useState<PresetDeckInfo[]>([]);
  const [selectedDeck, setSelectedDeck] = useState<SelectedDeck | null>(null);
  const [selectedFormat, setSelectedFormat] = useState<string>("standard");
  const [deckSearch, setDeckSearch] = useState("");

  const routeState = location.state as TabletopLocationState | null;
  const tabletopState = routeState && "manualTabletop" in routeState ? routeState : null;

  // Handle multiplayer tabletop join from lobby
  useEffect(() => {
    if (!tabletopState?.manualTabletop) return;
    if (tabletopState.isHost) {
      setMultiplayerState(true, true, tabletopState.myPlayerSlot);
      return;
    }
    void startManualRoomClient(tabletopState.myPlayerSlot, tabletopState.initialGameView);
  }, [setMultiplayerState, startManualRoomClient, tabletopState]);

  useEffect(() => {
    const runtime = getDefaultGameRuntime();
    runtime.api
      .getPresetDecks()
      .then(setPresetDecks)
      .catch((e) => console.error("[Tabletop] Failed to load preset decks:", e));
  }, []);

  if (isGameActive) {
    return (
      <div className="h-full min-h-0 no-scrollbar">
        <Game exitTo="/tabletop" />
      </div>
    );
  }

  if (tabletopState?.manualTabletop) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <div className="text-center space-y-2">
          <h1 className="text-2xl font-bold">Starting tabletop room...</h1>
          <p className="text-muted-foreground">Waiting for game synchronization...</p>
        </div>
      </div>
    );
  }

  const searchLower = deckSearch.toLowerCase();

  const currentDeckFingerprint = getDeckFingerprint(currentDeck);
  const distinctSavedDecks = savedDecks.filter(
    (saved) => !saved.deck.draft && getDeckFingerprint(saved.deck) !== currentDeckFingerprint,
  );

  const userDeckEntries: SelectedDeck[] = [
    currentDeck,
    ...distinctSavedDecks.map((saved) => saved.deck),
  ].map((deck, index) => ({
    id: index === 0 ? "current" : distinctSavedDecks[index - 1]!.id,
    name: deck.name,
    deckList: serializeDeck(deck),
    sourceDeck: deck,
    formatId: deck.format ?? "standard",
    commanderName: deck.commanders?.[0]?.name,
  }));

  const formatFilteredUserDecks = userDeckEntries.filter(
    (deck) => deck.formatId === selectedFormat,
  );
  const filteredUserDecks = searchLower
    ? formatFilteredUserDecks.filter((deck) => deck.name.toLowerCase().includes(searchLower))
    : formatFilteredUserDecks;

  const filteredPresetDecks = searchLower
    ? presetDecks.filter(
        (deck) =>
          deck.label.toLowerCase().includes(searchLower) ||
          deck.desc.toLowerCase().includes(searchLower),
      )
    : presetDecks;

  if (selectedDeck && selectedDeck.formatId !== selectedFormat) {
    setSelectedDeck(null);
  }

  const canStart = !!selectedDeck?.sourceDeck && selectedDeck.sourceDeck.cards.length > 0;

  function handleStart() {
    if (!selectedDeck?.sourceDeck || selectedDeck.sourceDeck.cards.length === 0) return;
    void startManualTabletopGame(selectedDeck.sourceDeck);
  }

  function selectPresetDeck(deck: PresetDeckInfo) {
    if (
      selectedFormat === "commander" ||
      selectedFormat === "brawl" ||
      selectedFormat === "oathbreaker"
    ) {
      return;
    }
    setSelectedDeck({
      id: deck.id,
      name: deck.label,
      desc: deck.desc,
      color: deck.color,
      deckList: [{ name: deck.id, setCode: "", section: "main" }],
      formatId: selectedFormat,
      coverCardName: deck.coverCardName,
    });
  }

  function selectUserDeck(entry: SelectedDeck) {
    setSelectedDeck(entry);
  }

  return (
    <div className="flex flex-col h-full">
      <div className="px-4 py-3 border-b flex items-center gap-3 flex-shrink-0">
        <Hand className="h-5 w-5 text-muted-foreground" />
        <div>
          <h1 className="text-lg font-semibold">Tabletop</h1>
          <p className="text-xs text-muted-foreground">
            Free-form sandbox — no rules engine, move cards manually.
          </p>
        </div>
      </div>

      <div className="px-4 py-3 border-b bg-muted/5 flex items-center justify-between gap-3 flex-shrink-0">
        <div>
          <p className="text-xs font-medium">Format</p>
          <p className="text-[10px] text-muted-foreground">
            Choose the format before selecting a deck.
          </p>
        </div>
        <div className="flex gap-1.5 flex-wrap justify-end">
          {GAME_FORMATS.map((format) => (
            <button
              key={format.id}
              type="button"
              onClick={() => setSelectedFormat(format.id)}
              className={cn(
                "rounded-md border px-2 py-1 text-xs transition-colors",
                selectedFormat === format.id
                  ? "border-primary bg-primary/5"
                  : "border-border hover:bg-muted/60",
              )}
            >
              <FormatBadge formatId={format.id} />
            </button>
          ))}
        </div>
      </div>

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

      <div className="flex-1 overflow-y-auto px-4 pb-4 space-y-4">
        {filteredUserDecks.length > 0 && (
          <div>
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-semibold pt-2 pb-1">
              My Decks
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3">
              {filteredUserDecks.map((entry) => {
                const displayCards = [
                  ...(entry.sourceDeck?.cards ?? []),
                  ...(entry.sourceDeck?.commanders ?? []),
                ];
                const cover = entry.sourceDeck
                  ? resolveDeckCoverSource(entry.sourceDeck)
                  : undefined;
                return (
                  <DeckSelectionCard
                    key={entry.id}
                    id={entry.id}
                    name={entry.name}
                    color={entry.color}
                    deckList={entry.deckList}
                    cards={displayCards}
                    cover={cover}
                    labels={entry.sourceDeck?.labels}
                    isPreset={false}
                    isSelected={selectedDeck?.id === entry.id}
                    isPlayerDeck={selectedDeck?.id === entry.id}
                    isOpponentDeck={false}
                    formatId={entry.sourceDeck?.format ?? entry.formatId ?? "standard"}
                    onSelect={() => selectUserDeck(entry)}
                  />
                );
              })}
            </div>
          </div>
        )}

        <div>
          {filteredUserDecks.length > 0 && (
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-semibold pb-1">
              Preset Decks
            </p>
          )}
          {selectedFormat === "commander" ||
          selectedFormat === "brawl" ||
          selectedFormat === "oathbreaker" ? (
            <p className="text-xs text-muted-foreground italic py-4">
              Preset decks are not available for singleton formats. Pick a saved deck above.
            </p>
          ) : filteredPresetDecks.length === 0 ? (
            <p className="text-xs text-muted-foreground italic py-4">No decks match your search.</p>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3 pt-1">
              {filteredPresetDecks.map((deck) => (
                <DeckSelectionCard
                  key={deck.id}
                  id={deck.id}
                  name={deck.label}
                  desc={deck.desc}
                  color={deck.color}
                  deckList={[{ name: deck.id, setCode: "", section: "main" }]}
                  cards={[]}
                  cover={resolvePresetDeckCoverSource(deck.coverCardName)}
                  coverFallbackClassName="absolute inset-0 bg-gradient-to-br from-muted-foreground/10 via-muted/40 to-muted-foreground/20"
                  isPreset={true}
                  isSelected={selectedDeck?.id === deck.id}
                  isPlayerDeck={selectedDeck?.id === deck.id}
                  isOpponentDeck={false}
                  formatId={selectedFormat}
                  onSelect={() => selectPresetDeck(deck)}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="px-4 py-3 border-t flex items-center justify-between bg-muted/10 flex-shrink-0">
        <div className="flex items-center gap-2 text-sm min-w-0">
          {selectedDeck ? (
            <span className="text-xs truncate">
              <span className="font-medium">{selectedDeck.name}</span>
              {!selectedDeck.sourceDeck && (
                <span className="text-muted-foreground ml-1.5 italic text-[10px]">
                  (preset — tabletop requires a saved deck)
                </span>
              )}
            </span>
          ) : (
            <span className="text-xs text-muted-foreground italic">Pick a deck to start</span>
          )}
        </div>
        <Button size="sm" onClick={handleStart} disabled={!canStart} className="gap-1.5">
          <Hand className="h-3.5 w-3.5" />
          Start Tabletop
        </Button>
      </div>
    </div>
  );
}
