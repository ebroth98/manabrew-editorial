import { useDeckStore } from "@/stores/useDeckStore";
import { Button } from "@/components/ui/button";
import { PrintPickerModal } from "./PrintPickerModal";
import { Input } from "@/components/ui/input";
import {
  X, Download, Upload, Save, FolderOpen, Trash2,
  Pencil, Check, Search, LayoutGrid, List, Layers,
  Plus, Loader2, Tag, Tags,
} from "lucide-react";
import { DeckStats } from "./DeckStats";
import { CardPreview } from "@/components/game/CardPreview";
import { useState, useRef, useEffect, useCallback } from "react";
import { toast } from "sonner";
import type { Card } from "@/types/xmage";
import { fetchCardCollection, searchCards, getScryfallImageUrl } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import { createEmptyCard } from "@/lib/scryfall.utils";
import { DROP_ZONE } from "@/lib/constants";
import { useDroppable } from "@dnd-kit/core";
import { cn } from "@/lib/utils";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { DeckListView } from "./DeckListView";
import { CardDetailModal } from "./CardDetailModal";
import { DeckLabelsModal } from "./DeckLabelsModal";
import { SetName } from "./SetSelect";
import {
  type CardGroup,
  type ViewMode,
  MAIN_SECTIONS,
  STACK_TYPE_COLS,
  groupCards,
  scryfallCardToPartial,
  exportToArena,
  computeSectionGroups,
  computeOtherGroups,
  computeStackColumns,
} from "./deckBuilder.utils";

const SIDEBOARD_LINE_REGEX = /^(sideboard|side)$/i;
const DECK_LINE_REGEX = /^(\d+)x?\s+(.+)$/i;

// ─── Quick Search ─────────────────────────────────────────────────────────────

function QuickCardSearch({ onSelect, onHover, onLeave }: {
  onSelect: (card: ScryfallCard) => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ScryfallCard[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();

  const doSearch = useCallback((q: string) => {
    if (q.trim().length < 2) {
      setResults([]);
      setIsOpen(false);
      return;
    }
    setIsLoading(true);
    searchCards(q, 1)
      .then((res) => {
        setResults(res.data.slice(0, 8));
        setIsOpen(true);
      })
      .catch(() => setResults([]))
      .finally(() => setIsLoading(false));
  }, []);

  function handleChange(value: string) {
    setQuery(value);
    clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(value), 400);
  }

  // Close dropdown on outside click
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  return (
    <div ref={containerRef} className="relative">
      <div className="relative">
        <Plus className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
        <Input
          className="h-7 text-xs pl-6 pr-6"
          placeholder="Quick add card…"
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          onFocus={() => { if (results.length > 0) setIsOpen(true); }}
        />
        {isLoading && (
          <Loader2 className="absolute right-2 top-1/2 -translate-y-1/2 h-3 w-3 animate-spin text-muted-foreground" />
        )}
        {!isLoading && query && (
          <button
            type="button"
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            onClick={() => { setQuery(""); setResults([]); setIsOpen(false); }}
          >
            <X className="h-3 w-3" />
          </button>
        )}
      </div>

      {isOpen && results.length > 0 && (
        <div className="absolute z-50 top-full left-0 right-0 mt-1 bg-popover border rounded-md shadow-lg max-h-72 overflow-y-auto">
          {results.map((sc) => {
            const previewCard: Card = {
              ...createEmptyCard(sc.name),
              imageUrl: getScryfallImageUrl(sc),
            };
            return (
            <button
              key={sc.id}
              type="button"
              className="w-full text-left px-2 py-1.5 hover:bg-muted flex items-center gap-2 border-b border-border/30 last:border-0"
              onClick={() => { onSelect(sc); setIsOpen(false); onLeave(); }}
              onMouseEnter={(e) => onHover(previewCard, e.clientX, e.clientY)}
              onMouseMove={(e) => onHover(previewCard, e.clientX, e.clientY)}
              onMouseLeave={onLeave}
            >
              {sc.image_uris?.small && (
                <img
                  src={sc.image_uris.small}
                  alt=""
                  className="w-6 h-6 rounded object-cover object-top shrink-0"
                />
              )}
              <span className="text-xs font-medium flex-1 truncate">{sc.name}</span>
              {sc.mana_cost && (
                <ManaSymbols cost={sc.mana_cost} size="sm" className="shrink-0" />
              )}
              <SetName code={sc.set} className="text-[10px] text-muted-foreground shrink-0" />
            </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ─── Main DeckBuilder Component ───────────────────────────────────────────────

export function DeckBuilder() {
  const [printPickerCard, setPrintPickerCard] = useState<string | null>(null);
  const [detailCard, setDetailCard] = useState<ScryfallCard | null>(null);
  const [labelsOpen, setLabelsOpen] = useState(false);
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
    setCommander,
    removeCommander,
    addCustomTag,
    removeCustomTag,
    tagCard,
    untagCard,
  } = useDeckStore();

  const [editingName, setEditingName] = useState(false);
  const [deckFilter, setDeckFilter] = useState("");
  const [newTagInput, setNewTagInput] = useState("");
  const [nameInput, setNameInput] = useState(currentDeck.name);
  const [loadDialogOpen, setLoadDialogOpen] = useState(false);
  const [hovered, setHovered] = useState<{ card: Card; x: number; y: number } | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [cardSize, setCardSize] = useState(3);
  const nameInputRef = useRef<HTMLInputElement>(null);
  const enrichedNamesRef = useRef(new Set<string>());

  const { setNodeRef: setMainDropRef, isOver: isOverMain } = useDroppable({ id: DROP_ZONE.MAIN });
  const { setNodeRef: setSideDropRef, isOver: isOverSide } = useDroppable({ id: DROP_ZONE.SIDE });

  // Auto-enrich cards missing CMC/mana data
  useEffect(() => {
    const allCards = [...currentDeck.cards, ...currentDeck.sideboard];
    const toFetch = allCards
      .filter((c) => (c.cmc === undefined || c.cmc === null) && !c.manaCost && !enrichedNamesRef.current.has(c.name.toLowerCase()))
      .map((c) => c.name);
    if (toFetch.length === 0) return;
    const uniqueNames = [...new Set(toFetch)];
    uniqueNames.forEach((n) => enrichedNamesRef.current.add(n.toLowerCase()));
    fetchCardCollection(uniqueNames.map((n) => ({ name: n }))).then((scryfallMap) => {
      const updates = new Map<string, Partial<Card>>();
      for (const [key, sc] of scryfallMap) updates.set(key, scryfallCardToPartial(sc));
      enrichDeckCards(updates);
    }).catch((err) => {
      console.warn('[DeckBuilder] Failed to enrich card images:', err);
    });
  }, [currentDeck.cards, currentDeck.sideboard, enrichDeckCards]);

  // Filter
  const filterLc = deckFilter.toLowerCase();
  const filteredMain = filterLc ? currentDeck.cards.filter((c) => c.name.toLowerCase().includes(filterLc)) : currentDeck.cards;
  const filteredSide = filterLc ? currentDeck.sideboard.filter((c) => c.name.toLowerCase().includes(filterLc)) : currentDeck.sideboard;

  // Compute groups
  const sectionGroups = computeSectionGroups(filteredMain, MAIN_SECTIONS);
  const otherGroups = computeOtherGroups(filteredMain, sectionGroups);
  const sideGroups = groupCards(filteredSide);
  const stackColsData = computeStackColumns(filteredMain, STACK_TYPE_COLS);

  // ── Handlers ──

  function handleRemoveOneFromMain(cardName: string) {
    const card = currentDeck.cards.findLast((c) => c.name === cardName);
    if (card) removeFromMain(card.id);
  }

  function handleRemoveAllFromMain(cardName: string) {
    currentDeck.cards.filter((c) => c.name === cardName).forEach((c) => removeFromMain(c.id));
  }

  function handleMoveToSide(cardName: string) {
    const copies = currentDeck.cards.filter((c) => c.name === cardName);
    for (const c of copies) { removeFromMain(c.id); addToSide({ ...c, id: crypto.randomUUID() }); }
    toast.success(`Moved ${cardName} to sideboard`);
  }

  function handleRemoveOneFromSide(cardName: string) {
    const card = currentDeck.sideboard.findLast((c) => c.name === cardName);
    if (card) removeFromSide(card.id);
  }

  function handleMoveToMain(cardName: string) {
    const copies = currentDeck.sideboard.filter((c) => c.name === cardName);
    for (const c of copies) { removeFromSide(c.id); addToMain({ ...c, id: crypto.randomUUID() }); }
    toast.success(`Moved ${cardName} to main`);
  }

  function handleAddOneToMain(group: CardGroup) {
    addToMain({ ...group.card, id: crypto.randomUUID() });
  }

  function confirmName() {
    if (nameInput.trim()) setDeckName(nameInput.trim());
    setEditingName(false);
  }

  function handleExport() {
    const text = exportToArena(currentDeck);
    navigator.clipboard.writeText(text).then(() => toast.success("Deck copied to clipboard"));
  }

  function handleImport() {
    navigator.clipboard.readText().then(async (text) => {
      const lines = text.split("\n").map((l) => l.trim()).filter(Boolean);
      let inSide = false;
      const parsed: { name: string; count: number; side: boolean }[] = [];
      for (const line of lines) {
        if (SIDEBOARD_LINE_REGEX.test(line)) { inSide = true; continue; }
        const match = line.match(DECK_LINE_REGEX);
        if (!match) continue;
        parsed.push({ count: parseInt(match[1], 10), name: match[2].trim(), side: inSide });
      }
      if (parsed.length === 0) { toast.error("No cards found in clipboard"); return; }
      let imported = 0;
      for (const { name, count, side } of parsed) {
        for (let i = 0; i < count; i++) {
          const card = createEmptyCard(name);
          if (side) addToSide(card); else addToMain(card);
          imported++;
        }
      }
      toast.success(`Imported ${imported} cards — fetching data…`);
      try {
        const scryfallMap = await fetchCardCollection(parsed.map((p) => ({ name: p.name })));
        const updates = new Map<string, Partial<Card>>();
        for (const [key, sc] of scryfallMap) updates.set(key, scryfallCardToPartial(sc));
        enrichDeckCards(updates);
        toast.success("Card data loaded from Scryfall");
      } catch { toast.error("Could not fetch card data from Scryfall"); }
    }).catch(() => toast.error("Could not read clipboard"));
  }

  function handleSave() {
    saveCurrentDeck();
    toast.success(`Deck "${currentDeck.name}" saved`);
  }

  return (
    <div className="flex flex-col h-full w-full">
      {/* ── Toolbar ── */}
      <div className="p-2 border-b flex items-center gap-2 flex-wrap shrink-0">
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
            <Button size="icon" variant="ghost" className="h-6 w-6 shrink-0"
              onClick={() => { setNameInput(currentDeck.name); setEditingName(true); }}>
              <Pencil className="h-3 w-3" />
            </Button>
          </div>
        )}

        <div className="flex items-center gap-1 shrink-0 text-xs text-muted-foreground">
          <span>{currentDeck.cards.length}</span>
          <span className="text-muted-foreground/50">/</span>
          <span className="text-muted-foreground/70">SB:{currentDeck.sideboard.length}</span>
        </div>

        {/* View mode toggle */}
        <div className="flex border rounded-md overflow-hidden shrink-0">
          {([["list", List, "List"], ["visual", LayoutGrid, "Visual"], ["stack", Layers, "Stack"]] as const).map(([mode, Icon, label]) => (
            <button
              key={mode}
              type="button"
              title={label}
              onClick={() => setViewMode(mode)}
              className={cn(
                "px-2 py-1 flex items-center gap-1 text-xs transition-colors border-r last:border-r-0",
                viewMode === mode ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground"
              )}
            >
              <Icon className="h-3 w-3" />
              <span className="hidden sm:inline">{label}</span>
            </button>
          ))}
        </div>

        {/* Card size slider */}
        {viewMode !== "list" && (
          <div className="flex items-center gap-1.5 shrink-0">
            <span className="text-xs text-muted-foreground select-none">Size</span>
            <input
              type="range"
              min={1}
              max={5}
              step={1}
              value={cardSize}
              onChange={(e) => setCardSize(Number(e.target.value))}
              className="w-16 h-1 cursor-pointer accent-primary"
              title={`Card size: ${cardSize}`}
            />
          </div>
        )}

        <div className="flex gap-1 shrink-0">
          <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1" onClick={handleImport}>
            <Upload className="h-3 w-3" /> Import
          </Button>
          <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1" onClick={handleExport} disabled={currentDeck.cards.length === 0}>
            <Download className="h-3 w-3" /> Export
          </Button>
          <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1" onClick={handleSave}>
            <Save className="h-3 w-3" /> Save
          </Button>
          <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1" onClick={() => setLabelsOpen(true)}>
            <Tags className="h-3 w-3" /> Labels
            {(currentDeck.labels?.length ?? 0) > 0 && (
              <span className="bg-primary text-primary-foreground text-[9px] rounded-full w-4 h-4 flex items-center justify-center">
                {currentDeck.labels!.length}
              </span>
            )}
          </Button>
          <Dialog open={loadDialogOpen} onOpenChange={setLoadDialogOpen}>
            <DialogTrigger asChild>
              <Button size="sm" variant="outline" className="h-7 px-2 text-xs gap-1">
                <FolderOpen className="h-3 w-3" /> Load
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader><DialogTitle>Saved Decks</DialogTitle></DialogHeader>
              {savedDecks.length === 0 ? (
                <p className="text-sm text-muted-foreground text-center py-4">No saved decks.</p>
              ) : (
                <div className="space-y-2 max-h-80 overflow-y-auto">
                  {savedDecks.map((s) => (
                    <div key={s.id} className="flex items-center gap-2 p-2 rounded border">
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate">{s.deck.name}</p>
                        <p className="text-xs text-muted-foreground">{s.deck.cards.length} cards · {new Date(s.savedAt).toLocaleDateString()}</p>
                      </div>
                      <Button size="sm" variant="secondary" className="h-7 text-xs"
                        onClick={() => { loadSavedDeck(s.id); setLoadDialogOpen(false); toast.success(`Loaded "${s.deck.name}"`); }}>
                        Load
                      </Button>
                      <Button size="icon" variant="ghost" className="h-7 w-7 text-destructive"
                        onClick={() => { deleteSavedDeck(s.id); toast.success("Deck deleted"); }}>
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </DialogContent>
          </Dialog>
          <Button size="sm" variant="ghost" className="h-7 px-2 text-xs text-destructive"
            onClick={() => { clearDeck(); toast.success("Deck cleared"); }}>
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      </div>

      {/* ── Quick add + Deck filter + Tag creation ── */}
      <div className="px-3 py-1.5 border-b shrink-0 flex gap-2">
        <div className="flex-1 min-w-0">
          <QuickCardSearch
            onSelect={(sc) => setDetailCard(sc)}
            onHover={(card, x, y) => setHovered({ card, x, y })}
            onLeave={() => setHovered(null)}
          />
        </div>
        <div className="flex-1 min-w-0 relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
          <Input className="h-7 text-xs pl-6 pr-6" placeholder="Filter deck…" value={deckFilter} onChange={(e) => setDeckFilter(e.target.value)} />
          {deckFilter && (
            <button type="button" className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground" onClick={() => setDeckFilter("")}>
              <X className="h-3 w-3" />
            </button>
          )}
        </div>
        <div className="shrink-0 relative">
          <Tag className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
          <Input
            className="h-7 text-xs pl-6 w-36"
            placeholder="Add tag…"
            value={newTagInput}
            onChange={(e) => setNewTagInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && newTagInput.trim()) {
                addCustomTag(newTagInput.trim());
                setNewTagInput("");
                toast.success(`Tag "${newTagInput.trim()}" added`);
              }
            }}
          />
        </div>
      </div>

      {/* ── Main drop zone (entire scrollable deck area) ── */}
      <div
        ref={setMainDropRef}
        className={cn(
          "flex-1 min-h-0 transition-colors overflow-hidden",
          isOverMain && !isOverSide && "bg-primary/5"
        )}
      >
        <DeckListView
          viewMode={viewMode}
          cardSize={cardSize}
          commander={currentDeck.commander ?? null}
          mainSections={sectionGroups}
          otherGroups={otherGroups}
          sideGroups={sideGroups}
          stackColumns={stackColsData}
          isOverSide={isOverSide}
          setSideDropRef={setSideDropRef}
          onAddOne={handleAddOneToMain}
          onRemoveOne={handleRemoveOneFromMain}
          onRemoveAll={handleRemoveAllFromMain}
          onSetCommander={setCommander}
          onRemoveCommander={removeCommander}
          onMoveToSide={handleMoveToSide}
          onMoveToMain={handleMoveToMain}
          onPickPrint={(name) => setPrintPickerCard(name)}
          onHover={(card, x, y) => setHovered({ card, x, y })}
          onLeave={() => setHovered(null)}
          onAddToSide={(card) => addToSide(card)}
          onRemoveFromSide={handleRemoveOneFromSide}
          totalCards={currentDeck.cards.length}
          customTags={currentDeck.customTags}
          cardTags={currentDeck.cardTags}
          allMainCards={currentDeck.cards}
          onUntagCard={untagCard}
          onRemoveTag={removeCustomTag}
        />
      </div>

      <DeckStats />

      {hovered && <CardPreview card={hovered.card} mouseX={hovered.x} mouseY={hovered.y} />}
      <PrintPickerModal cardName={printPickerCard} onClose={() => setPrintPickerCard(null)} />
      <CardDetailModal card={detailCard} onClose={() => setDetailCard(null)} />
      <DeckLabelsModal open={labelsOpen} onClose={() => setLabelsOpen(false)} />
    </div>
  );
}
