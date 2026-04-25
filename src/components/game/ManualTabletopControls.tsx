import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { searchCards } from "@/api/scryfall";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { scryfallToXMage } from "@/lib/scryfall.utils";
import { applyManualTabletopAction, type ManualTabletopApi } from "@/game";
import { useGameStore } from "@/stores/useGameStore";
import type { Card, GameView } from "@/types/openmagic";
import type { ScryfallCard } from "@/types/scryfall";
import {
  Archive,
  Hand,
  Heart,
  Plus,
  Search,
  Shuffle,
  Skull,
  Sparkles,
  Sword,
  Trash2,
} from "lucide-react";

interface ManualTabletopControlsProps {
  gameView: GameView;
  api: ManualTabletopApi;
}

function parseStat(value: string | undefined): number | undefined {
  if (value == null) return undefined;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function createManualCard(
  name: string,
  controllerId: string,
  isToken: boolean,
  scryfallCard?: ScryfallCard,
): Card {
  const base = scryfallCard
    ? scryfallToXMage(scryfallCard, `manual-card-${crypto.randomUUID()}`)
    : null;

  return {
    ...(base ?? {}),
    id: `manual-card-${crypto.randomUUID()}`,
    name: base?.name ?? name,
    setCode: base?.setCode ?? "",
    cardNumber: base?.cardNumber ?? "",
    color: base?.color ?? "",
    colorIdentity: base?.colorIdentity,
    manaCost: base?.manaCost ?? "",
    cmc: base?.cmc,
    types: base?.types ?? (isToken ? ["Creature"] : []),
    subtypes: base?.subtypes ?? [],
    supertypes: base?.supertypes ?? [],
    power: base?.power,
    toughness: base?.toughness,
    basePower: parseStat(base?.power),
    baseToughness: parseStat(base?.toughness),
    text: base?.text ?? "",
    imageUrl: base?.imageUrl,
    isPlayable: false,
    isSelected: false,
    isChoosable: false,
    controllerId,
    ownerId: controllerId,
    zoneId: "battlefield",
    tapped: false,
    isToken,
    isDoubleFaced: base?.isDoubleFaced,
  };
}

export function ManualTabletopControls({ gameView, api }: ManualTabletopControlsProps) {
  const [cardName, setCardName] = useState("");
  const [searchResults, setSearchResults] = useState<ScryfallCard[]>([]);
  const [selectedCard, setSelectedCard] = useState<ScryfallCard | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searching, setSearching] = useState(false);
  const [controllerId, setControllerId] = useState(gameView.players[0]?.id ?? "");
  const searchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const searchContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (gameView.players.some((player) => player.id === controllerId)) return;
    setControllerId(gameView.players[0]?.id ?? "");
  }, [controllerId, gameView.players]);

  const selectedPlayer = useMemo(
    () => gameView.players.find((player) => player.id === controllerId),
    [controllerId, gameView.players],
  );

  const applyAction = async (action: Parameters<typeof applyManualTabletopAction>[1]) => {
    const nextView = await applyManualTabletopAction(api, action);
    if (nextView) useGameStore.setState({ gameView: nextView });
  };

  const searchScryfall = useCallback((query: string) => {
    const trimmed = query.trim();
    setSelectedCard(null);
    if (trimmed.length < 2) {
      setSearchResults([]);
      setSearchOpen(false);
      return;
    }

    setSearching(true);
    searchCards(`${trimmed} -is:digital`, 1, "name")
      .then((result) => {
        setSearchResults(result.data.slice(0, 8));
        setSearchOpen(true);
      })
      .catch(() => {
        setSearchResults([]);
        setSearchOpen(false);
      })
      .finally(() => setSearching(false));
  }, []);

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (searchContainerRef.current?.contains(event.target as Node)) return;
      setSearchOpen(false);
    }
    document.addEventListener("mousedown", handlePointerDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      if (searchTimerRef.current) clearTimeout(searchTimerRef.current);
    };
  }, []);

  function handleCardNameChange(value: string) {
    setCardName(value);
    if (searchTimerRef.current) clearTimeout(searchTimerRef.current);
    searchTimerRef.current = setTimeout(() => searchScryfall(value), 300);
  }

  function selectSearchResult(card: ScryfallCard) {
    setSelectedCard(card);
    setCardName(card.name);
    setSearchOpen(false);
  }

  const addPermanent = async (isToken: boolean) => {
    const trimmedName = selectedCard?.name ?? cardName.trim();
    if (!trimmedName || !selectedPlayer) return;
    await applyAction({
      type: isToken ? "createToken" : "createCard",
      controllerId: selectedPlayer.id,
      card: createManualCard(trimmedName, selectedPlayer.id, isToken, selectedCard ?? undefined),
    });
    setCardName("");
    setSelectedCard(null);
    setSearchResults([]);
  };

  const moveCard = (card: Card, zoneId: string) =>
    applyAction({
      type: "moveCard",
      cardId: card.id,
      fromZoneId: card.zoneId,
      toZoneId: zoneId,
    });

  const permanents = gameView.battlefield
    .filter((card) => card.controllerId === controllerId)
    .slice(0, 8);
  const humanPlayerId = gameView.players[0]?.id;

  return (
    <div className="absolute right-2 top-2 z-40 w-[320px] max-w-[calc(100%-1rem)] rounded-md border bg-background/95 shadow-sm backdrop-blur">
      <div className="flex items-center justify-between gap-2 border-b px-3 py-2">
        <Badge variant="outline" className="gap-1.5">
          <Sparkles className="h-3 w-3" />
          Tabletop
        </Badge>
        <span className="text-[10px] text-muted-foreground">
          {gameView.battlefield.length} permanents
        </span>
      </div>

      <div className="space-y-2 p-3">
        <div className="grid grid-cols-2 gap-2">
          {gameView.players.map((player) => (
            <div key={player.id} className="rounded-md border p-2">
              <button
                type="button"
                className={cn(
                  "block max-w-full truncate text-xs font-semibold",
                  controllerId === player.id && "text-primary",
                )}
                onClick={() => setControllerId(player.id)}
              >
                {player.name}
              </button>
              <div className="mt-2 grid grid-cols-[1fr_auto_auto] items-center gap-1">
                <div className="flex items-center gap-1 text-xs">
                  <Heart className="h-3 w-3 text-life" />
                  <span className="tabular-nums">{player.life}</span>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() =>
                    void applyAction({
                      type: "adjustLife",
                      playerId: player.id,
                      delta: -1,
                    })
                  }
                >
                  -
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() =>
                    void applyAction({
                      type: "adjustLife",
                      playerId: player.id,
                      delta: 1,
                    })
                  }
                >
                  +
                </Button>
              </div>
              <div className="mt-1 grid grid-cols-[1fr_auto_auto] items-center gap-1">
                <div className="flex items-center gap-1 text-xs">
                  <Skull className="h-3 w-3 text-poison" />
                  <span className="tabular-nums">{player.poison}</span>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() =>
                    void applyAction({
                      type: "setPoison",
                      playerId: player.id,
                      poison: Math.max(0, player.poison - 1),
                    })
                  }
                >
                  -
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() =>
                    void applyAction({
                      type: "setPoison",
                      playerId: player.id,
                      poison: player.poison + 1,
                    })
                  }
                >
                  +
                </Button>
              </div>
              <div className="mt-2 grid grid-cols-3 gap-1">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="h-7 gap-1 px-1.5 text-[10px]"
                  disabled={player.libraryCount <= 0}
                  title="Draw a card"
                  onClick={() =>
                    void applyAction({
                      type: "drawLibraryCard",
                      playerId: player.id,
                    })
                  }
                >
                  <Hand className="h-3 w-3" />
                  {player.libraryCount}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="h-7 w-full"
                  disabled={player.libraryCount <= 0}
                  title="Put top library card onto battlefield"
                  onClick={() =>
                    void applyAction({
                      type: "putLibraryCardOntoBattlefield",
                      playerId: player.id,
                    })
                  }
                >
                  <Sword className="h-3 w-3" />
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="h-7 w-full"
                  disabled={player.libraryCount < 2}
                  title="Shuffle library"
                  onClick={() =>
                    void applyAction({
                      type: "shuffleLibrary",
                      playerId: player.id,
                    })
                  }
                >
                  <Shuffle className="h-3 w-3" />
                </Button>
              </div>
            </div>
          ))}
        </div>

        <div className="grid grid-cols-[1fr_auto_auto] gap-1.5">
          <div ref={searchContainerRef} className="relative">
            <Search className="pointer-events-none absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={cardName}
              onChange={(event) => handleCardNameChange(event.target.value)}
              onFocus={() => searchResults.length > 0 && setSearchOpen(true)}
              placeholder="Search Scryfall"
              className="h-8 pl-7 text-xs"
            />
            {searchOpen && (
              <div className="absolute left-0 right-0 top-[calc(100%+0.25rem)] z-50 max-h-56 overflow-y-auto rounded-md border bg-popover p-1 shadow-lg">
                {searching && searchResults.length === 0 ? (
                  <div className="px-2 py-1.5 text-xs text-muted-foreground">Searching...</div>
                ) : searchResults.length === 0 ? (
                  <div className="px-2 py-1.5 text-xs text-muted-foreground">No cards found</div>
                ) : (
                  searchResults.map((card) => (
                    <button
                      key={card.id}
                      type="button"
                      className="flex w-full items-center justify-between gap-2 rounded px-2 py-1.5 text-left text-xs hover:bg-muted"
                      onClick={() => selectSearchResult(card)}
                    >
                      <span className="truncate font-medium">{card.name}</span>
                      <span className="shrink-0 text-[10px] uppercase text-muted-foreground">
                        {card.set}
                      </span>
                    </button>
                  ))
                )}
              </div>
            )}
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="gap-1"
            disabled={!cardName.trim() || !selectedPlayer}
            onClick={() => void addPermanent(false)}
          >
            <Plus className="h-3.5 w-3.5" />
            Card
          </Button>
          <Button
            type="button"
            size="sm"
            className="gap-1"
            disabled={!cardName.trim() || !selectedPlayer}
            onClick={() => void addPermanent(true)}
          >
            <Plus className="h-3.5 w-3.5" />
            Token
          </Button>
        </div>

        {permanents.length > 0 && (
          <div className="space-y-1.5">
            <div className="flex items-center justify-between text-[10px] uppercase tracking-wide text-muted-foreground">
              <span>Battlefield</span>
              <span>{permanents.length}</span>
            </div>
            <div className="max-h-48 space-y-1 overflow-y-auto pr-1">
              {permanents.map((card) => (
                <div
                  key={card.id}
                  className="grid grid-cols-[1fr_repeat(4,auto)] items-center gap-1 rounded-md border px-2 py-1.5"
                >
                  <span className="truncate text-xs font-medium">{card.name}</span>
                  <Button
                    type="button"
                    variant={card.tapped ? "secondary" : "outline"}
                    size="icon"
                    className="h-6 w-6"
                    title={card.tapped ? "Untap" : "Tap"}
                    onClick={() =>
                      void applyAction({
                        type: "tapCard",
                        cardId: card.id,
                        tapped: !card.tapped,
                      })
                    }
                  >
                    <Sword className="h-3 w-3" />
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    className="h-6 w-6"
                    title="Move to hand"
                    onClick={() => void moveCard(card, "hand")}
                  >
                    <Hand className="h-3 w-3" />
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    className="h-6 w-6"
                    title="Move to exile"
                    onClick={() =>
                      void moveCard(
                        card,
                        card.controllerId === humanPlayerId ? "exile" : "opponentExile",
                      )
                    }
                  >
                    <Archive className="h-3 w-3" />
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    className="h-6 w-6"
                    title="Move to graveyard"
                    onClick={() =>
                      void moveCard(
                        card,
                        card.controllerId === humanPlayerId ? "graveyard" : "opponentGraveyard",
                      )
                    }
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
