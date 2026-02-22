import { createPortal } from "react-dom";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { Card as CardType } from "@/types/xmage";
import { cn } from "@/lib/utils";
import { useState, useEffect } from "react";

export type LibraryPeekMode = "scry" | "surveil" | "dig" | "discard";

interface LibraryPeekModalProps {
  mode: LibraryPeekMode;
  cards: CardType[];
  /** dig: maximum number of cards the player may take */
  numToTake?: number;
  /** dig: whether taking 0 cards is a valid choice */
  optional?: boolean;
  /** Selected IDs sent back:
   *  scry    → IDs going to the bottom (rest go to top)
   *  surveil → IDs going to the graveyard (rest go to top)
   *  dig     → IDs going to hand (rest go to graveyard)  */
  onConfirm: (selectedIds: string[]) => void;
}

const MODE_CONFIG: Record<
  LibraryPeekMode,
  {
    title: string;
    subtitle: string;
    instructions: string;
    selectedLabel: string;
    unselectedLabel: string;
    selectedRing: string;
    confirmLabel: (selected: number, total: number, required?: number) => string;
  }
> = {
  scry: {
    title: "Scry",
    subtitle: "Arrange the top cards of your library",
    instructions:
      "Click cards you want to put on the bottom. Unselected cards return to the top.",
    selectedLabel: "BOTTOM",
    unselectedLabel: "TOP",
    selectedRing: "ring-orange-400",
    confirmLabel: (n, t) => `Confirm — ${n} on bottom, ${t - n} on top`,
  },
  surveil: {
    title: "Surveil",
    subtitle: "Choose cards to send to the graveyard",
    instructions:
      "Click cards to send to the graveyard. Unselected cards return to the top of your library.",
    selectedLabel: "GRAVEYARD",
    unselectedLabel: "TOP",
    selectedRing: "ring-red-500",
    confirmLabel: (n, t) => `Confirm — ${n} to graveyard, ${t - n} on top`,
  },
  dig: {
    title: "Dig",
    subtitle: "Choose cards to add to your hand",
    instructions:
      "Select cards to take to your hand. The rest go to the graveyard.",
    selectedLabel: "HAND",
    unselectedLabel: "GRAVEYARD",
    selectedRing: "ring-green-400",
    confirmLabel: (n) => `Take ${n} to Hand`,
  },
  discard: {
    title: "Discard",
    subtitle: "Choose cards to discard from your hand",
    instructions:
      "Click cards to discard them. You must discard the required number.",
    selectedLabel: "DISCARD",
    unselectedLabel: "KEEP",
    selectedRing: "ring-red-500",
    confirmLabel: (n, _t, required) =>
      n < (required ?? 0)
        ? `Select ${(required ?? 0) - n} more to discard`
        : `Discard ${n}`,
  },
};

export function LibraryPeekModal({
  mode,
  cards,
  numToTake,
  optional,
  onConfirm,
}: LibraryPeekModalProps) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [hoveredCard, setHoveredCard] = useState<CardType | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  const config = MODE_CONFIG[mode];
  const required = mode === "discard" ? (numToTake ?? 1) : undefined;
  const max = mode === "dig" ? (numToTake ?? cards.length) : mode === "discard" ? (numToTake ?? cards.length) : cards.length;
  const canConfirm =
    mode === "dig"
      ? optional || selected.size > 0
      : mode === "discard"
        ? selected.size === (numToTake ?? 1)
        : true;

  function toggleCard(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        if ((mode === "dig" || mode === "discard") && next.size >= max) return prev;
        next.add(id);
      }
      return next;
    });
  }

  function handleConfirm() {
    onConfirm([...selected]);
  }

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Enter" && canConfirm) handleConfirm();
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, canConfirm]);

  return createPortal(
    <div
      className="fixed inset-0 z-[9000] flex items-center justify-center"
      style={{ backgroundColor: "rgba(0,0,0,0.6)" }}
    >
      <div
        className="bg-card border rounded-xl shadow-2xl flex flex-col max-w-4xl w-full max-h-[85vh] mx-4"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <div>
            <h2 className="font-semibold text-base">{config.title}</h2>
            <p className="text-xs text-muted-foreground">{config.subtitle}</p>
          </div>
          {(mode === "dig" || mode === "discard") && numToTake !== undefined && (
            <Badge variant="secondary">
              {selected.size} / {numToTake} selected
            </Badge>
          )}
        </div>

        {/* Instructions */}
        <div className="px-4 py-2 bg-blue-50 dark:bg-blue-950/20 border-b">
          <p className="text-sm font-semibold text-blue-700 dark:text-blue-400 text-center">
            {config.instructions}
          </p>
        </div>

        {/* Card grid */}
        <div className="overflow-y-auto p-4 flex-1">
          {cards.length === 0 ? (
            <p className="text-sm text-muted-foreground italic text-center py-8">
              No cards to choose from
            </p>
          ) : (
            <div className="flex flex-wrap gap-4 content-start justify-center">
              {cards.map((card) => {
                const isSelected = selected.has(card.id);
                return (
                  <div
                    key={card.id}
                    className="shrink-0 cursor-pointer group flex flex-col items-center gap-1"
                    onMouseEnter={(e) => {
                      setHoveredCard(card);
                      setMousePos({ x: e.clientX, y: e.clientY });
                    }}
                    onMouseLeave={() => setHoveredCard(null)}
                    onClick={() => toggleCard(card.id)}
                  >
                    <Card
                      card={card}
                      className={cn(
                        "w-[100px] h-[140px] transition-transform group-hover:scale-105",
                        isSelected && `ring-2 ${config.selectedRing}`,
                      )}
                    />
                    <Badge
                      variant={isSelected ? "default" : "outline"}
                      className="text-[10px] h-4 px-1"
                    >
                      {isSelected
                        ? config.selectedLabel
                        : config.unselectedLabel}
                    </Badge>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-4 py-3 border-t gap-3">
          <div className="text-xs text-muted-foreground">
            {cards.length} card{cards.length !== 1 ? "s" : ""}
            {mode === "dig" && optional && " · Taking 0 is allowed"}
          </div>
          <div className="flex gap-2">
            {/* Select All / Clear helpers for scry and surveil */}
            {mode !== "dig" && mode !== "discard" && (
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  setSelected(new Set(cards.map((c) => c.id)))
                }
              >
                All to {config.selectedLabel}
              </Button>
            )}
            {mode !== "dig" && mode !== "discard" && selected.size > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setSelected(new Set())}
              >
                Clear
              </Button>
            )}
            <Button
              size="sm"
              disabled={!canConfirm}
              onClick={handleConfirm}
            >
              {config.confirmLabel(selected.size, cards.length, required)}
            </Button>
          </div>
        </div>
      </div>

      {hoveredCard && (
        <CardPreview
          card={hoveredCard}
          mouseX={mousePos.x}
          mouseY={mousePos.y}
        />
      )}
    </div>,
    document.body,
  );
}
