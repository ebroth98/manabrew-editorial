import { useState, type ReactNode } from "react";
import { usePresetDecks } from "@/stores/usePresetDecksStore";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { FormatBadge } from "@/components/game/FormatBadge";
import { DeckSelectionCard } from "./DeckSelectionCard";
import { DeckCoverImage } from "@/components/deck/deckCover";
import { DECK_NAME_SHADOW_CLASS, getDeckNameColorClass } from "@/components/deck/deckDisplay.utils";
import { cn } from "@/lib/utils";
import { GAME_FORMATS } from "@/lib/formats";
import { getDeckFingerprint, serializeDeck } from "@/lib/decks";
import { useDeckStore } from "@/stores/useDeckStore";
import type { CardIdentity } from "@/types/server";
import type { Card, Deck } from "@/types/manabrew";
import { Hand, Search, Shuffle, Swords, User, Bot } from "lucide-react";
import { resolveCoverCard } from "../deck/deckCover.utils";

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

interface DeckVsSelectorProps {
  onStart: (
    playerDeck: CardIdentity[],
    opponentDeck: CardIdentity[],
    formatId?: string,
    commanderName?: string,
  ) => void;
  onStartTabletop?: (
    deckList: CardIdentity[],
    formatId?: string,
    commanderName?: string,
    sourceDeck?: Deck,
  ) => void;
}

type PickingSide = "player" | "opponent";
type PlayFormatId = string;

export function DeckVsSelector({ onStart, onStartTabletop }: DeckVsSelectorProps) {
  const presetDecks = usePresetDecks();
  const [playerDeck, setPlayerDeck] = useState<SelectedDeck | null>(null);
  const [opponentDeck, setOpponentDeck] = useState<SelectedDeck | null>(null);
  const [pickingSide, setPickingSide] = useState<PickingSide>("player");
  const [selectedFormat, setSelectedFormat] = useState<PlayFormatId>("standard");
  const [deckSearch, setDeckSearch] = useState("");
  const { savedDecks, currentDeck } = useDeckStore();

  const searchLower = deckSearch.toLowerCase();
  const formatFilteredPresets = presetDecks.filter(
    (deck) => (deck.format ?? "standard") === selectedFormat,
  );
  const filteredDecks = searchLower
    ? formatFilteredPresets.filter(
        (deck) =>
          deck.name.toLowerCase().includes(searchLower) ||
          (deck.description ?? "").toLowerCase().includes(searchLower),
      )
    : formatFilteredPresets;

  const currentDeckFingerprint = getDeckFingerprint(currentDeck);
  const distinctSavedDecks = savedDecks.filter(
    (saved) => !saved.deck.draft && getDeckFingerprint(saved.deck) !== currentDeckFingerprint,
  );

  const currentDeckIsPlayable =
    currentDeck.cards.length > 0 || (currentDeck.commanders?.length ?? 0) > 0;

  const userDeckEntries: SelectedDeck[] = [
    ...(currentDeckIsPlayable ? [currentDeck] : []),
    ...distinctSavedDecks.map((saved) => saved.deck),
  ].map((deck, index) => {
    const id =
      currentDeckIsPlayable && index === 0
        ? "current"
        : distinctSavedDecks[currentDeckIsPlayable ? index - 1 : index]!.id;
    return {
      id,
      name: deck.name,
      deckList: serializeDeck(deck),
      sourceDeck: deck,
      formatId: deck.format ?? "standard",
      commanderName: deck.commanders?.[0]?.name,
    };
  });

  const formatFilteredUserDecks = userDeckEntries.filter(
    (deck) => deck.formatId === selectedFormat,
  );
  const filteredUserDecks = searchLower
    ? formatFilteredUserDecks.filter((deck) => deck.name.toLowerCase().includes(searchLower))
    : formatFilteredUserDecks;

  // Drop selected decks if the format changed and they no longer match.
  if (playerDeck && playerDeck.formatId !== selectedFormat) {
    setPlayerDeck(null);
  }
  if (opponentDeck && opponentDeck.formatId !== selectedFormat) {
    setOpponentDeck(null);
  }

  function assignDeck(selected: SelectedDeck) {
    if (pickingSide === "player") {
      setPlayerDeck(selected);
      if (!opponentDeck) setPickingSide("opponent");
      return;
    }

    setOpponentDeck(selected);
  }

  function selectDeck(deck: Deck) {
    const id = deck.id ?? deck.name;
    assignDeck({
      id,
      name: deck.name,
      desc: deck.description,
      color: deck.color,
      deckList: serializeDeck(deck),
      sourceDeck: deck,
      formatId: selectedFormat,
      commanderName: deck.commanders?.[0]?.name,
      coverCardName: deck.coverCardName,
    });
  }

  function selectUserDeck(entry: SelectedDeck) {
    assignDeck(entry);
  }

  function handleRandomOpponent() {
    if (formatFilteredPresets.length === 0) return;

    const random = formatFilteredPresets[Math.floor(Math.random() * formatFilteredPresets.length)];
    const id = random.id ?? random.name;
    setOpponentDeck({
      id,
      name: random.name,
      desc: random.description,
      color: random.color,
      deckList: serializeDeck(random),
      sourceDeck: random,
      formatId: selectedFormat,
      commanderName: random.commanders?.[0]?.name,
      coverCardName: random.coverCardName,
    });
  }

  function handleFight() {
    if (!playerDeck || !opponentDeck) return;
    onStart(
      playerDeck.deckList,
      opponentDeck.deckList,
      playerDeck.formatId,
      playerDeck.commanderName,
    );
  }

  function handleTabletop() {
    if (!playerDeck || playerDeck.deckList.length === 0) return;
    onStartTabletop?.(
      playerDeck.deckList,
      playerDeck.formatId,
      playerDeck.commanderName,
      playerDeck.sourceDeck,
    );
  }

  const isReady = !!playerDeck && !!opponentDeck;
  const canStartTabletop = !!onStartTabletop && !!playerDeck && playerDeck.deckList.length > 0;

  function resolveDeckCover(deck: SelectedDeck | null): Card | undefined {
    if (!deck) return undefined;
    if (deck.sourceDeck) return resolveCoverCard(deck.sourceDeck);
    return undefined;
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-stretch gap-0 border-b flex-shrink-0 h-72">
        <FighterPanel
          side="player"
          label="YOU"
          icon={<User className="h-4 w-4" />}
          deck={playerDeck}
          cover={resolveDeckCover(playerDeck)}
          isActive={pickingSide === "player"}
          onClick={() => setPickingSide("player")}
          onClear={() => setPlayerDeck(null)}
        />

        <div className="flex flex-col items-center justify-center px-4 flex-shrink-0 bg-muted/30 border-x">
          <span className="text-2xl font-black tracking-tighter text-muted-foreground/60">VS</span>
        </div>

        <FighterPanel
          side="opponent"
          label="AI"
          icon={<Bot className="h-4 w-4" />}
          deck={opponentDeck}
          cover={resolveDeckCover(opponentDeck)}
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

      <div className="px-4 py-2 border-b bg-muted/10 flex items-center gap-2 flex-shrink-0">
        <span className="text-xs text-muted-foreground">Picking for:</span>
        <div className="flex gap-1">
          <button
            type="button"
            onClick={() => setPickingSide("player")}
            className={cn(
              "px-2.5 py-1 rounded-md text-xs font-medium transition-colors",
              pickingSide === "player" ? "ring-1" : "text-muted-foreground hover:bg-muted/60",
            )}
            style={
              pickingSide === "player"
                ? {
                    color: "var(--player-colors-self)",
                    backgroundColor:
                      "color-mix(in srgb, var(--player-colors-self) 10%, transparent)",
                    boxShadow:
                      "inset 0 0 0 1px color-mix(in srgb, var(--player-colors-self) 30%, transparent)",
                  }
                : undefined
            }
          >
            <User className="h-3 w-3 inline mr-1" />
            You
          </button>
          <button
            type="button"
            onClick={() => setPickingSide("opponent")}
            className={cn(
              "px-2.5 py-1 rounded-md text-xs font-medium transition-colors",
              pickingSide === "opponent" ? "ring-1" : "text-muted-foreground hover:bg-muted/60",
            )}
            style={
              pickingSide === "opponent"
                ? {
                    color: "var(--player-colors-opponent1)",
                    backgroundColor:
                      "color-mix(in srgb, var(--player-colors-opponent1) 10%, transparent)",
                    boxShadow:
                      "inset 0 0 0 1px color-mix(in srgb, var(--player-colors-opponent1) 30%, transparent)",
                  }
                : undefined
            }
          >
            <Bot className="h-3 w-3 inline mr-1" />
            AI
          </button>
        </div>
      </div>

      <div className="px-4 py-3 border-b bg-muted/5 flex items-center justify-between gap-3 flex-shrink-0">
        <div>
          <p className="text-xs font-medium">Format</p>
          <p className="text-[10px] text-muted-foreground">
            Choose the game mode before selecting decks.
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
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
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
                const cover = entry.sourceDeck ? resolveCoverCard(entry.sourceDeck) : undefined;
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
                    isSelected={false}
                    isPlayerDeck={playerDeck?.id === entry.id}
                    isOpponentDeck={opponentDeck?.id === entry.id}
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
          {filteredDecks.length === 0 ? (
            <p className="text-xs text-muted-foreground italic py-4">
              No preset decks for this format.
            </p>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3 pt-1">
              {filteredDecks.map((deck) => (
                <DeckSelectionCard
                  key={deck.id ?? deck.name}
                  id={deck.id ?? deck.name}
                  name={deck.name}
                  desc={deck.description}
                  color={deck.color}
                  deckList={serializeDeck(deck)}
                  cards={deck.cards}
                  cover={resolveCoverCard(deck)}
                  coverFallbackClassName="absolute inset-0 bg-gradient-to-br from-muted-foreground/10 via-muted/40 to-muted-foreground/20"
                  isPreset={true}
                  isSelected={false}
                  isPlayerDeck={playerDeck?.id === (deck.id ?? deck.name)}
                  isOpponentDeck={opponentDeck?.id === (deck.id ?? deck.name)}
                  formatId={selectedFormat}
                  onSelect={() => selectDeck(deck)}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="px-4 py-3 border-t flex items-center justify-between bg-muted/10 flex-shrink-0">
        <div className="flex items-center gap-2 text-sm min-w-0">
          {playerDeck ? (
            <span className="text-xs truncate">
              <span className="font-medium" style={{ color: "var(--player-colors-self)" }}>
                {playerDeck.name}
              </span>
              <span className="text-muted-foreground mx-1.5">vs</span>
              {opponentDeck ? (
                <span className="font-medium" style={{ color: "var(--player-colors-opponent1)" }}>
                  {opponentDeck.name}
                </span>
              ) : (
                <span className="text-muted-foreground italic">Pick opponent...</span>
              )}
            </span>
          ) : (
            <span className="text-xs text-muted-foreground italic">Pick your deck to start</span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {onStartTabletop && (
            <Button
              size="sm"
              variant="outline"
              onClick={handleTabletop}
              disabled={!canStartTabletop}
              className="gap-1.5"
            >
              <Hand className="h-3.5 w-3.5" />
              Tabletop
            </Button>
          )}
          <Button size="sm" onClick={handleFight} disabled={!isReady} className="gap-1.5">
            <Swords className="h-3.5 w-3.5" />
            Fight!
          </Button>
        </div>
      </div>
    </div>
  );
}

interface FighterPanelProps {
  side: PickingSide;
  label: string;
  icon: ReactNode;
  deck: SelectedDeck | null;
  cover?: Card;
  isActive: boolean;
  onClick: () => void;
  onClear: () => void;
  extra?: ReactNode;
}

function FighterPanel({
  side,
  label,
  icon,
  deck,
  cover,
  isActive,
  onClick,
  onClear,
  extra,
}: FighterPanelProps) {
  const sideStyleVars: React.CSSProperties = {
    "--fighter-color":
      side === "player" ? "var(--player-colors-self)" : "var(--player-colors-opponent1)",
  } as React.CSSProperties;

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      }}
      className={cn(
        "relative flex-1 h-full text-left transition-all flex flex-col justify-between cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-ring overflow-hidden",
        !cover && (isActive ? "bg-muted/20" : "hover:bg-muted/30"),
      )}
      style={{
        ...sideStyleVars,
        ...(isActive ? { boxShadow: "inset 0 0 0 1px var(--fighter-color)" } : {}),
      }}
    >
      {cover && (
        <>
          <DeckCoverImage cover={cover} alt="" className="scale-110 blur-2xl opacity-60" />
          <DeckCoverImage cover={cover} alt={deck?.name} className="object-contain" />
          <div className="absolute inset-0 bg-gradient-to-t from-black/70 via-black/30 to-transparent z-[1]" />
        </>
      )}

      <div className={cn("relative z-10 p-4 flex flex-col justify-between h-full")}>
        <div className="flex items-center gap-2 mb-2">
          <Badge
            variant="outline"
            className="text-[10px] font-bold gap-1 text-white border"
            style={{
              backgroundColor: "var(--fighter-color)",
              borderColor: "var(--fighter-color)",
            }}
          >
            {icon}
            {label}
          </Badge>
          {isActive && (
            <span
              className={cn(
                "text-[9px] animate-pulse",
                cover ? "text-white/70" : "text-muted-foreground",
              )}
            >
              selecting...
            </span>
          )}
        </div>

        {deck ? (
          <div>
            <p
              className={cn(
                "font-semibold text-sm",
                cover
                  ? "text-white"
                  : deck.sourceDeck
                    ? getDeckNameColorClass(
                        [...deck.sourceDeck.cards, ...(deck.sourceDeck.commanders ?? [])],
                        deck.color,
                      )
                    : deck.color,
                DECK_NAME_SHADOW_CLASS,
              )}
            >
              {deck.name}
            </p>
            <p
              className={cn(
                "text-[10px] leading-tight mt-0.5 line-clamp-2",
                cover ? "text-white/80" : "text-muted-foreground",
              )}
            >
              {deck.desc}
            </p>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onClear();
              }}
              className={cn(
                "text-[10px] transition-colors mt-1 underline",
                cover
                  ? "text-white/70 hover:text-white"
                  : "text-muted-foreground hover:text-destructive",
              )}
            >
              Clear
            </button>
          </div>
        ) : (
          <div>
            <p className="text-xs text-muted-foreground italic">No deck selected</p>
            {extra}
          </div>
        )}
      </div>
    </div>
  );
}
