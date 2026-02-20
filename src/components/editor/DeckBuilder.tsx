import { useDeckStore } from "@/stores/useDeckStore";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  X, Minus, Plus, Download, Upload, Save, FolderOpen, Trash2,
  Pencil, Check, Crown, Search, LayoutGrid, List, Layers,
  ChevronDown, ChevronRight,
} from "lucide-react";
import { DeckStats } from "./DeckStats";
import { CardPreview } from "@/components/game/CardPreview";
import { useState, useRef, useEffect } from "react";
import { toast } from "sonner";
import type { Card } from "@/types/xmage";
import { fetchCardCollection } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import { useDroppable, useDraggable } from "@dnd-kit/core";
import { cn } from "@/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

// ─── Types ────────────────────────────────────────────────────────────────────

interface CardGroup {
  card: Card;
  count: number;
}

type ViewMode = "list" | "visual" | "stack";

// ─── Helpers ──────────────────────────────────────────────────────────────────

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
    (sc as unknown as { card_faces?: { mana_cost?: string }[] }).card_faces?.[0]?.mana_cost ??
    "";
  return {
    manaCost, cmc: sc.cmc, types, subtypes, supertypes,
    color: (sc.colors ?? []).join(""),
    power: sc.power, toughness: sc.toughness,
    setCode: sc.set, cardNumber: sc.collector_number,
    ...(imageUrl ? { imageUrl } : {}),
  };
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

function exportToArena(deck: { name: string; cards: Card[]; sideboard: Card[] }): string {
  const mainGroups = groupCards(deck.cards);
  const sideGroups = groupCards(deck.sideboard);
  const lines: string[] = [];
  for (const g of mainGroups) lines.push(`${g.count} ${g.card.name}`);
  if (sideGroups.length > 0) {
    lines.push("");
    lines.push("Sideboard");
    for (const g of sideGroups) lines.push(`${g.count} ${g.card.name}`);
  }
  return lines.join("\n");
}

// ─── Section definitions ──────────────────────────────────────────────────────

const MAIN_SECTIONS = [
  { id: "creatures",     label: "Creatures",      filter: (t: string[]) => t.includes("Creature") },
  { id: "planeswalkers", label: "Planeswalkers",   filter: (t: string[]) => t.includes("Planeswalker") && !t.includes("Creature") },
  { id: "instants",      label: "Instants",        filter: (t: string[]) => t.includes("Instant") },
  { id: "sorceries",     label: "Sorceries",       filter: (t: string[]) => t.includes("Sorcery") },
  { id: "enchantments",  label: "Enchantments",    filter: (t: string[]) => t.includes("Enchantment") && !t.includes("Creature") },
  { id: "artifacts",     label: "Artifacts",       filter: (t: string[]) => t.includes("Artifact") && !t.includes("Creature") },
  { id: "lands",         label: "Lands",           filter: (t: string[]) => t.includes("Land") },
];

// ─── Stack view (Moxfield-style type columns) ────────────────────────────────
// Cards are grouped into type columns (Creatures, Instants, …, Lands).
// Within each column, different cards are stacked vertically: each card is
// offset by `peek` px, so only its top strip is visible — except the last card
// which is fully shown. Higher z-index cards appear "above" earlier ones.

const CARD_WIDTH_MAP: Record<number, number> = { 1: 75, 2: 95, 3: 115, 4: 140, 5: 170 };

const STACK_TYPE_COLS = [
  { id: "creatures",     label: "Creatures",     filter: (t: string[]) => t.includes("Creature") },
  { id: "instants",      label: "Instants",      filter: (t: string[]) => t.includes("Instant") },
  { id: "sorceries",     label: "Sorceries",     filter: (t: string[]) => t.includes("Sorcery") },
  { id: "enchantments",  label: "Enchantments",  filter: (t: string[]) => t.includes("Enchantment") && !t.includes("Creature") },
  { id: "artifacts",     label: "Artifacts",     filter: (t: string[]) => t.includes("Artifact") && !t.includes("Creature") },
  { id: "planeswalkers", label: "Planeswalkers", filter: (t: string[]) => t.includes("Planeswalker") && !t.includes("Creature") },
  { id: "lands",         label: "Lands",         filter: (t: string[]) => t.includes("Land") },
];

function StackColumn({
  label, groups, cardWidth,
  onAddOne, onRemoveOne, onHover, onLeave,
}: {
  label: string; groups: CardGroup[]; cardWidth: number;
  onAddOne: (g: CardGroup) => void; onRemoveOne: (name: string) => void;
  onHover: (card: Card, x: number, y: number) => void; onLeave: () => void;
}) {
  const cardHeight = Math.round(cardWidth * 1.4);
  // Peek = top strip of each card that stays visible above the next card
  const peek = Math.round(cardHeight * 0.28);
  const totalHeight = groups.length > 0 ? peek * (groups.length - 1) + cardHeight : 0;
  const count = groups.reduce((s, g) => s + g.count, 0);

  return (
    <div className="shrink-0 flex flex-col" style={{ width: cardWidth }}>
      <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 truncate">
        {label} <span className="font-normal opacity-60">({count})</span>
      </div>
      <div className="relative" style={{ height: totalHeight }}>
        {groups.map((g, i) => (
          <div
            key={g.card.name}
            className="absolute left-0 group cursor-pointer"
            style={{ top: i * peek, width: cardWidth, zIndex: i + 1 }}
            onMouseEnter={(e) => onHover(g.card, e.clientX, e.clientY)}
            onMouseMove={(e) => onHover(g.card, e.clientX, e.clientY)}
            onMouseLeave={onLeave}
          >
            {g.card.imageUrl ? (
              <img
                src={g.card.imageUrl}
                alt={g.card.name}
                className="w-full block rounded-[4%] shadow-sm border border-border/40"
                draggable={false}
              />
            ) : (
              <div
                className="bg-muted border border-border rounded-[4%] p-1 flex flex-col"
                style={{ width: cardWidth, height: cardHeight }}
              >
                <span className="text-[9px] text-muted-foreground leading-tight">{g.card.name}</span>
              </div>
            )}
            {g.count > 1 && (
              <div className="absolute top-1 left-1 bg-black/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center border border-white/30 shadow" style={{ zIndex: 10 }}>
                {g.count}
              </div>
            )}
            <div className="absolute inset-0 rounded-[4%] bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto">
              <Button size="sm" variant="secondary" className="h-6 w-4/5 text-xs" onClick={(e) => { e.stopPropagation(); onAddOne(g); }}>
                <Plus className="h-3 w-3 mr-1" /> Add
              </Button>
              <Button size="sm" variant="ghost" className="h-6 w-4/5 text-xs text-white/80 hover:text-white hover:bg-white/10" onClick={(e) => { e.stopPropagation(); onRemoveOne(g.card.name); }}>
                <Minus className="h-3 w-3 mr-1" /> Remove
              </Button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Visual grid card ─────────────────────────────────────────────────────────

function CardVisual({
  group,
  dragId,
  onAddOne,
  onRemoveOne,
  onHover,
  onLeave,
}: {
  group: CardGroup;
  dragId: string;
  onAddOne: () => void;
  onRemoveOne: () => void;
  onHover: (x: number, y: number) => void;
  onLeave: () => void;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: dragId,
    data: { type: "deck-card", card: group.card, name: group.card.name },
  });

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={cn("relative group cursor-grab active:cursor-grabbing select-none", isDragging && "opacity-30")}
      onMouseEnter={(e) => onHover(e.clientX, e.clientY)}
      onMouseMove={(e) => onHover(e.clientX, e.clientY)}
      onMouseLeave={onLeave}
    >
      {group.card.imageUrl ? (
        <img
          src={group.card.imageUrl}
          alt={group.card.name}
          className="w-full rounded-lg border border-border/50 shadow-sm"
          draggable={false}
        />
      ) : (
        <div className="w-full aspect-[2.5/3.5] rounded-lg border border-border bg-muted flex items-center justify-center p-2">
          <span className="text-xs text-center text-muted-foreground font-medium leading-tight">{group.card.name}</span>
        </div>
      )}
      {group.count > 1 && (
        <div className="absolute top-1 left-1 bg-black/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center border border-white/20">
          {group.count}
        </div>
      )}
      <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto">
        <Button size="sm" variant="secondary" className="h-6 w-4/5 text-xs" onClick={(e) => { e.stopPropagation(); onAddOne(); }}>
          <Plus className="h-3 w-3 mr-1" /> Add
        </Button>
        <Button size="sm" variant="ghost" className="h-6 w-4/5 text-xs text-white/80 hover:text-white hover:bg-white/10" onClick={(e) => { e.stopPropagation(); onRemoveOne(); }}>
          <Minus className="h-3 w-3 mr-1" /> Remove
        </Button>
      </div>
    </div>
  );
}

// ─── List row ─────────────────────────────────────────────────────────────────

function CardRow({
  group,
  dragId,
  isCommander,
  onAddOne,
  onRemoveOne,
  onRemoveAll,
  onSetCommander,
  onRemoveCommander,
  onMoveToSide,
  onHover,
  onLeave,
}: {
  group: CardGroup;
  dragId: string;
  isCommander: boolean;
  onAddOne: () => void;
  onRemoveOne: () => void;
  onRemoveAll: () => void;
  onSetCommander: () => void;
  onRemoveCommander: () => void;
  onMoveToSide: () => void;
  onHover: (x: number, y: number) => void;
  onLeave: () => void;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: dragId,
    data: { type: "deck-card", card: group.card, name: group.card.name },
  });

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={cn(
        "flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5 cursor-grab active:cursor-grabbing select-none",
        isDragging && "opacity-30"
      )}
      onMouseEnter={(e) => onHover(e.clientX, e.clientY)}
      onMouseMove={(e) => onHover(e.clientX, e.clientY)}
      onMouseLeave={onLeave}
    >
      <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">{group.count}</span>
      <span className="text-sm flex-1 truncate" title={group.card.name}>{group.card.name}</span>
      {group.card.manaCost && (
        <span className="text-xs text-muted-foreground shrink-0 font-mono">{group.card.manaCost}</span>
      )}
      {group.card.power && group.card.toughness && (
        <span className="text-xs text-muted-foreground/60 shrink-0 font-mono">
          {group.card.power}/{group.card.toughness}
        </span>
      )}
      <div className="flex gap-0.5 shrink-0 pointer-events-none group-hover:pointer-events-auto">
        <Button
          size="icon" variant="ghost"
          className={isCommander ? "h-5 w-5 text-yellow-500" : "h-5 w-5 text-muted-foreground/40 opacity-0 group-hover:opacity-100 transition-opacity"}
          title={isCommander ? "Remove commander" : "Set as commander"}
          onClick={(e) => { e.stopPropagation(); isCommander ? onRemoveCommander() : onSetCommander(); }}
        >
          <Crown className="h-3 w-3" />
        </Button>
        <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button size="icon" variant="ghost" className="h-5 w-5" title="Add one" onClick={(e) => { e.stopPropagation(); onAddOne(); }}>
            <Plus className="h-3 w-3" />
          </Button>
          <Button size="icon" variant="ghost" className="h-5 w-5" title="Remove one" onClick={(e) => { e.stopPropagation(); onRemoveOne(); }}>
            <Minus className="h-3 w-3" />
          </Button>
          <Button size="icon" variant="ghost" className="h-5 w-5 text-muted-foreground" title="Move to sideboard" onClick={(e) => { e.stopPropagation(); onMoveToSide(); }}>
            <Download className="h-3 w-3" />
          </Button>
          <Button size="icon" variant="ghost" className="h-5 w-5 text-destructive" title="Remove all" onClick={(e) => { e.stopPropagation(); onRemoveAll(); }}>
            <X className="h-3 w-3" />
          </Button>
        </div>
      </div>
    </div>
  );
}

// ─── Collapsible section ──────────────────────────────────────────────────────

function DeckSection({
  label,
  groups,
  commanderName,
  viewMode,
  sectionId,
  gridCols,
  onAddOne,
  onRemoveOne,
  onRemoveAll,
  onSetCommander,
  onRemoveCommander,
  onMoveToSide,
  onHover,
  onLeave,
}: {
  label: string;
  groups: CardGroup[];
  commanderName?: string;
  viewMode: ViewMode;
  sectionId: string;
  gridCols: string;
  onAddOne: (g: CardGroup) => void;
  onRemoveOne: (name: string) => void;
  onRemoveAll: (name: string) => void;
  onSetCommander: (card: Card) => void;
  onRemoveCommander: () => void;
  onMoveToSide: (name: string) => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
}) {
  const [collapsed, setCollapsed] = useState(false);
  if (groups.length === 0) return null;
  const count = groups.reduce((s, g) => s + g.count, 0);

  return (
    <div className="mb-3">
      <button
        type="button"
        className="flex items-center gap-1 w-full text-left mb-1.5 hover:text-foreground text-muted-foreground"
        onClick={() => setCollapsed((v) => !v)}
      >
        {collapsed ? <ChevronRight className="h-3 w-3 shrink-0" /> : <ChevronDown className="h-3 w-3 shrink-0" />}
        <span className="text-xs font-semibold uppercase tracking-wide">{label}</span>
        <span className="text-xs text-muted-foreground/60 ml-1">({count})</span>
      </button>

      {!collapsed && (
        viewMode === "list" ? (
          <div className="space-y-0.5">
            {groups.map((g) => (
              <CardRow
                key={g.card.name}
                group={g}
                dragId={`deck-${sectionId}-${g.card.name}`}
                isCommander={commanderName === g.card.name}
                onAddOne={() => onAddOne(g)}
                onRemoveOne={() => onRemoveOne(g.card.name)}
                onRemoveAll={() => onRemoveAll(g.card.name)}
                onSetCommander={() => onSetCommander(g.card)}
                onRemoveCommander={onRemoveCommander}
                onMoveToSide={() => onMoveToSide(g.card.name)}
                onHover={(x, y) => onHover(g.card, x, y)}
                onLeave={onLeave}
              />
            ))}
          </div>
        ) : (
          <div className={cn("grid gap-2", gridCols)}>
            {groups.map((g) => (
              <CardVisual
                key={g.card.name}
                group={g}
                dragId={`deck-${sectionId}-${g.card.name}`}
                onAddOne={() => onAddOne(g)}
                onRemoveOne={() => onRemoveOne(g.card.name)}
                onHover={(x, y) => onHover(g.card, x, y)}
                onLeave={onLeave}
              />
            ))}
          </div>
        )
      )}
    </div>
  );
}

// ─── Main DeckBuilder component ───────────────────────────────────────────────

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
    setCommander,
    removeCommander,
  } = useDeckStore();

  const [editingName, setEditingName] = useState(false);
  const [deckFilter, setDeckFilter] = useState("");
  const [nameInput, setNameInput] = useState(currentDeck.name);
  const [loadDialogOpen, setLoadDialogOpen] = useState(false);
  const [hovered, setHovered] = useState<{ card: Card; x: number; y: number } | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  // 1=smallest (many cols) … 5=largest (few cols)
  const [cardSize, setCardSize] = useState(3);
  const nameInputRef = useRef<HTMLInputElement>(null);
  const enrichedNamesRef = useRef(new Set<string>());

  // Map cardSize (1–5) → Tailwind grid-cols class
  const GRID_COLS: Record<number, string> = {
    1: "grid-cols-5",
    2: "grid-cols-4",
    3: "grid-cols-3",
    4: "grid-cols-2",
    5: "grid-cols-1",
  };
  const gridCols = GRID_COLS[cardSize] ?? "grid-cols-3";
  const cardWidth = CARD_WIDTH_MAP[cardSize] ?? 115;

  // Large drop zone covering the entire deck panel (main)
  const { setNodeRef: setMainDropRef, isOver: isOverMain } = useDroppable({ id: "drop-main" });
  // Specific drop zone for sideboard
  const { setNodeRef: setSideDropRef, isOver: isOverSide } = useDroppable({ id: "drop-side" });

  // Auto-enrich cards missing CMC/mana data
  useEffect(() => {
    const allCards = [...currentDeck.cards, ...currentDeck.sideboard];
    const toFetch = allCards
      .filter((c) => (c.cmc === undefined || c.cmc === null) && !c.manaCost && !enrichedNamesRef.current.has(c.name.toLowerCase()))
      .map((c) => c.name);
    if (toFetch.length === 0) return;
    const uniqueNames = [...new Set(toFetch)];
    uniqueNames.forEach((n) => enrichedNamesRef.current.add(n.toLowerCase()));
    fetchCardCollection(uniqueNames).then((scryfallMap) => {
      const updates = new Map<string, Partial<Card>>();
      for (const [key, sc] of scryfallMap) updates.set(key, scryfallCardToPartial(sc));
      enrichDeckCards(updates);
    }).catch(() => {});
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentDeck.cards, currentDeck.sideboard]);

  // Filter
  const filterLc = deckFilter.toLowerCase();
  const filteredMain = filterLc ? currentDeck.cards.filter((c) => c.name.toLowerCase().includes(filterLc)) : currentDeck.cards;
  const filteredSide = filterLc ? currentDeck.sideboard.filter((c) => c.name.toLowerCase().includes(filterLc)) : currentDeck.sideboard;

  // Compute groups per type section
  const sectionGroups = MAIN_SECTIONS.map((s) => ({
    ...s,
    groups: groupCards(filteredMain.filter((c) => s.filter(c.types))),
  }));

  // "Other" catches anything not matched
  const matchedNames = new Set(sectionGroups.flatMap((s) => s.groups.map((g) => g.card.name)));
  const otherGroups = groupCards(filteredMain.filter((c) => !matchedNames.has(c.name)));

  const sideGroups = groupCards(filteredSide);

  // Stack-mode columns: group all main cards into type columns
  const stackColsData = (() => {
    const allGroups = groupCards(filteredMain);
    const usedNames = new Set<string>();
    const cols = STACK_TYPE_COLS.map((col) => ({
      ...col,
      groups: allGroups.filter((g) => {
        if (usedNames.has(g.card.name)) return false;
        if (col.filter(g.card.types)) { usedNames.add(g.card.name); return true; }
        return false;
      }),
    }));
    const otherGroups = allGroups.filter((g) => !usedNames.has(g.card.name));
    if (otherGroups.length > 0) cols.push({ id: "other", label: "Other", filter: () => false, groups: otherGroups });
    return cols.filter((c) => c.groups.length > 0);
  })();

  // ── Handlers ──

  function handleRemoveOneFromMain(cardName: string) {
    const cards = currentDeck.cards;
    for (let i = cards.length - 1; i >= 0; i--) {
      if (cards[i].name === cardName) { removeFromMain(cards[i].id); return; }
    }
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
    const side = currentDeck.sideboard;
    for (let i = side.length - 1; i >= 0; i--) {
      if (side[i].name === cardName) { removeFromSide(side[i].id); return; }
    }
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
        if (/^(sideboard|side)$/i.test(line)) { inSide = true; continue; }
        const match = line.match(/^(\d+)x?\s+(.+)$/i);
        if (!match) continue;
        parsed.push({ count: parseInt(match[1], 10), name: match[2].trim(), side: inSide });
      }
      if (parsed.length === 0) { toast.error("No cards found in clipboard"); return; }
      let imported = 0;
      for (const { name, count, side } of parsed) {
        for (let i = 0; i < count; i++) {
          const card: Card = {
            id: crypto.randomUUID(), name, setCode: "", cardNumber: "", color: "",
            manaCost: "", types: [], subtypes: [], supertypes: [], text: "",
            isPlayable: true, isSelected: false, isChoosable: true, controllerId: "", ownerId: "", zoneId: "",
          };
          if (side) addToSide(card); else addToMain(card);
          imported++;
        }
      }
      toast.success(`Imported ${imported} cards — fetching data…`);
      try {
        const scryfallMap = await fetchCardCollection(parsed.map((p) => p.name));
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

  const sharedSectionProps = {
    commanderName: currentDeck.commander?.name,
    viewMode,
    gridCols,
    onAddOne: handleAddOneToMain,
    onRemoveOne: handleRemoveOneFromMain,
    onRemoveAll: handleRemoveAllFromMain,
    onSetCommander: setCommander,
    onRemoveCommander: removeCommander,
    onMoveToSide: handleMoveToSide,
    onHover: (card: Card, x: number, y: number) => setHovered({ card, x, y }),
    onLeave: () => setHovered(null),
  };

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

        {/* Card size slider — only meaningful in visual/stack modes */}
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

      {/* ── Deck filter ── */}
      <div className="px-3 py-1.5 border-b shrink-0">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
          <Input className="h-7 text-xs pl-6 pr-6" placeholder="Filter deck…" value={deckFilter} onChange={(e) => setDeckFilter(e.target.value)} />
          {deckFilter && (
            <button type="button" className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground" onClick={() => setDeckFilter("")}>
              <X className="h-3 w-3" />
            </button>
          )}
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
        {viewMode === "stack" ? (
          /* ── Stack view: horizontal type columns ── */
          <div className="h-full overflow-auto">
            <div className="flex gap-5 items-start p-3 min-w-max">
              {/* Commander column */}
              {currentDeck.commander && (
                <StackColumn
                  label="Commander"
                  groups={[{ card: currentDeck.commander, count: 1 }]}
                  cardWidth={cardWidth}
                  onAddOne={() => {}}
                  onRemoveOne={removeCommander}
                  onHover={(card, x, y) => setHovered({ card, x, y })}
                  onLeave={() => setHovered(null)}
                />
              )}

              {/* Type columns */}
              {stackColsData.map((col) => (
                <StackColumn
                  key={col.id}
                  label={col.label}
                  groups={col.groups}
                  cardWidth={cardWidth}
                  onAddOne={handleAddOneToMain}
                  onRemoveOne={handleRemoveOneFromMain}
                  onHover={(card, x, y) => setHovered({ card, x, y })}
                  onLeave={() => setHovered(null)}
                />
              ))}

              {/* Sideboard column */}
              <div
                ref={setSideDropRef}
                className={cn("shrink-0 rounded-lg transition-colors p-1 -m-1", isOverSide && "bg-primary/10")}
              >
                {sideGroups.length > 0 ? (
                  <StackColumn
                    label="Sideboard"
                    groups={sideGroups}
                    cardWidth={cardWidth}
                    onAddOne={(g) => addToSide({ ...g.card, id: crypto.randomUUID() })}
                    onRemoveOne={handleRemoveOneFromSide}
                    onHover={(card, x, y) => setHovered({ card, x, y })}
                    onLeave={() => setHovered(null)}
                  />
                ) : (
                  <div className="flex flex-col" style={{ width: cardWidth }}>
                    <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Sideboard</div>
                    <div className="border-2 border-dashed border-border/40 rounded-lg py-6 flex items-center justify-center">
                      <p className="text-[10px] text-muted-foreground/40 text-center">Drop cards here</p>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        ) : (
          /* ── List / Visual view: vertical scroll ── */
          <ScrollArea className="h-full px-3 py-2">
            {/* Commander */}
            {currentDeck.commander && (() => {
              const cmdGroup: CardGroup = { card: currentDeck.commander!, count: 1 };
              return (
                <div className="mb-3">
                  <div className="flex items-center gap-1 mb-1.5">
                    <Crown className="h-3 w-3 text-yellow-500 shrink-0" />
                    <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Commander</span>
                  </div>
                  {viewMode === "list" ? (
                    <div
                      className="flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5"
                      onMouseEnter={(e) => setHovered({ card: currentDeck.commander!, x: e.clientX, y: e.clientY })}
                      onMouseMove={(e) => setHovered({ card: currentDeck.commander!, x: e.clientX, y: e.clientY })}
                      onMouseLeave={() => setHovered(null)}
                    >
                      <Crown className="h-3 w-3 text-yellow-500 shrink-0" />
                      <span className="text-sm flex-1 truncate">{currentDeck.commander.name}</span>
                      {currentDeck.commander.manaCost && (
                        <span className="text-xs text-muted-foreground shrink-0 font-mono">{currentDeck.commander.manaCost}</span>
                      )}
                      <Button size="icon" variant="ghost" className="h-5 w-5 text-destructive opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                        onClick={() => removeCommander()}>
                        <X className="h-3 w-3" />
                      </Button>
                    </div>
                  ) : (
                    <div className={cn("grid gap-2", gridCols)}>
                      <div className="relative">
                        <div className="absolute top-1 right-1 z-20 bg-black/70 rounded-full p-0.5 shadow">
                          <Crown className="h-3.5 w-3.5 text-yellow-400" />
                        </div>
                        <CardVisual
                          group={cmdGroup}
                          dragId="deck-commander"
                          onAddOne={() => {}}
                          onRemoveOne={removeCommander}
                          onHover={(x, y) => setHovered({ card: currentDeck.commander!, x, y })}
                          onLeave={() => setHovered(null)}
                        />
                      </div>
                    </div>
                  )}
                </div>
              );
            })()}

            {/* Empty state */}
            {currentDeck.cards.length === 0 && (
              <div className="flex flex-col items-center justify-center py-16 text-center">
                <div className="text-4xl mb-3 opacity-20">🃏</div>
                <p className="text-sm text-muted-foreground">Drag cards here from the search panel</p>
                <p className="text-xs text-muted-foreground/60 mt-1">or use the + buttons on hover</p>
              </div>
            )}

            {/* Type sections */}
            {sectionGroups.map((s) => (
              <DeckSection
                key={s.id}
                label={s.label}
                groups={s.groups}
                sectionId={s.id}
                {...sharedSectionProps}
              />
            ))}

            {/* Other */}
            {otherGroups.length > 0 && (
              <DeckSection
                label="Other"
                groups={otherGroups}
                sectionId="other"
                {...sharedSectionProps}
              />
            )}

            {/* Sideboard */}
            <div
              ref={setSideDropRef}
              className={cn(
                "mt-2 rounded-lg border-2 border-dashed transition-colors",
                isOverSide ? "border-primary bg-primary/10" : "border-border/40 hover:border-border/60"
              )}
            >
              <div className="px-2 pt-2 pb-1">
                <div className="flex items-center gap-2 mb-1.5">
                  <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                    Sideboard ({currentDeck.sideboard.length})
                  </span>
                  <span className="text-xs text-muted-foreground/40">— drop cards here</span>
                </div>
                {sideGroups.length === 0 ? (
                  <div className="py-4 text-center">
                    <p className="text-xs text-muted-foreground/40">Drag cards here for sideboard</p>
                  </div>
                ) : viewMode === "list" ? (
                  <div className="space-y-0.5 pb-1">
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
                        {g.card.manaCost && <span className="text-xs text-muted-foreground shrink-0 font-mono">{g.card.manaCost}</span>}
                        <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                          <Button size="icon" variant="ghost" className="h-5 w-5 text-muted-foreground" title="Move to main" onClick={() => handleMoveToMain(g.card.name)}>
                            <Upload className="h-3 w-3" />
                          </Button>
                          <Button size="icon" variant="ghost" className="h-5 w-5 text-destructive" onClick={() => handleRemoveOneFromSide(g.card.name)}>
                            <X className="h-3 w-3" />
                          </Button>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className={cn("grid gap-2 pb-1", gridCols)}>
                    {sideGroups.map((g) => (
                      <div
                        key={g.card.name}
                        className="relative group"
                        onMouseEnter={(e) => setHovered({ card: g.card, x: e.clientX, y: e.clientY })}
                        onMouseMove={(e) => setHovered({ card: g.card, x: e.clientX, y: e.clientY })}
                        onMouseLeave={() => setHovered(null)}
                      >
                        {g.card.imageUrl ? (
                          <img src={g.card.imageUrl} alt={g.card.name} className="w-full rounded-lg border border-border/50" draggable={false} />
                        ) : (
                          <div className="w-full aspect-[2.5/3.5] rounded-lg border border-border bg-muted flex items-center justify-center p-2">
                            <span className="text-xs text-center text-muted-foreground">{g.card.name}</span>
                          </div>
                        )}
                        {g.count > 1 && (
                          <div className="absolute top-1 left-1 bg-black/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center">
                            {g.count}
                          </div>
                        )}
                        <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto">
                          <Button size="sm" variant="secondary" className="h-6 w-4/5 text-xs" onClick={() => handleMoveToMain(g.card.name)}>
                            → Main
                          </Button>
                          <Button size="sm" variant="ghost" className="h-6 w-4/5 text-xs text-white/80 hover:text-white" onClick={() => handleRemoveOneFromSide(g.card.name)}>
                            <X className="h-3 w-3 mr-1" /> Remove
                          </Button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </ScrollArea>
        )}
      </div>

      <DeckStats />

      {hovered && <CardPreview card={hovered.card} mouseX={hovered.x} mouseY={hovered.y} />}
    </div>
  );
}
