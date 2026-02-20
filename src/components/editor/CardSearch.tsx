import { useState, useRef, useEffect } from "react";
import { useCardSearch } from "@/hooks/useCards";
import { Input } from "@/components/ui/input";
import { Card } from "@/components/game/Card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useDeckStore } from "@/stores/useDeckStore";
import { Loader2, Crown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { ScryfallCard } from "@/types/scryfall";
import type { Card as XMageCard } from "@/types/xmage";

// ─── Filter definitions (mirrors Forge CardColorFilter / CardTypeFilter / CardCMCFilter) ────

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

// ─── Toggle button helper ────────────────────────────────────────────────────

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

// ─── Component ────────────────────────────────────────────────────────────────

export function CardSearch() {
  const [text, setText] = useState("");
  const [debouncedText, setDebouncedText] = useState("");
  const [activeColors, setActiveColors] = useState<Set<string>>(new Set());
  const [activeTypes, setActiveTypes] = useState<Set<string>>(new Set());
  const [activeCmc, setActiveCmc] = useState<CmcId>("any");

  const { addToMain, addToSide, setCommander } = useDeckStore();
  const observerTarget = useRef(null);

  const effectiveQuery = buildScryfallQuery(debouncedText, activeColors, activeTypes, activeCmc);

  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, status } =
    useCardSearch(effectiveQuery);

  // Debounce text
  useEffect(() => {
    const handler = setTimeout(() => setDebouncedText(text), 500);
    return () => clearTimeout(handler);
  }, [text]);

  // Infinite scroll
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasNextPage) fetchNextPage();
      },
      { threshold: 1.0 }
    );
    if (observerTarget.current) observer.observe(observerTarget.current);
    return () => observer.disconnect();
  }, [hasNextPage, fetchNextPage]);

  function toggleColor(id: string) {
    setActiveColors((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }

  function toggleType(id: string) {
    setActiveTypes((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }

  return (
    <div className="flex flex-col h-full w-full">
      {/* Search bar + filter rows */}
      <div className="p-3 border-b space-y-2">
        <Input
          placeholder="Search (e.g. 'Jace', 'Counterspell', or use filters below)"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />

        {/* Color filters */}
        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-xs text-muted-foreground w-10 shrink-0">Color</span>
          {COLOR_FILTERS.map((f) => (
            <FilterBtn
              key={f.id}
              active={activeColors.has(f.id)}
              onClick={() => toggleColor(f.id)}
              title={f.title}
            >
              {f.label}
            </FilterBtn>
          ))}
        </div>

        {/* Type filters */}
        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-xs text-muted-foreground w-10 shrink-0">Type</span>
          {TYPE_FILTERS.map((f) => (
            <FilterBtn
              key={f.id}
              active={activeTypes.has(f.id)}
              onClick={() => toggleType(f.id)}
            >
              {f.label}
            </FilterBtn>
          ))}
        </div>

        {/* CMC filters */}
        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-xs text-muted-foreground w-10 shrink-0">CMC</span>
          {CMC_FILTERS.map((f) => (
            <FilterBtn
              key={f.id}
              active={activeCmc === f.id}
              onClick={() => setActiveCmc(f.id)}
            >
              {f.label}
            </FilterBtn>
          ))}
        </div>
      </div>

      <ScrollArea className="flex-1 p-4">
        {status === "pending" && effectiveQuery && (
          <div className="flex justify-center p-8">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        )}

        {status === "error" && (
          <div className="text-center p-8 text-red-500">
            Error fetching cards. Please try again.
          </div>
        )}

        {!effectiveQuery && (
          <p className="text-center text-sm text-muted-foreground py-12">
            Enter a card name or select filters to search.
          </p>
        )}

        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8 gap-3 pb-4">
          {data?.pages.map((group) =>
            group.data.map((card) => {
              const xmageCard = mapScryfallToXMage(card);
              const isLegendaryCreature =
                xmageCard.supertypes.includes("Legendary") &&
                xmageCard.types.includes("Creature");
              return (
                <div key={card.id} className="relative group">
                  <Card card={xmageCard} className="w-full" />
                  <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-1.5 rounded-lg">
                    <Button
                      size="sm"
                      variant="secondary"
                      className="w-4/5"
                      onClick={() => addToMain(xmageCard)}
                    >
                      + Main
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="w-4/5"
                      onClick={() => addToSide(xmageCard)}
                    >
                      + Side
                    </Button>
                    {isLegendaryCreature && (
                      <Button
                        size="sm"
                        variant="outline"
                        className="w-4/5 gap-1 text-yellow-500 border-yellow-500/50 hover:bg-yellow-500/10"
                        onClick={() => setCommander(xmageCard)}
                      >
                        <Crown className="h-3 w-3" />
                        Commander
                      </Button>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>

        <div ref={observerTarget} className="h-10 flex justify-center items-center">
          {isFetchingNextPage && (
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
