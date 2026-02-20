import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useDeckStore } from "@/stores/useDeckStore";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";
import { Trash2, Pencil, Plus, Search } from "lucide-react";
import { toast } from "sonner";
import { CardPreview } from "@/components/game/CardPreview";
import { DeckStats } from "@/components/editor/DeckStats";
import type { Card } from "@/types/xmage";
import { fetchCardCollection } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";

// ── Helpers ──────────────────────────────────────────────────────────────────

const COLOR_MAP: Record<string, { bg: string; border: string; label: string }> =
  {
    W: { bg: "bg-yellow-50",  border: "border-yellow-300", label: "W" },
    U: { bg: "bg-blue-100",   border: "border-blue-400",   label: "U" },
    B: { bg: "bg-gray-800",   border: "border-gray-600",   label: "B" },
    R: { bg: "bg-red-100",    border: "border-red-400",    label: "R" },
    G: { bg: "bg-green-100",  border: "border-green-400",  label: "G" },
    C: { bg: "bg-zinc-200",   border: "border-zinc-400",   label: "C" },
  };

function extractColors(cards: Card[]): string[] {
  const set = new Set<string>();
  for (const card of cards) {
    for (const ch of card.color ?? "") {
      if (ch in COLOR_MAP) set.add(ch);
    }
    // Detect explicit colourless mana requirement {C}
    if (card.manaCost?.includes("{C}")) set.add("C");
  }
  return ["W", "U", "B", "R", "G", "C"].filter((c) => set.has(c));
}

function ColorPip({ color }: { color: string }) {
  const style = COLOR_MAP[color];
  if (!style) return null;
  return (
    <span
      className={cn(
        "inline-flex items-center justify-center w-5 h-5 rounded-full border text-xs font-bold",
        style.bg,
        style.border,
        color === "B" ? "text-gray-100" : "text-gray-700",
      )}
    >
      {style.label}
    </span>
  );
}

interface CardGroup {
  card: Card;
  count: number;
}

function groupCards(cards: Card[]): CardGroup[] {
  const map = new Map<string, CardGroup>();
  for (const card of cards) {
    const existing = map.get(card.name);
    if (existing) existing.count++;
    else map.set(card.name, { card, count: 1 });
  }
  return Array.from(map.values()).sort((a, b) => {
    const aCmc = a.card.cmc ?? 0;
    const bCmc = b.card.cmc ?? 0;
    if (aCmc !== bCmc) return aCmc - bCmc;
    return a.card.name.localeCompare(b.card.name);
  });
}

// Group card list by type category (Forge-style: Lands, Creatures, Spells)
function categorize(
  groups: CardGroup[],
): { label: string; items: CardGroup[] }[] {
  const lands: CardGroup[] = [];
  const creatures: CardGroup[] = [];
  const other: CardGroup[] = [];
  for (const g of groups) {
    const types = g.card.types ?? [];
    if (types.includes("Land")) lands.push(g);
    else if (types.includes("Creature")) creatures.push(g);
    else other.push(g);
  }
  return [
    { label: "Creatures", items: creatures },
    { label: "Spells & Other", items: other },
    { label: "Lands", items: lands },
  ].filter((c) => c.items.length > 0);
}

// ── Component ────────────────────────────────────────────────────────────────

function scryfallCardToPartial(sc: ScryfallCard): Partial<Card> {
  const SUPERTYPES = new Set([
    "Basic",
    "Legendary",
    "Snow",
    "World",
    "Ongoing",
  ]);
  const [mainPart = "", subPart = ""] = sc.type_line
    .split("—")
    .map((s) => s.trim());
  const mainTokens = mainPart.split(/\s+/).filter(Boolean);
  const supertypes = mainTokens.filter((t) => SUPERTYPES.has(t));
  const types = mainTokens.filter((t) => !SUPERTYPES.has(t));
  const subtypes = subPart ? subPart.split(/\s+/).filter(Boolean) : [];
  const imageUrl =
    sc.image_uris?.normal ??
    (sc as unknown as { card_faces?: { image_uris?: { normal?: string } }[] })
      .card_faces?.[0]?.image_uris?.normal;
  const manaCost =
    sc.mana_cost ??
    (sc as unknown as { card_faces?: { mana_cost?: string }[] }).card_faces?.[0]
      ?.mana_cost ??
    "";
  return {
    manaCost,
    cmc: sc.cmc,
    types,
    subtypes,
    supertypes,
    color: (sc.colors ?? []).join(""),
    power: sc.power,
    toughness: sc.toughness,
    setCode: sc.set,
    cardNumber: sc.collector_number,
    ...(imageUrl ? { imageUrl } : {}),
  };
}

export default function MyDecks() {
  const navigate = useNavigate();
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
  const [search, setSearch] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [hovered, setHovered] = useState<{
    card: Card;
    x: number;
    y: number;
  } | null>(null);
  const enrichedDecksRef = useRef(new Set<string>());

  const filtered = savedDecks.filter((s) =>
    s.deck.name.toLowerCase().includes(search.toLowerCase()),
  );

  const selected = savedDecks.find((s) => s.id === selectedId) ?? null;

  // Auto-enrich the selected saved deck when it changes and has missing CMC data
  useEffect(() => {
    if (!selected) return;
    if (enrichedDecksRef.current.has(selected.id)) return;

    const allCards = [...selected.deck.cards, ...selected.deck.sideboard];
    const toFetch = allCards
      .filter((c) => (c.cmc === undefined || c.cmc === null) && !c.manaCost)
      .map((c) => c.name);

    if (toFetch.length === 0) {
      enrichedDecksRef.current.add(selected.id);
      return;
    }

    enrichedDecksRef.current.add(selected.id);
    const uniqueNames = [...new Set(toFetch)];
    fetchCardCollection(uniqueNames)
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
  const mainGroups = selected ? groupCards(selected.deck.cards) : [];
  const sideGroups = selected ? groupCards(selected.deck.sideboard) : [];
  const categories = categorize(mainGroups);
  const colors = selected ? extractColors(selected.deck.cards) : [];

  function handleEdit(id: string) {
    loadSavedDeck(id);
    navigate("/deck-editor");
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
    setDeckName("New Deck");
    navigate("/deck-editor");
  }

  function startRename(id: string, currentName: string) {
    setEditingId(id);
    setEditName(currentName);
  }

  function confirmRename(id: string) {
    if (!editName.trim()) return;
    const deck = savedDecks.find((s) => s.id === id);
    if (!deck) return;
    // update the saved deck name in place
    useDeckStore.setState((state) => ({
      savedDecks: state.savedDecks.map((s) =>
        s.id === id ? { ...s, deck: { ...s.deck, name: editName.trim() } } : s,
      ),
    }));
    setEditingId(null);
    toast.success("Deck renamed");
  }

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full">
      {/* ── Left: deck list (Forge-style) ────────────────────────── */}
      <ResizablePanel defaultSize={28} minSize={18} maxSize={300}>
        <div className="h-full flex flex-col border-r">
          <div className="p-3 border-b flex items-center gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <Input
                placeholder="Filter decks..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="h-8 pl-7 text-sm"
              />
            </div>
            <Button
              size="sm"
              className="h-8 shrink-0 gap-1"
              onClick={handleNew}
            >
              <Plus className="h-3.5 w-3.5" />
              New
            </Button>
          </div>

          <ScrollArea className="flex-1">
            {filtered.length === 0 ? (
              <div className="p-6 text-center text-sm text-muted-foreground">
                {savedDecks.length === 0
                  ? "No saved decks. Create one in the Deck Editor."
                  : "No decks match your filter."}
              </div>
            ) : (
              <div className="py-1">
                {filtered.map((s) => {
                  const deckColors = extractColors(s.deck.cards);
                  const isSelected = s.id === selectedId;
                  return (
                    <div
                      key={s.id}
                      className={cn(
                        "flex items-center gap-2 px-3 py-2 cursor-pointer group",
                        isSelected
                          ? "bg-secondary text-secondary-foreground"
                          : "hover:bg-muted/60",
                      )}
                      onClick={() => setSelectedId(s.id)}
                    >
                      {/* Color identity pips */}
                      <div className="flex gap-0.5 w-16 shrink-0">
                        {deckColors.length > 0 ? (
                          deckColors.map((c) => <ColorPip key={c} color={c} />)
                        ) : (
                          <span className="text-xs text-muted-foreground italic">
                            —
                          </span>
                        )}
                      </div>

                      {/* Name + count */}
                      <div className="flex-1 min-w-0">
                        {editingId === s.id ? (
                          <Input
                            autoFocus
                            value={editName}
                            className="h-6 text-sm px-1"
                            onChange={(e) => setEditName(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") confirmRename(s.id);
                              if (e.key === "Escape") setEditingId(null);
                            }}
                            onBlur={() => confirmRename(s.id)}
                            onClick={(e) => e.stopPropagation()}
                          />
                        ) : (
                          <p className="text-sm font-medium truncate">
                            {s.deck.name}
                          </p>
                        )}
                        <p className="text-xs text-muted-foreground">
                          {s.deck.cards.length} cards ·{" "}
                          {new Date(s.savedAt).toLocaleDateString()}
                        </p>
                      </div>

                      {/* Actions (visible on hover or selection) */}
                      <div
                        className={cn(
                          "flex gap-1 shrink-0 transition-opacity",
                          isSelected
                            ? "opacity-100"
                            : "opacity-0 group-hover:opacity-100",
                        )}
                      >
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-6 w-6"
                          title="Rename"
                          onClick={(e) => {
                            e.stopPropagation();
                            startRename(s.id, s.deck.name);
                          }}
                        >
                          <Pencil className="h-3 w-3" />
                        </Button>
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-6 w-6 text-destructive hover:text-destructive"
                          title="Delete"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDelete(s.id);
                          }}
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </div>
                  );
                })}
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
                <div className="flex items-center gap-3 text-sm text-muted-foreground">
                  <span>{selected.deck.cards.length} main</span>
                  {selected.deck.sideboard.length > 0 && (
                    <span>{selected.deck.sideboard.length} side</span>
                  )}
                  <div className="flex gap-1 items-center">
                    {colors.map((c) => (
                      <ColorPip key={c} color={c} />
                    ))}
                  </div>
                </div>
              </div>
              <Button size="sm" onClick={() => handleEdit(selected.id)}>
                <Pencil className="h-3.5 w-3.5 mr-1" />
                Edit Deck
              </Button>
            </div>

            {/* Mana curve */}
            <DeckStats cards={selected.deck.cards} />

            {/* Card list body — Forge-style grouped by type */}
            <ScrollArea className="flex-1 px-4 py-3">
              <div className="space-y-4">
                {categories.map(({ label, items }) => (
                  <div key={label}>
                    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">
                      {label} ({items.reduce((a, g) => a + g.count, 0)})
                    </h3>
                    <div className="space-y-0.5">
                      {items.map(({ card, count }) => (
                        <div
                          key={card.name}
                          className="flex items-center gap-2 py-0.5 px-1 rounded hover:bg-muted/40 group"
                          onMouseEnter={(e) =>
                            setHovered({ card, x: e.clientX, y: e.clientY })
                          }
                          onMouseMove={(e) =>
                            setHovered({ card, x: e.clientX, y: e.clientY })
                          }
                          onMouseLeave={() => setHovered(null)}
                        >
                          <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">
                            {count}
                          </span>
                          <span className="text-sm flex-1 truncate">
                            {card.name}
                          </span>
                          {card.manaCost && (
                            <span className="text-xs text-muted-foreground font-mono shrink-0">
                              {card.manaCost}
                            </span>
                          )}
                          {card.power && card.toughness && (
                            <Badge
                              variant="outline"
                              className="text-xs h-4 px-1 shrink-0"
                            >
                              {card.power}/{card.toughness}
                            </Badge>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                ))}

                {/* Sideboard */}
                {sideGroups.length > 0 && (
                  <>
                    <Separator />
                    <div>
                      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">
                        Sideboard ({selected.deck.sideboard.length})
                      </h3>
                      <div className="space-y-0.5">
                        {sideGroups.map(({ card, count }) => (
                          <div
                            key={card.name}
                            className="flex items-center gap-2 py-0.5 px-1 rounded hover:bg-muted/40"
                            onMouseEnter={(e) =>
                              setHovered({ card, x: e.clientX, y: e.clientY })
                            }
                            onMouseMove={(e) =>
                              setHovered({ card, x: e.clientX, y: e.clientY })
                            }
                            onMouseLeave={() => setHovered(null)}
                          >
                            <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">
                              {count}
                            </span>
                            <span className="text-sm flex-1 truncate">
                              {card.name}
                            </span>
                            {card.manaCost && (
                              <span className="text-xs text-muted-foreground font-mono shrink-0">
                                {card.manaCost}
                              </span>
                            )}
                          </div>
                        ))}
                      </div>
                    </div>
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

      {hovered && (
        <CardPreview
          card={hovered.card}
          mouseX={hovered.x}
          mouseY={hovered.y}
        />
      )}
    </ResizablePanelGroup>
  );
}
