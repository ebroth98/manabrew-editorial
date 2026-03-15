import { Modal } from "@/components/game/Modal";
import { cn } from "@/lib/utils";
import { useState } from "react";

interface SpecifyManaComboModalProps {
  availableColors: string[];
  amount: number;
  sourceCardName?: string;
  onConfirm: (chosenColors: string[]) => void;
}

const LETTER_INFO: Record<string, { label: string; bg: string; hoverRing: string; text: string }> = {
  W: { label: "White", bg: "bg-amber-50 dark:bg-amber-100", hoverRing: "hover:ring-amber-300", text: "text-amber-900" },
  U: { label: "Blue",  bg: "bg-blue-100 dark:bg-blue-200", hoverRing: "hover:ring-blue-400", text: "text-blue-900" },
  B: { label: "Black", bg: "bg-gray-300 dark:bg-gray-400", hoverRing: "hover:ring-gray-600", text: "text-gray-900" },
  R: { label: "Red",   bg: "bg-red-100 dark:bg-red-200", hoverRing: "hover:ring-red-400", text: "text-red-900" },
  G: { label: "Green", bg: "bg-green-100 dark:bg-green-200", hoverRing: "hover:ring-green-500", text: "text-green-900" },
  C: { label: "Colorless", bg: "bg-gray-100 dark:bg-gray-200", hoverRing: "hover:ring-gray-400", text: "text-gray-700" },
};

function manaSymbolUrl(symbol: string): string {
  return `https://svgs.scryfall.io/card-symbols/${encodeURIComponent(symbol)}.svg`;
}

export function SpecifyManaComboModal({ availableColors, amount, sourceCardName, onConfirm }: SpecifyManaComboModalProps) {
  const [counts, setCounts] = useState<Record<string, number>>(() => {
    const initial: Record<string, number> = {};
    for (const c of availableColors) initial[c] = 0;
    // Default: all to first color
    if (availableColors.length > 0) initial[availableColors[0]] = amount;
    return initial;
  });

  const total = Object.values(counts).reduce((a, b) => a + b, 0);
  const remaining = amount - total;

  const increment = (color: string) => {
    if (remaining <= 0) return;
    setCounts(prev => ({ ...prev, [color]: (prev[color] ?? 0) + 1 }));
  };

  const decrement = (color: string) => {
    setCounts(prev => {
      const cur = prev[color] ?? 0;
      if (cur <= 0) return prev;
      return { ...prev, [color]: cur - 1 };
    });
  };

  const handleConfirm = () => {
    const result: string[] = [];
    for (const [color, count] of Object.entries(counts)) {
      for (let i = 0; i < count; i++) result.push(color);
    }
    onConfirm(result);
  };

  return (
    <Modal maxWidth="max-w-sm" maxHeight="">
      <Modal.Header>
        <div>
          <h2 className="font-semibold text-base">Choose Mana Colors</h2>
          {sourceCardName && <p className="text-xs text-muted-foreground font-medium">{sourceCardName}</p>}
        </div>
      </Modal.Header>

      <Modal.Instructions>
        Distribute {amount} mana across colors. {remaining > 0 ? `${remaining} remaining.` : ""}
      </Modal.Instructions>

      <div className="p-4 flex flex-col gap-2">
        {availableColors.map(color => {
          const info = LETTER_INFO[color] ?? { label: color, bg: "bg-muted", hoverRing: "", text: "text-foreground" };
          const count = counts[color] ?? 0;
          return (
            <div key={color} className="flex items-center gap-3">
              <img src={manaSymbolUrl(color)} alt={`{${color}}`} className="w-8 h-8" />
              <span className={cn("text-sm font-medium w-16", info.text)}>{info.label}</span>
              <div className="flex items-center gap-1">
                <button
                  onClick={() => decrement(color)}
                  disabled={count <= 0}
                  className="w-7 h-7 rounded bg-muted hover:bg-muted/80 disabled:opacity-30 text-sm font-bold"
                >
                  −
                </button>
                <span className="w-8 text-center text-sm font-semibold">{count}</span>
                <button
                  onClick={() => increment(color)}
                  disabled={remaining <= 0}
                  className="w-7 h-7 rounded bg-muted hover:bg-muted/80 disabled:opacity-30 text-sm font-bold"
                >
                  +
                </button>
              </div>
            </div>
          );
        })}
      </div>

      <div className="p-3 border-t flex justify-end">
        <button
          onClick={handleConfirm}
          disabled={remaining > 0}
          className={cn(
            "px-4 py-1.5 rounded text-sm font-medium transition-colors",
            remaining > 0
              ? "bg-muted text-muted-foreground cursor-not-allowed"
              : "bg-primary text-primary-foreground hover:bg-primary/90"
          )}
        >
          Confirm
        </button>
      </div>
    </Modal>
  );
}
