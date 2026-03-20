import { useDeckStore } from "@/stores/useDeckStore";
import { Button } from "@/components/ui/button";
import { PrintPickerModal } from "./PrintPickerModal";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  X, Save, FolderOpen, Trash2,
  Pencil, Check, Search, LayoutGrid, List, Layers,
  Plus, Loader2, ChevronDown,
  ClipboardPaste, ClipboardCopy, Palette, Bookmark, BookmarkMinus,
  Group, ArrowUpToLine, ArrowDownToLine,
} from "lucide-react";
import { extractColors } from "@/views/myDecks.utils";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { DeckStats } from "./DeckStats";
import { CardPreview } from "@/components/game/CardPreview";
import { useState, useRef, useEffect, useCallback } from "react";
import { toast } from "sonner";
import type { Card } from "@/types/openmagic";
import { fetchCardCollection, searchCards, getScryfallImageUrl, getCardByName } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import { createEmptyCard, scryfallToXMage } from "@/lib/scryfall.utils";
import { DROP_ZONE } from "@/lib/constants";
import { useDroppable } from "@dnd-kit/core";
import { cn } from "@/lib/utils";
import { DeckListView } from "./DeckListView";
import { CardDetailModal } from "./CardDetailModal";
import { DeckLabelsModal } from "./DeckLabelsModal";
import { SetName } from "./SetSelect";
import { useDeckSelection } from "./useDeckSelection";
import {
  type CardGroup,
  type ViewMode,
  type GroupByMode,
  GROUP_BY_OPTIONS,
  groupCards,
  scryfallCardToPartial,
  exportToArena,
  computeGroupedSections,
  computeGroupedStackColumns,
} from "./deckBuilder.utils";

// ─── Unsaved changes tracking (shared with DeckEditor) ──────────────────────

let _hasUnsavedChanges = false;
const _listeners = new Set<() => void>();

function setUnsavedState(_snapshot: string, current: string) {
  const next = current !== _snapshot;
  if (next !== _hasUnsavedChanges) {
    _hasUnsavedChanges = next;
    _listeners.forEach((fn) => fn());
  }
}

/** Hook to read unsaved changes state from outside DeckBuilder. */
export function useDeckUnsavedChanges(): boolean {
  const [, forceUpdate] = useState(0);
  useEffect(() => {
    const listener = () => forceUpdate((n) => n + 1);
    _listeners.add(listener);
    return () => { _listeners.delete(listener); };
  }, []);
  return _hasUnsavedChanges;
}

function getContrastColor(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255 > 0.5 ? "#000000" : "#ffffff";
}

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
  const [hovered, setHovered] = useState<{ card: Card; x: number; y: number } | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [cardSize, setCardSize] = useState(3);
  const [groupBy, setGroupBy] = useState<GroupByMode>("type");
  const [lastSavedSnapshot, setLastSavedSnapshot] = useState(() => JSON.stringify({ cards: currentDeck.cards, sideboard: currentDeck.sideboard, name: currentDeck.name }));
  const [pendingSwitchAction, setPendingSwitchAction] = useState<(() => void) | null>(null);
  const nameInputRef = useRef<HTMLInputElement>(null);
  const enrichedNamesRef = useRef(new Set<string>());

  const currentSnapshot = JSON.stringify({ cards: currentDeck.cards, sideboard: currentDeck.sideboard, name: currentDeck.name });
  const hasUnsavedChanges = currentSnapshot !== lastSavedSnapshot;

  // Sync shared unsaved state for DeckEditor blocker
  useEffect(() => {
    setUnsavedState(lastSavedSnapshot, currentSnapshot);
  }, [lastSavedSnapshot, currentSnapshot]);

  // Reset snapshot when a deck is loaded
  const deckIdentity = `${currentDeck.name}:${savedDecks.length}`;
  useEffect(() => {
    setLastSavedSnapshot(JSON.stringify({ cards: currentDeck.cards, sideboard: currentDeck.sideboard, name: currentDeck.name }));
  }, [deckIdentity]);

  // Warn on navigation/tab close with unsaved changes
  useEffect(() => {
    if (!hasUnsavedChanges) return;
    const handler = (e: BeforeUnloadEvent) => { e.preventDefault(); };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [hasUnsavedChanges]);

  const { selectedCards, isSelected, toggleCard, clearSelection, selectCards } = useDeckSelection();

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

  // ESC to clear selection
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") clearSelection();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [clearSelection]);

  // Bulk selection actions
  function bulkAction(action: (name: string) => void, message: string) {
    for (const name of selectedCards) action(name);
    clearSelection();
    toast.success(message);
  }

  const handleRemoveSelected = () => bulkAction(
    (name) => currentDeck.cards.filter((c) => c.name.toLowerCase() === name).forEach((c) => removeFromMain(c.id)),
    `Removed ${selectedCards.size} cards`,
  );
  const handleMoveSelectedToSide = () => bulkAction(
    (name) => currentDeck.cards.filter((c) => c.name.toLowerCase() === name).forEach((c) => { removeFromMain(c.id); addToSide({ ...c, id: crypto.randomUUID() }); }),
    `Moved ${selectedCards.size} cards to sideboard`,
  );
  const handleMoveSelectedToMain = () => bulkAction(
    (name) => currentDeck.sideboard.filter((c) => c.name.toLowerCase() === name).forEach((c) => { removeFromSide(c.id); addToMain({ ...c, id: crypto.randomUUID() }); }),
    `Moved ${selectedCards.size} cards to main`,
  );
  const handleTagSelected = (tag: string) => bulkAction((name) => tagCard(name, tag), `Tagged ${selectedCards.size} cards with "${tag}"`);
  const handleUntagSelected = (tag: string) => bulkAction((name) => untagCard(name, tag), `Untagged ${selectedCards.size} cards from "${tag}"`);

  // Tags that any of the selected cards belong to
  const selectedCardTags = (() => {
    if (selectedCards.size === 0 || !currentDeck.cardTags) return [];
    const tags = new Set<string>();
    for (const name of selectedCards) {
      const cardTagList = currentDeck.cardTags[name];
      if (cardTagList) cardTagList.forEach((t) => tags.add(t));
    }
    return [...tags];
  })();

  // Filter
  const filterLc = deckFilter.toLowerCase();
  const filteredMain = filterLc ? currentDeck.cards.filter((c) => c.name.toLowerCase().includes(filterLc)) : currentDeck.cards;
  const filteredSide = filterLc ? currentDeck.sideboard.filter((c) => c.name.toLowerCase().includes(filterLc)) : currentDeck.sideboard;

  // Compute groups
  const { sections: sectionGroups, otherGroups } = computeGroupedSections(filteredMain, groupBy, currentDeck.customTags, currentDeck.cardTags);
  const sideGroups = groupCards(filteredSide);
  const stackColsData = computeGroupedStackColumns(filteredMain, groupBy, currentDeck.customTags, currentDeck.cardTags);

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

  function handleShowInfo(cardName: string) {
    getCardByName(cardName)
      .then((sc) => setDetailCard(sc))
      .catch(() => toast.error(`Could not fetch info for "${cardName}"`));
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

  function handleAddOneToMainByName(cardName: string) {
    const existing = currentDeck.cards.find((c) => c.name === cardName);
    if (existing) addToMain({ ...existing, id: crypto.randomUUID() });
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
    setLastSavedSnapshot(JSON.stringify({ cards: currentDeck.cards, sideboard: currentDeck.sideboard, name: currentDeck.name }));
    toast.success(`Deck "${currentDeck.name}" saved`);
  }

  /** If unsaved changes exist, queue the action behind a confirm dialog. Otherwise run it immediately. */
  function guardUnsaved(action: () => void) {
    if (hasUnsavedChanges) {
      setPendingSwitchAction(() => action);
    } else {
      action();
    }
  }

  return (
    <div className="flex flex-col h-full w-full relative">
      {/* ── Header: deck name + counts + switch ── */}
      <div className="px-3 py-2 border-b shrink-0 flex items-center gap-3">
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
          <div className="flex items-center gap-1.5 flex-1 min-w-0">
            <span className="font-semibold text-sm truncate">{currentDeck.name}</span>
            <Button size="icon" variant="ghost" className="h-6 w-6 shrink-0"
              onClick={() => { setNameInput(currentDeck.name); setEditingName(true); }}>
              <Pencil className="h-3 w-3" />
            </Button>
            {(currentDeck.labels ?? []).map((label) => (
              <span
                key={label.name}
                className="text-[10px] px-1.5 py-0.5 rounded-full font-medium shrink-0 border"
                style={label.color
                  ? { backgroundColor: label.color, color: getContrastColor(label.color), borderColor: label.color }
                  : { backgroundColor: "hsl(var(--muted))", color: "hsl(var(--muted-foreground))", borderColor: "hsl(var(--border))" }
                }
              >
                {label.name}
              </span>
            ))}
            <span className="text-xs text-muted-foreground ml-auto shrink-0">
              {currentDeck.cards.length}
              {currentDeck.sideboard.length > 0 && <span className="text-muted-foreground/50"> · SB:{currentDeck.sideboard.length}</span>}
            </span>
          </div>
        )}

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="sm" variant="ghost" className="h-7 shrink-0 gap-1 text-xs" title="My Decks">
              <FolderOpen className="h-3.5 w-3.5" />
              My Decks
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="min-w-56 max-h-80 overflow-y-auto">
            <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground">My Decks</div>
            <DropdownMenuItem
              onSelect={() => guardUnsaved(() => { clearDeck(); setNameInput("New Deck"); setDeckName("New Deck"); setLastSavedSnapshot(JSON.stringify({ cards: [], sideboard: [], name: "New Deck" })); toast.success("New deck created"); })}
              className="gap-2 text-primary"
            >
              <Plus className="h-3.5 w-3.5 shrink-0" />
              <span className="text-xs font-medium">New Deck</span>
            </DropdownMenuItem>
            {savedDecks.length > 0 && <div className="border-t my-1" />}
            {savedDecks.map((s) => {
              const colors = extractColors(s.deck.cards);
              const isActive = s.deck.name === currentDeck.name;
              return (
                <DropdownMenuItem
                  key={s.id}
                  onSelect={() => guardUnsaved(() => {
                    loadSavedDeck(s.id);
                    toast.success(`Loaded "${s.deck.name}"`);
                  })}
                  className={cn("gap-2", isActive && "bg-muted")}
                >
                  <div className="w-20 shrink-0">
                    {colors.length > 0 ? (
                      <ManaSymbols cost={colors.map((c) => `{${c}}`).join("")} size="sm" />
                    ) : (
                      <span className="text-xs text-muted-foreground">—</span>
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <span className="text-xs truncate block">{s.deck.name}</span>
                    {(s.deck.labels ?? []).length > 0 && (
                      <div className="flex gap-1 mt-0.5 flex-wrap">
                        {(s.deck.labels ?? []).map((label) => (
                          <span
                            key={label.name}
                            className="text-[8px] px-1 py-0 rounded-full font-medium border leading-tight"
                            style={label.color
                              ? { backgroundColor: label.color, color: getContrastColor(label.color), borderColor: label.color }
                              : { backgroundColor: "hsl(var(--muted))", color: "hsl(var(--muted-foreground))", borderColor: "hsl(var(--border))" }
                            }
                          >
                            {label.name}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                  <span className="text-[10px] text-muted-foreground shrink-0">{s.deck.cards.length}</span>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-5 w-5 text-destructive shrink-0 opacity-0 group-hover:opacity-100"
                    title="Delete deck"
                    onClick={(e) => { e.stopPropagation(); deleteSavedDeck(s.id); toast.success(`Deleted "${s.deck.name}"`); }}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </DropdownMenuItem>
              );
            })}
            {savedDecks.length === 0 && (
              <div className="px-3 py-4 text-center text-xs text-muted-foreground">
                No saved decks yet
              </div>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* ── Toolbar: view + search + actions ── */}
      <div className="px-3 py-1.5 border-b shrink-0 flex items-center gap-2">
        {/* View toggle */}
        <div className="flex rounded-md border overflow-hidden shrink-0">
          {([["list", List], ["visual", LayoutGrid], ["stack", Layers]] as const).map(([mode, Icon]) => (
            <button
              key={mode}
              type="button"
              title={mode.charAt(0).toUpperCase() + mode.slice(1)}
              onClick={() => setViewMode(mode)}
              className={cn(
                "p-1.5 transition-colors border-r last:border-r-0",
                viewMode === mode ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground"
              )}
            >
              <Icon className="h-3.5 w-3.5" />
            </button>
          ))}
        </div>

        {/* Group by */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground px-2 py-1 rounded-md border shrink-0 transition-colors">
              <Group className="h-3 w-3" />
              <span>{GROUP_BY_OPTIONS.find((o) => o.value === groupBy)?.label}</span>
              <ChevronDown className="h-3 w-3 opacity-60" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            {GROUP_BY_OPTIONS.map((opt) => (
              <DropdownMenuItem
                key={opt.value}
                onSelect={() => setGroupBy(opt.value)}
                className={cn(groupBy === opt.value && "bg-muted font-medium")}
              >
                {opt.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>

        {viewMode !== "list" && (
          <input
            type="range"
            min={1}
            max={5}
            step={1}
            value={cardSize}
            onChange={(e) => setCardSize(Number(e.target.value))}
            className="w-14 h-1 cursor-pointer accent-primary shrink-0"
            title={`Card size: ${cardSize}`}
          />
        )}

        {/* Filter / Quick add */}
        <div className="flex-1 min-w-0 relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
          <Input className="h-7 text-xs pl-6 pr-6" placeholder="Filter deck…" value={deckFilter} onChange={(e) => setDeckFilter(e.target.value)} />
          {deckFilter && (
            <button type="button" className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground" onClick={() => setDeckFilter("")}>
              <X className="h-3 w-3" />
            </button>
          )}
        </div>
        <div className="shrink-0">
          <QuickCardSearch
            onSelect={(sc) => { addToMain(scryfallToXMage(sc)); toast.success(`Added ${sc.name}`); }}
            onHover={(card, x, y) => setHovered({ card, x, y })}
            onLeave={() => setHovered(null)}
          />
        </div>

        {/* Save button — standalone, always visible */}
        <Button
          size="sm"
          variant="default"
          className="h-7 shrink-0 gap-1 text-xs"
          title={hasUnsavedChanges ? "Save deck (unsaved changes)" : "Save deck"}
          onClick={handleSave}
        >
          <Save className="h-3.5 w-3.5" />
          Save
        </Button>

        {/* Compact action icons */}
        <div className="flex items-center rounded-md border bg-muted/30 p-0.5 shrink-0">
          <Button size="icon" variant="ghost" className="h-7 w-7" title="Import from clipboard" onClick={handleImport}>
            <ClipboardPaste className="h-3.5 w-3.5" />
          </Button>
          <Button size="icon" variant="ghost" className="h-7 w-7" title="Export to clipboard" onClick={handleExport} disabled={currentDeck.cards.length === 0}>
            <ClipboardCopy className="h-3.5 w-3.5" />
          </Button>
          <Button size="icon" variant="ghost" className="h-7 w-7 relative" title="Labels" onClick={() => setLabelsOpen(true)}>
            <Palette className="h-3.5 w-3.5" />
            {(currentDeck.labels?.length ?? 0) > 0 && (
              <span className="absolute -top-0.5 -right-0.5 bg-primary text-primary-foreground text-[8px] rounded-full w-3.5 h-3.5 flex items-center justify-center">
                {currentDeck.labels!.length}
              </span>
            )}
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="icon" variant="ghost" className="h-7 w-7 relative" title="Manage tags">
                <Bookmark className="h-3.5 w-3.5" />
                {(currentDeck.customTags?.length ?? 0) > 0 && (
                  <span className="absolute -top-0.5 -right-0.5 bg-primary text-primary-foreground text-[8px] rounded-full w-3.5 h-3.5 flex items-center justify-center">
                    {currentDeck.customTags!.length}
                  </span>
                )}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
              {(currentDeck.customTags ?? []).map((tag) => (
                <DropdownMenuItem key={tag} className="text-xs justify-between" onSelect={(e) => e.preventDefault()}>
                  <span>{tag}</span>
                  <Button size="icon" variant="ghost" className="h-5 w-5 text-destructive shrink-0" onClick={() => { removeCustomTag(tag); toast.success(`Tag "${tag}" removed`); }}>
                    <X className="h-3 w-3" />
                  </Button>
                </DropdownMenuItem>
              ))}
              {(currentDeck.customTags ?? []).length > 0 && <div className="border-t my-1" />}
              <div className="px-2 py-1.5">
                <Input
                  className="h-7 text-xs"
                  placeholder="New tag…"
                  value={newTagInput}
                  onChange={(e) => setNewTagInput(e.target.value)}
                  onKeyDown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Enter" && newTagInput.trim()) {
                      addCustomTag(newTagInput.trim());
                      toast.success(`Tag "${newTagInput.trim()}" added`);
                      setNewTagInput("");
                    }
                  }}
                  onClick={(e) => e.stopPropagation()}
                />
              </div>
            </DropdownMenuContent>
          </DropdownMenu>
          <Button size="icon" variant="ghost" className="h-7 w-7 text-destructive" title="Clear deck"
            onClick={() => { clearDeck(); toast.success("Deck cleared"); }}>
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
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
          selectedCards={selectedCards}
          onSelectCard={toggleCard}
          onSelectAll={(names) => selectCards(names, true)}
          onShowInfo={handleShowInfo}
        />
      </div>

      {selectedCards.size > 0 && (
        <div className="absolute bottom-0 left-0 right-0 bg-background/95 backdrop-blur border-t border-selection/30 px-4 py-2 flex items-center gap-2 z-50">
          <span className="text-sm font-medium text-selection">{selectedCards.size} card{selectedCards.size !== 1 ? "s" : ""} selected</span>
          <div className="flex-1" />
          <Button size="sm" variant="outline" onClick={handleMoveSelectedToMain}>
            <ArrowUpToLine className="h-3 w-3 mr-1" /> To Main
          </Button>
          <Button size="sm" variant="outline" onClick={handleMoveSelectedToSide}>
            <ArrowDownToLine className="h-3 w-3 mr-1" /> To Sideboard
          </Button>
          {(currentDeck.customTags?.length ?? 0) > 0 && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button size="sm" variant="outline">
                  <Bookmark className="h-3 w-3 mr-1" /> Tag
                  <ChevronDown className="h-3 w-3 ml-1 opacity-60" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {currentDeck.customTags!.map((tag) => (
                  <DropdownMenuItem key={tag} onSelect={() => handleTagSelected(tag)}>
                    <Bookmark className="h-3 w-3 mr-2 text-primary/60" />
                    {tag}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          )}
          {selectedCardTags.length > 0 && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button size="sm" variant="outline">
                  <BookmarkMinus className="h-3 w-3 mr-1" /> Untag
                  <ChevronDown className="h-3 w-3 ml-1 opacity-60" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {selectedCardTags.map((tag) => (
                  <DropdownMenuItem key={tag} onSelect={() => handleUntagSelected(tag)}>
                    <BookmarkMinus className="h-3 w-3 mr-2 text-destructive" />
                    {tag}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          )}
          <Button size="sm" variant="destructive" onClick={handleRemoveSelected}>
            <X className="h-3 w-3 mr-1" /> Remove
          </Button>
          <Button size="sm" variant="ghost" onClick={clearSelection}>
            Clear
          </Button>
        </div>
      )}

      <DeckStats />

      {hovered && <CardPreview card={hovered.card} mouseX={hovered.x} mouseY={hovered.y} />}
      <PrintPickerModal cardName={printPickerCard} onClose={() => setPrintPickerCard(null)} />
      <CardDetailModal
        card={detailCard}
        onClose={() => setDetailCard(null)}
        deckEditorActions={{
          onAddOne: handleAddOneToMainByName,
          onRemoveOne: handleRemoveOneFromMain,
          onPickPrint: (name) => setPrintPickerCard(name),
          onSetCommander: (name) => {
            if (currentDeck.commander?.name === name) {
              removeCommander();
            } else {
              const card = currentDeck.cards.find((c) => c.name === name);
              if (card) setCommander(card);
            }
          },
          isCommander: detailCard ? currentDeck.commander?.name === detailCard.name : false,
          customTags: currentDeck.customTags,
          onTagCard: tagCard,
          onAddTag: addCustomTag,
        }}
      />
      <DeckLabelsModal open={labelsOpen} onClose={() => setLabelsOpen(false)} />

      {/* Unsaved changes confirm dialog (for deck switching) */}
      {pendingSwitchAction && (
        <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-overlay/50 backdrop-blur-sm">
          <div className="bg-card border rounded-xl shadow-xl p-6 max-w-sm space-y-4">
            <h3 className="text-lg font-semibold">Unsaved Changes</h3>
            <p className="text-sm text-muted-foreground">
              You have unsaved changes. Do you want to discard them?
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" size="sm" onClick={() => setPendingSwitchAction(null)}>
                Cancel
              </Button>
              <Button variant="default" size="sm" onClick={() => { handleSave(); pendingSwitchAction(); setPendingSwitchAction(null); }}>
                Save & Switch
              </Button>
              <Button variant="destructive" size="sm" onClick={() => { pendingSwitchAction(); setPendingSwitchAction(null); }}>
                Discard
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
