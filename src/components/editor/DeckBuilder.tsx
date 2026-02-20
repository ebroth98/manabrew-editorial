import { useDeckStore } from "@/stores/useDeckStore";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { X, Minus, Plus, Download, Upload, Save, FolderOpen, Trash2, Pencil, Check } from "lucide-react";
import { DeckStats } from "./DeckStats";
import { CardPreview } from "@/components/game/CardPreview";
import { useState, useRef, useEffect } from "react";
import { toast } from "sonner";
import type { Card } from "@/types/xmage";
import { fetchCardCollection } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

interface CardGroup {
  card: Card;
  count: number;
}

function scryfallCardToPartial(sc: ScryfallCard): Partial<Card> {
  const SUPERTYPES = new Set(["Basic", "Legendary", "Snow", "World", "Ongoing"]);
  const [mainPart = "", subPart = ""] = sc.type_line.split("—").map((s) => s.trim());
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
    (sc as unknown as { card_faces?: { mana_cost?: string }[] })
      .card_faces?.[0]?.mana_cost ??
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

function groupCards(cards: Card[]): CardGroup[] {
  const map = new Map<string, CardGroup>();
  for (const card of cards) {
    const key = card.name;
    const existing = map.get(key);
    if (existing) {
      existing.count++;
    } else {
      map.set(key, { card, count: 1 });
    }
  }
  return Array.from(map.values()).sort((a, b) => {
    // Sort by type then name
    const aIsLand = a.card.types.includes("Land");
    const bIsLand = b.card.types.includes("Land");
    if (aIsLand !== bIsLand) return aIsLand ? 1 : -1;
    const aCmc = a.card.cmc ?? 0;
    const bCmc = b.card.cmc ?? 0;
    if (aCmc !== bCmc) return aCmc - bCmc;
    return a.card.name.localeCompare(b.card.name);
  });
}

function exportToArena(deck: { name: string; cards: Card[]; sideboard: Card[] }): string {
  const mainGroups = groupCards(deck.cards);
  const sideGroups = groupCards(deck.sideboard);
  const lines: string[] = [];
  for (const g of mainGroups) {
    lines.push(`${g.count} ${g.card.name}`);
  }
  if (sideGroups.length > 0) {
    lines.push("");
    lines.push("Sideboard");
    for (const g of sideGroups) {
      lines.push(`${g.count} ${g.card.name}`);
    }
  }
  return lines.join("\n");
}

export function DeckBuilder() {
  const {
    currentDeck,
    savedDecks,
    removeFromMain,
    removeFromSide,
    addToMain,
    addToSide,
    setDeckName,
    clearDeck,
    saveCurrentDeck,
    loadSavedDeck,
    deleteSavedDeck,
    enrichDeckCards,
  } = useDeckStore();

  const [editingName, setEditingName] = useState(false);
  const [nameInput, setNameInput] = useState(currentDeck.name);
  const [loadDialogOpen, setLoadDialogOpen] = useState(false);
  const [hovered, setHovered] = useState<{ card: Card; x: number; y: number } | null>(null);
  const nameInputRef = useRef<HTMLInputElement>(null);
  // Track which card names we've already attempted to enrich (avoids re-fetching)
  const enrichedNamesRef = useRef(new Set<string>());

  // Auto-enrich cards that are missing CMC/mana data (e.g. loaded from localStorage)
  useEffect(() => {
    const allCards = [...currentDeck.cards, ...currentDeck.sideboard];
    const toFetch = allCards
      .filter(
        (c) =>
          (c.cmc === undefined || c.cmc === null) &&
          !c.manaCost &&
          !enrichedNamesRef.current.has(c.name.toLowerCase())
      )
      .map((c) => c.name);

    if (toFetch.length === 0) return;

    // Mark as attempted so we don't loop
    const uniqueNames = [...new Set(toFetch)];
    uniqueNames.forEach((n) => enrichedNamesRef.current.add(n.toLowerCase()));

    fetchCardCollection(uniqueNames).then((scryfallMap) => {
      const updates = new Map<string, Partial<Card>>();
      for (const [key, sc] of scryfallMap) {
        updates.set(key, scryfallCardToPartial(sc));
      }
      enrichDeckCards(updates);
    }).catch(() => {/* silent — curve will show unknown */});
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentDeck.cards, currentDeck.sideboard]);

  const mainGroups = groupCards(currentDeck.cards);
  const sideGroups = groupCards(currentDeck.sideboard);

  function confirmName() {
    if (nameInput.trim()) setDeckName(nameInput.trim());
    setEditingName(false);
  }

  function handleRemoveOneFromMain(cardName: string) {
    const cards = currentDeck.cards;
    let idx = -1;
    for (let i = cards.length - 1; i >= 0; i--) {
      if (cards[i].name === cardName) { idx = i; break; }
    }
    if (idx !== -1) removeFromMain(cards[idx].id);
  }

  function handleRemoveAllFromMain(cardName: string) {
    const ids = currentDeck.cards.filter((c) => c.name === cardName).map((c) => c.id);
    ids.forEach((id) => removeFromMain(id));
  }

  function handleRemoveOneFromSide(cardName: string) {
    const side = currentDeck.sideboard;
    let idx = -1;
    for (let i = side.length - 1; i >= 0; i--) {
      if (side[i].name === cardName) { idx = i; break; }
    }
    if (idx !== -1) removeFromSide(side[idx].id);
  }

  function handleAddOneToMain(group: CardGroup) {
    addToMain({ ...group.card, id: crypto.randomUUID() });
  }

  function handleExport() {
    const text = exportToArena(currentDeck);
    navigator.clipboard.writeText(text).then(() => {
      toast.success("Deck copied to clipboard");
    });
  }

  function handleImport() {
    navigator.clipboard.readText().then(async (text) => {
      // Parse "N CardName" or "Nx CardName" format (Arena / MTGO)
      const lines = text.split("\n").map((l) => l.trim()).filter(Boolean);
      let inSide = false;
      const parsed: { name: string; count: number; side: boolean }[] = [];

      for (const line of lines) {
        if (/^(sideboard|side)$/i.test(line)) { inSide = true; continue; }
        const match = line.match(/^(\d+)x?\s+(.+)$/i);
        if (!match) continue;
        parsed.push({ count: parseInt(match[1], 10), name: match[2].trim(), side: inSide });
      }

      if (parsed.length === 0) { toast.error("No cards found in clipboard"); return; }

      // Add placeholder cards immediately so the list populates fast
      let imported = 0;
      for (const { name, count, side } of parsed) {
        for (let i = 0; i < count; i++) {
          const card: Card = {
            id: crypto.randomUUID(),
            name,
            setCode: "",
            cardNumber: "",
            color: "",
            manaCost: "",
            types: [],
            subtypes: [],
            supertypes: [],
            text: "",
            isPlayable: true,
            isSelected: false,
            isChoosable: true,
            controllerId: "",
            ownerId: "",
            zoneId: "",
          };
          if (side) addToSide(card); else addToMain(card);
          imported++;
        }
      }
      toast.success(`Imported ${imported} cards — fetching data…`);

      // Enrich with real Scryfall data
      const allNames = parsed.map((p) => p.name);
      try {
        const scryfallMap = await fetchCardCollection(allNames);
        const updates = new Map<string, Partial<Card>>();
        for (const [key, sc] of scryfallMap) {
          updates.set(key, scryfallCardToPartial(sc));
        }
        enrichDeckCards(updates);
        toast.success("Card data loaded from Scryfall");
      } catch {
        toast.error("Could not fetch card data from Scryfall");
      }
    }).catch(() => {
      toast.error("Could not read clipboard");
    });
  }

  function handleSave() {
    saveCurrentDeck();
    toast.success(`Deck "${currentDeck.name}" saved`);
  }

  return (
    <div className="flex flex-col h-full w-full">
      {/* Toolbar */}
      <div className="p-3 border-b flex items-center gap-2 flex-wrap">
        {editingName ? (
          <div className="flex items-center gap-1 flex-1 min-w-0">
            <Input
              ref={nameInputRef}
              className="h-7 text-sm font-semibold max-w-48"
              value={nameInput}
              onChange={(e) => setNameInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") confirmName();
                if (e.key === "Escape") { setEditingName(false); setNameInput(currentDeck.name); }
              }}
              autoFocus
            />
            <Button size="icon" variant="ghost" className="h-7 w-7" onClick={confirmName}>
              <Check className="h-3 w-3" />
            </Button>
          </div>
        ) : (
          <div className="flex items-center gap-1 flex-1 min-w-0">
            <span className="font-semibold text-sm truncate">{currentDeck.name}</span>
            <Button
              size="icon"
              variant="ghost"
              className="h-6 w-6 shrink-0"
              onClick={() => { setNameInput(currentDeck.name); setEditingName(true); }}
            >
              <Pencil className="h-3 w-3" />
            </Button>
          </div>
        )}

        <div className="flex items-center gap-1 shrink-0 text-xs text-muted-foreground">
          <span>{currentDeck.cards.length}</span>
          <span className="text-muted-foreground/50">/</span>
          <span className="text-muted-foreground/70">SB:{currentDeck.sideboard.length}</span>
        </div>

        <div className="flex gap-1 shrink-0">
          <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1" onClick={handleImport}>
            <Upload className="h-3 w-3" />
            Import
          </Button>
          <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1" onClick={handleExport} disabled={currentDeck.cards.length === 0}>
            <Download className="h-3 w-3" />
            Export
          </Button>
          <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1" onClick={handleSave}>
            <Save className="h-3 w-3" />
            Save
          </Button>
          <Dialog open={loadDialogOpen} onOpenChange={setLoadDialogOpen}>
            <DialogTrigger asChild>
              <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1">
                <FolderOpen className="h-3 w-3" />
                Load
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Saved Decks</DialogTitle>
              </DialogHeader>
              {savedDecks.length === 0 ? (
                <p className="text-sm text-muted-foreground text-center py-4">No saved decks.</p>
              ) : (
                <div className="space-y-2 max-h-80 overflow-y-auto">
                  {savedDecks.map((s) => (
                    <div key={s.id} className="flex items-center gap-2 p-2 rounded border">
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate">{s.deck.name}</p>
                        <p className="text-xs text-muted-foreground">
                          {s.deck.cards.length} cards · {new Date(s.savedAt).toLocaleDateString()}
                        </p>
                      </div>
                      <Button
                        size="sm"
                        variant="secondary"
                        className="h-7 text-xs"
                        onClick={() => { loadSavedDeck(s.id); setLoadDialogOpen(false); toast.success(`Loaded "${s.deck.name}"`); }}
                      >
                        Load
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-7 w-7 text-destructive"
                        onClick={() => { deleteSavedDeck(s.id); toast.success("Deck deleted"); }}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </DialogContent>
          </Dialog>
          <Button size="sm" variant="ghost" className="h-7 px-2 text-xs text-destructive" onClick={() => { clearDeck(); toast.success("Deck cleared"); }}>
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1 px-3 py-2">
        <div className="space-y-4">
          {/* Mainboard */}
          <div>
            <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1">
              Mainboard ({currentDeck.cards.length})
            </h3>
            {mainGroups.length === 0 ? (
              <p className="text-xs text-muted-foreground italic px-1">No cards yet. Search and add cards above.</p>
            ) : (
              <div className="space-y-0.5">
                {mainGroups.map((g) => (
                  <div
                    key={g.card.name}
                    className="flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5"
                    onMouseEnter={(e) => setHovered({ card: g.card, x: e.clientX, y: e.clientY })}
                    onMouseMove={(e) => setHovered({ card: g.card, x: e.clientX, y: e.clientY })}
                    onMouseLeave={() => setHovered(null)}
                  >
                    <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">{g.count}</span>
                    <span className="text-sm flex-1 truncate" title={g.card.name}>{g.card.name}</span>
                    {g.card.manaCost && (
                      <span className="text-xs text-muted-foreground shrink-0 font-mono">{g.card.manaCost}</span>
                    )}
                    <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-5 w-5"
                        onClick={() => handleAddOneToMain(g)}
                      >
                        <Plus className="h-3 w-3" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-5 w-5"
                        onClick={() => handleRemoveOneFromMain(g.card.name)}
                      >
                        <Minus className="h-3 w-3" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-5 w-5 text-destructive"
                        onClick={() => handleRemoveAllFromMain(g.card.name)}
                      >
                        <X className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Sideboard */}
          {sideGroups.length > 0 && (
            <div>
              <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1">
                Sideboard ({currentDeck.sideboard.length})
              </h3>
              <div className="space-y-0.5">
                {sideGroups.map((g) => (
                  <div
                    key={g.card.name}
                    className="flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5"
                    onMouseEnter={(e) => setHovered({ card: g.card, x: e.clientX, y: e.clientY })}
                    onMouseMove={(e) => setHovered({ card: g.card, x: e.clientX, y: e.clientY })}
                    onMouseLeave={() => setHovered(null)}
                  >
                    <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">{g.count}</span>
                    <span className="text-sm flex-1 truncate">{g.card.name}</span>
                    {g.card.manaCost && (
                      <span className="text-xs text-muted-foreground shrink-0 font-mono">{g.card.manaCost}</span>
                    )}
                    <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-5 w-5 text-destructive"
                        onClick={() => handleRemoveOneFromSide(g.card.name)}
                      >
                        <X className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </ScrollArea>

      <DeckStats />

      {hovered && (
        <CardPreview card={hovered.card} mouseX={hovered.x} mouseY={hovered.y} />
      )}
    </div>
  );
}
