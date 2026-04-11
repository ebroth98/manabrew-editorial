import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useDeckStore } from "@/stores/useDeckStore";
import { useGameStore } from "@/stores/useGameStore";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { Plus, Search, Swords, Crown, X, Pencil } from "lucide-react";
import { toast } from "sonner";
import { DeckStats } from "@/components/editor/DeckStats";
import { FormatBadge } from "@/components/game/FormatBadge";
import { CreateGameDialog } from "@/components/lobby/CreateGameDialog";
import { DeckCard } from "@/components/deck/DeckCard";
import { DeckListControls } from "@/components/deck/DeckListControls";
import type { Card } from "@/types/openmagic";
import type { CardIdentity } from "@/types/server";
import { fetchCardCollection } from "@/api/scryfall";
import { scryfallCardToPartial } from "@/lib/scryfall.utils";
import { ROUTES, DEFAULT_DECK_NAME } from "@/lib/constants";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import {
  extractColors,
  groupCards,
  categorize,
  applyDeckFilters,
} from "./myDecks.utils";
import type { SortBy } from "./myDecks.utils";
import { getDeckCardNames } from "@/lib/decks";
import { useCardPreview } from "@/hooks/useCardPreview";
import { HoverCardPreview } from "@/components/game/HoverCardPreview";

// ── Component ────────────────────────────────────────────────────────────────

export default function MyDecks() {
  const navigate = useNavigate();
  const startGame = useGameStore((s) => s.startGame);
  const {
    savedDecks,
    loadSavedDeck,
    deleteSavedDeck,
    clearDeck,
    setDeckName,
    enrichSavedDeck,
  } = useDeckStore();
  const [selectedId, setSelectedId] = useState<string | null>(
    savedDecks[0]?.id ?? null,
  );
  const [playDialogOpen, setPlayDialogOpen] = useState(false);
  const [playDeckId, setPlayDeckId] = useState<string | undefined>(undefined);
  const [search, setSearch] = useState("");
  const [formatFilter, setFormatFilter] = useState("");
  const [colorFilter, setColorFilter] = useState<string[]>([]);
  const [sortBy, setSortBy] = useState<SortBy>("name");
  const [cardFilter, setCardFilter] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  
  const preview = useCardPreview();

  const enrichedDecksRef = useRef(new Set<string>());

  function toggleColor(color: string) {
    setColorFilter((prev) =>
      prev.includes(color) ? prev.filter((c) => c !== color) : [...prev, color]
    );
  }

  const { valid: filteredValid, drafts: filteredDrafts } = applyDeckFilters(savedDecks, {
    search, formatFilter, colorFilter, sortBy,
  });

  const selected = savedDecks.find((s) => s.id === selectedId) ?? null;

  // Auto-enrich the selected saved deck when it changes and has missing CMC data
  useEffect(() => {
    if (!selected) return;
    if (enrichedDecksRef.current.has(selected.id)) return;

    const allCards = [
      ...selected.deck.cards,
      ...selected.deck.sideboard,
      ...(selected.deck.attractions ?? []),
      ...(selected.deck.contraptions ?? []),
      ...(selected.deck.schemes ?? []),
      ...(selected.deck.planes ?? []),
    ];
    const toFetch = allCards
      .filter((c) => (c.cmc === undefined || c.cmc === null) && !c.manaCost)
      .map((c) => c.name);

    if (toFetch.length === 0) {
      enrichedDecksRef.current.add(selected.id);
      return;
    }

    enrichedDecksRef.current.add(selected.id);
    const uniqueNames = [...new Set(toFetch)];
    fetchCardCollection(uniqueNames.map((n) => ({ name: n })))
      .then((scryfallMap) => {
        const updates = new Map<string, Partial<Card>>();
        for (const [key, sc] of scryfallMap) {
          updates.set(key, scryfallCardToPartial(sc));
        }
        enrichSavedDeck(selected.id, updates);
      })
      .catch(() => {
        /* silent */
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected?.id]);

  // Reset card filter when switching decks
  useEffect(() => {
    setCardFilter("");
  }, [selectedId]);
  const allMainGroups = selected ? groupCards(selected.deck.cards) : [];
  const sideGroups = selected ? groupCards(selected.deck.sideboard) : [];
  const attractionGroups = selected ? groupCards(selected.deck.attractions ?? []) : [];
  const contraptionGroups = selected ? groupCards(selected.deck.contraptions ?? []) : [];
  const schemeGroups = selected ? groupCards(selected.deck.schemes ?? []) : [];
  const planeGroups = selected ? groupCards(selected.deck.planes ?? []) : [];
  const filterLc = cardFilter.toLowerCase();
  const mainGroups = filterLc
    ? allMainGroups.filter((g) => g.card.name.toLowerCase().includes(filterLc))
    : allMainGroups;
  const categories = categorize(mainGroups);
  const colors = selected ? extractColors(selected.deck.cards) : [];

  function patchSavedDeck(id: string, patch: { name?: string; commander?: Card | undefined }) {
    useDeckStore.setState((state) => ({
      savedDecks: state.savedDecks.map((s) =>
        s.id === id ? { ...s, deck: { ...s.deck, ...patch } } : s,
      ),
    }));
  }

  function setSavedCommander(id: string, card: Card) {
    patchSavedDeck(id, { commander: card });
  }

  function removeSavedCommander(id: string) {
    patchSavedDeck(id, { commander: undefined });
  }

  function handleEdit(id: string) {
    loadSavedDeck(id);
    navigate(ROUTES.DECK_EDITOR, { state: { directToEditor: true } });
  }

  function handlePlay(id: string) {
    setPlayDeckId(id);
    setPlayDialogOpen(true);
  }

  function handleDelete(id: string) {
    deleteSavedDeck(id);
    if (selectedId === id) {
      setSelectedId(savedDecks.find((s) => s.id !== id)?.id ?? null);
    }
    toast.success("Deck deleted");
  }

  function handleNew() {
    clearDeck();
    setDeckName(DEFAULT_DECK_NAME);
    navigate(ROUTES.DECK_EDITOR);
  }

  function startRename(id: string, currentName: string) {
    setEditingId(id);
    setEditName(currentName);
  }

  function confirmRename(id: string) {
    if (!editName.trim()) return;
    const deck = savedDecks.find((s) => s.id === id);
    if (!deck) return;
    patchSavedDeck(id, { name: editName.trim() });
    setEditingId(null);
    toast.success("Deck renamed");
  }

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full">
      {/* ── Left: deck list (Forge-style) ────────────────────────── */}
      <ResizablePanel defaultSize={28} minSize={18} maxSize={300}>
        <div className="h-full flex flex-col border-r">
          <div className="px-3 py-2 border-b flex items-center gap-2 shrink-0">
            <span className="text-sm font-semibold flex-1">My Decks</span>
            <Button size="sm" className="h-7 shrink-0 gap-1" onClick={handleNew}>
              <Plus className="h-3.5 w-3.5" />
              New
            </Button>
          </div>

          <DeckListControls
            search={search}
            onSearchChange={setSearch}
            formatFilter={formatFilter}
            onFormatChange={setFormatFilter}
            colorFilter={colorFilter}
            onColorToggle={toggleColor}
            sortBy={sortBy}
            onSortChange={setSortBy}
          />

          <ScrollArea className="flex-1">
            {filteredValid.length === 0 && filteredDrafts.length === 0 ? (
              <div className="p-6 text-center text-sm text-muted-foreground">
                {savedDecks.length === 0
                  ? "No saved decks. Create one in the Deck Editor."
                  : "No decks match your filters."}
              </div>
            ) : (
              <div className="py-1">
                {filteredValid.map((s) => (
                  <DeckCard
                    key={s.id}
                    deck={s}
                    isSelected={s.id === selectedId}
                    isEditing={editingId === s.id}
                    editName={editName}
                    onSelect={() => setSelectedId(s.id)}
                    onRename={(name) => setEditName(name)}
                    onStartRename={() => startRename(s.id, s.deck.name)}
                    onConfirmRename={() => confirmRename(s.id)}
                    onCancelRename={() => setEditingId(null)}
                    onDelete={() => handleDelete(s.id)}
                    onEditNameChange={setEditName}
                  />
                ))}

                {filteredDrafts.length > 0 && (
                  <>
                    <div className="px-3 pt-3 pb-1 flex items-center gap-2 border-t mt-1">
                      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                        Drafts
                      </span>
                      <span className="text-[10px] text-muted-foreground">
                        ({filteredDrafts.length})
                      </span>
                    </div>
                    {filteredDrafts.map((s) => (
                      <DeckCard
                        key={s.id}
                        deck={s}
                        isSelected={s.id === selectedId}
                        isEditing={editingId === s.id}
                        editName={editName}
                        onSelect={() => setSelectedId(s.id)}
                        onRename={(name) => setEditName(name)}
                        onStartRename={() => startRename(s.id, s.deck.name)}
                        onConfirmRename={() => confirmRename(s.id)}
                        onCancelRename={() => setEditingId(null)}
                        onDelete={() => handleDelete(s.id)}
                        onEditNameChange={setEditName}
                      />
                    ))}
                  </>
                )}
              </div>
            )}
          </ScrollArea>
        </div>
      </ResizablePanel>

      <ResizableHandle withHandle />

      {/* ── Right: selected deck detail ──────────────────────────── */}
      <ResizablePanel minSize={40}>
        {selected ? (
          <div className="h-full flex flex-col overflow-hidden">
            {/* Deck header */}
            <div className="px-4 py-3 border-b flex items-center gap-3 shrink-0">
              <div className="flex-1 min-w-0">
                <h2 className="text-lg font-bold truncate">
                  {selected.deck.name}
                </h2>
                <div className="flex items-center gap-2 flex-wrap text-sm text-muted-foreground">
                  <span>{getDeckCardNames(selected.deck).length} main</span>
                  {selected.deck.sideboard.length > 0 && (
                    <span>{selected.deck.sideboard.length} side</span>
                  )}
                  {(selected.deck.attractions?.length ?? 0) > 0 && (
                    <span>{selected.deck.attractions!.length} attractions</span>
                  )}
                  {(selected.deck.contraptions?.length ?? 0) > 0 && (
                    <span>{selected.deck.contraptions!.length} contraptions</span>
                  )}
                  {(selected.deck.schemes?.length ?? 0) > 0 && (
                    <span>{selected.deck.schemes!.length} schemes</span>
                  )}
                  {(selected.deck.planes?.length ?? 0) > 0 && (
                    <span>{selected.deck.planes!.length} planes</span>
                  )}
                  {colors.length > 0 && (
                    <ManaSymbols cost={colors.map((c) => `{${c}}`).join("")} size="sm" />
                  )}
                  <FormatBadge formatId={selected.deck.format ?? "standard"} />
                </div>
              </div>
              <div className="flex gap-2 shrink-0">
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => handleEdit(selected.id)}
                >
                  <Pencil className="h-3.5 w-3.5 mr-1" />
                  Edit
                </Button>
                <Button size="sm" onClick={() => handlePlay(selected.id)}>
                  <Swords className="h-3.5 w-3.5 mr-1" />
                  Play
                </Button>
              </div>
            </div>

            {/* Mana curve */}
            <DeckStats cards={selected.deck.cards} />

            {/* Card filter input */}
            <div className="px-4 py-1.5 border-b shrink-0">
              <div className="relative">
                <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
                <Input
                  className="h-7 text-xs pl-6 pr-6"
                  placeholder="Filter cards…"
                  value={cardFilter}
                  onChange={(e) => setCardFilter(e.target.value)}
                />
                {cardFilter && (
                  <button
                    type="button"
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                    onClick={() => setCardFilter("")}
                  >
                    <X className="h-3 w-3" />
                  </button>
                )}
              </div>
            </div>

            {/* Card list body — Forge-style grouped by type */}
            <ScrollArea className="flex-1 px-4 py-3">
              <div className="space-y-4">
                {/* Commander section */}
                {selected.deck.commanders?.[0] && !cardFilter && (
                  <div>
                    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">
                      Commander
                    </h3>
                    <div
                      className="flex items-center gap-2 py-0.5 px-1 rounded hover:bg-muted/40 group"
                      onMouseEnter={(e) => preview.handleMouseEnter(selected.deck.commanders![0], e)}
                      onMouseLeave={preview.handleMouseLeave}
                    >
                      <Crown className="h-3 w-3 text-yellow-500 shrink-0" />
                      <span className="text-sm flex-1 truncate">
                        {selected.deck.commanders![0].name}
                      </span>
                      {selected.deck.commanders![0].manaCost && (
                        <ManaSymbols
                          cost={selected.deck.commanders![0].manaCost}
                          size="sm"
                          className="shrink-0"
                        />
                      )}
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-5 w-5 text-destructive opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                        title="Remove commander"
                        onClick={() => removeSavedCommander(selected.id)}
                      >
                        <X className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                )}

                {categories.map(({ label, items }) => (
                  <div key={label}>
                    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">
                      {label} ({items.reduce((a, g) => a + g.count, 0)})
                    </h3>
                    <div className="space-y-0.5">
                      {items.map(({ card, count }) => {
                        const isCommander =
                          selected.deck.commanders?.[0]?.name === card.name;
                        return (
                          <div
                            key={card.name}
                            className="flex items-center gap-2 py-0.5 px-1 rounded hover:bg-muted/40 group"
                            onMouseEnter={(e) => preview.handleMouseEnter(card, e)}
                            onMouseLeave={preview.handleMouseLeave}
                          >
                            <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">
                              {count}
                            </span>
                            <span className="text-sm flex-1 truncate">
                              {card.name}
                            </span>
                            {card.manaCost && (
                              <ManaSymbols
                                cost={card.manaCost}
                                size="sm"
                                className="shrink-0"
                              />
                            )}
                            {card.power && card.toughness && (
                              <Badge
                                variant="outline"
                                className="text-xs h-4 px-1 shrink-0"
                              >
                                {card.power}/{card.toughness}
                              </Badge>
                            )}
                            <Button
                              size="icon"
                              variant="ghost"
                              className={
                                isCommander
                                  ? "h-5 w-5 text-yellow-500 shrink-0"
                                  : "h-5 w-5 text-muted-foreground/40 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                              }
                              title={
                                isCommander
                                  ? "Remove commander"
                                  : "Set as commander"
                              }
                              onClick={() =>
                                isCommander
                                  ? removeSavedCommander(selected.id)
                                  : setSavedCommander(selected.id, card)
                              }
                            >
                              <Crown className="h-3 w-3" />
                            </Button>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ))}

                {/* Supplementary decks */}
                {(sideGroups.length > 0 || attractionGroups.length > 0 || contraptionGroups.length > 0 || schemeGroups.length > 0 || planeGroups.length > 0) && (
                  <>
                    <Separator />
                    {[
                      ["Sideboard", selected.deck.sideboard.length, sideGroups],
                      ["Attractions", selected.deck.attractions?.length ?? 0, attractionGroups],
                      ["Contraptions", selected.deck.contraptions?.length ?? 0, contraptionGroups],
                      ["Schemes", selected.deck.schemes?.length ?? 0, schemeGroups],
                      ["Planes", selected.deck.planes?.length ?? 0, planeGroups],
                    ].map(([label, count, groups]) =>
                      Number(count) === 0 ? null : (
                        <div key={String(label)}>
                          <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">
                            {String(label)} ({Number(count)})
                          </h3>
                          <div className="space-y-0.5">
                            {(groups as typeof sideGroups).map(({ card, count: copies }) => (
                              <div
                                key={`${label}-${card.name}`}
                                className="flex items-center gap-2 py-0.5 px-1 rounded hover:bg-muted/40"
                                onMouseEnter={(e) => preview.handleMouseEnter(card, e)}
                                onMouseLeave={preview.handleMouseLeave}
                              >
                                <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">
                                  {copies}
                                </span>
                                <span className="text-sm flex-1 truncate">
                                  {card.name}
                                </span>
                                {card.manaCost && (
                                  <ManaSymbols
                                    cost={card.manaCost}
                                    size="sm"
                                    className="shrink-0"
                                  />
                                )}
                              </div>
                            ))}
                          </div>
                        </div>
                      )
                    )}
                  </>
                )}
              </div>
            </ScrollArea>
          </div>
        ) : (
          <div className="h-full flex items-center justify-center text-muted-foreground">
            <div className="text-center space-y-2">
              <p className="text-sm">Select a deck to view its contents</p>
              <Button size="sm" variant="outline" onClick={handleNew}>
                <Plus className="h-3.5 w-3.5 mr-1" />
                Create New Deck
              </Button>
            </div>
          </div>
        )}
      </ResizablePanel>

      <HoverCardPreview preview={preview} />

      <CreateGameDialog
        key={playDeckId}
        open={playDialogOpen}
        onOpenChange={setPlayDialogOpen}
        preSelectedDeckId={playDeckId}
        onStart={(deckList: CardIdentity[], formatId, commanderName) => {
          startGame(deckList, formatId, commanderName);
          navigate(ROUTES.PLAY);
        }}
      />
    </ResizablePanelGroup>
  );
}
