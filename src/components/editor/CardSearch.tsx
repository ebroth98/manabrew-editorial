import { useState, useRef, useEffect } from "react";
import { useCardSearch } from "@/hooks/useCards";
import { Input } from "@/components/ui/input";
import { Card } from "@/components/game/Card";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useDeckStore } from "@/stores/useDeckStore";
import { Loader2, Crown, LayoutGrid, List } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { ScryfallCard } from "@/types/scryfall";
import type { Card as XMageCard } from "@/types/xmage";
import { useDraggable } from "@dnd-kit/core";

// ─── Filter definitions ────────────────────────────────────────────────────────

const COLOR_FILTERS = [
  { id: "W", label: "W", scryfall: "c:w", title: "White" },
  { id: "U", label: "U", scryfall: "c:u", title: "Blue" },
  { id: "B", label: "B", scryfall: "c:b", title: "Black" },
  { id: "R", label: "R", scryfall: "c:r", title: "Red" },
  { id: "G", label: "G", scryfall: "c:g", title: "Green" },
  { id: "C", label: "C", scryfall: "c:c", title: "Colorless" },
  { id: "M", label: "M", scryfall: "c:m", title: "Multicolor" },
] as const;

const TYPE_FILTERS = [
  { id: "Creature", label: "Creature" },
  { id: "Land", label: "Land" },
  { id: "Instant", label: "Instant" },
  { id: "Sorcery", label: "Sorcery" },
  { id: "Enchantment", label: "Enchant." },
  { id: "Artifact", label: "Artifact" },
  { id: "Planeswalker", label: "PW" },
] as const;

const CMC_FILTERS = [
  { id: "any", label: "Any" },
  { id: "0", label: "0" },
  { id: "1", label: "1" },
  { id: "2", label: "2" },
  { id: "3", label: "3" },
  { id: "4", label: "4" },
  { id: "5", label: "5" },
  { id: "6", label: "6+" },
] as const;

type CmcId = (typeof CMC_FILTERS)[number]["id"];

function buildScryfallQuery(
  text: string,
  colors: Set<string>,
  types: Set<string>,
  cmc: CmcId
): string {
  const parts: string[] = [];
  if (text.trim()) parts.push(text.trim());
  if (colors.size > 0) {
    const clauses = [...colors].map((id) => COLOR_FILTERS.find((f) => f.id === id)!.scryfall);
    parts.push(clauses.length === 1 ? clauses[0] : `(${clauses.join(" or ")})`);
  }
  if (types.size > 0) {
    const clauses = [...types].map((t) => `t:${t.toLowerCase()}`);
    parts.push(clauses.length === 1 ? clauses[0] : `(${clauses.join(" or ")})`);
  }
  if (cmc !== "any") {
    parts.push(cmc === "6" ? "cmc>=6" : `cmc=${cmc}`);
  }
  return parts.join(" ");
}

const SUPERTYPES = new Set(["Basic", "Legendary", "Snow", "World", "Ongoing"]);

function mapScryfallToXMage(sfCard: ScryfallCard): XMageCard {
  const [mainPart = "", subPart = ""] = sfCard.type_line.split("—").map((s) => s.trim());
  const mainTokens = mainPart.split(/\s+/).filter(Boolean);
  const supertypes = mainTokens.filter((t) => SUPERTYPES.has(t));
  const types = mainTokens.filter((t) => !SUPERTYPES.has(t));
  const subtypes = subPart ? subPart.split(/\s+/).filter(Boolean) : [];
  return {
    id: sfCard.id,
    name: sfCard.name,
    setCode: sfCard.set,
    cardNumber: sfCard.collector_number,
    color: sfCard.colors ? sfCard.colors.join("") : "",
    manaCost: sfCard.mana_cost || "",
    cmc: sfCard.cmc,
    types,
    subtypes,
    supertypes,
    power: sfCard.power,
    toughness: sfCard.toughness,
    text: sfCard.oracle_text || "",
    imageUrl: sfCard.image_uris?.normal,
    isPlayable: true,
    isSelected: false,
    isChoosable: true,
    controllerId: "",
    ownerId: "",
    zoneId: "",
  };
}

// ─── Filter toggle button ─────────────────────────────────────────────────────

function FilterBtn({
  active,
  onClick,
  title,
  children,
  className,
}: {
  active: boolean;
  onClick: () => void;
  title?: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={cn(
        "h-6 min-w-[1.5rem] px-1.5 rounded text-xs font-semibold border transition-colors select-none",
        active
          ? "bg-primary text-primary-foreground border-primary"
          : "bg-background text-muted-foreground border-border hover:bg-muted",
        className
      )}
    >
      {children}
    </button>
  );
}

// ─── Draggable card wrapper (grid mode) ───────────────────────────────────────

function DraggableCardGrid({
  card,
  onAddMain,
  onAddSide,
  onSetCommander,
  isLegendaryCreature,
}: {
  card: XMageCard;
  onAddMain: () => void;
  onAddSide: () => void;
  onSetCommander: () => void;
  isLegendaryCreature: boolean;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `search-${card.id}`,
    data: { card },
  });

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={cn("relative group cursor-grab active:cursor-grabbing", isDragging && "opacity-30")}
    >
      <Card card={card} className="w-full" />
      <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-1.5 rounded-lg pointer-events-none group-hover:pointer-events-auto">
        <Button size="sm" variant="secondary" className="w-4/5" onClick={(e) => { e.stopPropagation(); onAddMain(); }}>
          + Main
        </Button>
        <Button size="sm" variant="outline" className="w-4/5" onClick={(e) => { e.stopPropagation(); onAddSide(); }}>
          + Side
        </Button>
        {isLegendaryCreature && (
          <Button
            size="sm"
            variant="outline"
            className="w-4/5 gap-1 text-yellow-500 border-yellow-500/50 hover:bg-yellow-500/10"
            onClick={(e) => { e.stopPropagation(); onSetCommander(); }}
          >
            <Crown className="h-3 w-3" />
            Commander
          </Button>
        )}
      </div>
    </div>
  );
}

// ─── Draggable card wrapper (list mode) ───────────────────────────────────────

function DraggableCardRow({
  card,
  onAddMain,
  onAddSide,
  onSetCommander,
  isLegendaryCreature,
}: {
  card: XMageCard;
  onAddMain: () => void;
  onAddSide: () => void;
  onSetCommander: () => void;
  isLegendaryCreature: boolean;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `search-${card.id}`,
    data: { card },
  });

  const typeStr = [...(card.supertypes ?? []), ...(card.types ?? [])].join(" ");

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={cn(
        "flex items-center gap-2 px-2 py-1.5 rounded hover:bg-muted/50 group cursor-grab active:cursor-grabbing border-b border-border/30 last:border-0",
        isDragging && "opacity-30"
      )}
    >
      {/* Small card art thumbnail */}
      {card.imageUrl && (
        <div className="w-8 h-8 shrink-0 rounded overflow-hidden bg-muted">
          <img src={card.imageUrl} alt="" className="w-full h-full object-cover object-top" draggable={false} />
        </div>
      )}
      {!card.imageUrl && (
        <div className="w-8 h-8 shrink-0 rounded bg-muted flex items-center justify-center text-[9px] text-muted-foreground font-mono">
          {card.setCode?.toUpperCase() || "?"}
        </div>
      )}

      {/* Name + type */}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate leading-tight">{card.name}</div>
        <div className="text-xs text-muted-foreground truncate leading-tight">{typeStr}</div>
      </div>

      {/* Mana cost */}
      {card.manaCost && (
        <ManaSymbols cost={card.manaCost} size="sm" className="shrink-0" />
      )}

      {/* Action buttons (hover) */}
      <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0 pointer-events-none group-hover:pointer-events-auto">
        <Button size="sm" variant="secondary" className="h-6 px-2 text-xs" onClick={(e) => { e.stopPropagation(); onAddMain(); }}>
          Main
        </Button>
        <Button size="sm" variant="outline" className="h-6 px-2 text-xs" onClick={(e) => { e.stopPropagation(); onAddSide(); }}>
          Side
        </Button>
        {isLegendaryCreature && (
          <Button
            size="sm"
            variant="ghost"
            className="h-6 w-6 p-0 text-yellow-500"
            title="Set as commander"
            onClick={(e) => { e.stopPropagation(); onSetCommander(); }}
          >
            <Crown className="h-3 w-3" />
          </Button>
        )}
      </div>
    </div>
  );
}

// ─── Main component ────────────────────────────────────────────────────────────

export function CardSearch() {
  const [text, setText] = useState("");
  const [debouncedText, setDebouncedText] = useState("");
  const [activeColors, setActiveColors] = useState<Set<string>>(new Set());
  const [activeTypes, setActiveTypes] = useState<Set<string>>(new Set());
  const [activeCmc, setActiveCmc] = useState<CmcId>("any");
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");

  const { addToMain, addToSide, setCommander } = useDeckStore();
  const observerTarget = useRef(null);

  const effectiveQuery = buildScryfallQuery(debouncedText, activeColors, activeTypes, activeCmc);
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, status } = useCardSearch(effectiveQuery);

  useEffect(() => {
    const handler = setTimeout(() => setDebouncedText(text), 500);
    return () => clearTimeout(handler);
  }, [text]);

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => { if (entries[0].isIntersecting && hasNextPage) fetchNextPage(); },
      { threshold: 1.0 }
    );
    if (observerTarget.current) observer.observe(observerTarget.current);
    return () => observer.disconnect();
  }, [hasNextPage, fetchNextPage]);

  function toggleColor(id: string) {
    setActiveColors((prev) => { const n = new Set(prev); n.has(id) ? n.delete(id) : n.add(id); return n; });
  }
  function toggleType(id: string) {
    setActiveTypes((prev) => { const n = new Set(prev); n.has(id) ? n.delete(id) : n.add(id); return n; });
  }

  const allCards: XMageCard[] = (data?.pages.flatMap((p) => p.data.map(mapScryfallToXMage))) ?? [];

  return (
    <div className="flex flex-col h-full w-full">
      {/* Filters */}
      <div className="p-3 border-b space-y-2 shrink-0">
        <div className="flex gap-2">
          <Input
            placeholder="Search cards…"
            value={text}
            onChange={(e) => setText(e.target.value)}
            className="flex-1"
          />
          {/* View toggle */}
          <div className="flex border rounded-md overflow-hidden shrink-0">
            <button
              type="button"
              title="Grid view"
              onClick={() => setViewMode("grid")}
              className={cn(
                "px-2 py-1 text-xs transition-colors",
                viewMode === "grid" ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground"
              )}
            >
              <LayoutGrid className="h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              title="List view"
              onClick={() => setViewMode("list")}
              className={cn(
                "px-2 py-1 text-xs transition-colors border-l",
                viewMode === "list" ? "bg-primary text-primary-foreground" : "hover:bg-muted text-muted-foreground"
              )}
            >
              <List className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-xs text-muted-foreground w-10 shrink-0">Color</span>
          {COLOR_FILTERS.map((f) => (
            <FilterBtn key={f.id} active={activeColors.has(f.id)} onClick={() => toggleColor(f.id)} title={f.title}>
              {f.label}
            </FilterBtn>
          ))}
        </div>

        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-xs text-muted-foreground w-10 shrink-0">Type</span>
          {TYPE_FILTERS.map((f) => (
            <FilterBtn key={f.id} active={activeTypes.has(f.id)} onClick={() => toggleType(f.id)}>
              {f.label}
            </FilterBtn>
          ))}
        </div>

        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-xs text-muted-foreground w-10 shrink-0">CMC</span>
          {CMC_FILTERS.map((f) => (
            <FilterBtn key={f.id} active={activeCmc === f.id} onClick={() => setActiveCmc(f.id)}>
              {f.label}
            </FilterBtn>
          ))}
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-3">
          {status === "pending" && effectiveQuery && (
            <div className="flex justify-center p-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          )}
          {status === "error" && (
            <div className="text-center p-8 text-red-500">Error fetching cards. Please try again.</div>
          )}
          {!effectiveQuery && (
            <p className="text-center text-sm text-muted-foreground py-12">
              Enter a card name or select filters to search.
            </p>
          )}

          {viewMode === "grid" ? (
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3 pb-4">
              {allCards.map((card) => {
                const isLegendaryCreature =
                  card.supertypes.includes("Legendary") && card.types.includes("Creature");
                return (
                  <DraggableCardGrid
                    key={card.id}
                    card={card}
                    onAddMain={() => addToMain({ ...card, id: crypto.randomUUID() })}
                    onAddSide={() => addToSide({ ...card, id: crypto.randomUUID() })}
                    onSetCommander={() => setCommander(card)}
                    isLegendaryCreature={isLegendaryCreature}
                  />
                );
              })}
            </div>
          ) : (
            <div className="pb-4">
              {allCards.map((card) => {
                const isLegendaryCreature =
                  card.supertypes.includes("Legendary") && card.types.includes("Creature");
                return (
                  <DraggableCardRow
                    key={card.id}
                    card={card}
                    onAddMain={() => addToMain({ ...card, id: crypto.randomUUID() })}
                    onAddSide={() => addToSide({ ...card, id: crypto.randomUUID() })}
                    onSetCommander={() => setCommander(card)}
                    isLegendaryCreature={isLegendaryCreature}
                  />
                );
              })}
            </div>
          )}

          <div ref={observerTarget} className="h-10 flex justify-center items-center">
            {isFetchingNextPage && <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
