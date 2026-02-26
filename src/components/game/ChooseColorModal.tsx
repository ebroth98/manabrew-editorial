import { Modal } from "@/components/game/Modal";
import { cn } from "@/lib/utils";
import { useEffect, useRef } from "react";

interface ChooseColorModalProps {
  validColors: string[];
  sourceCardName?: string;
  onConfirm: (color: string) => void;
}

const COLOR_INFO: Record<string, { symbol: string; bg: string; hoverRing: string; text: string }> = {
  White: { symbol: "W", bg: "bg-amber-50 dark:bg-amber-100", hoverRing: "hover:ring-amber-300", text: "text-amber-900" },
  Blue:  { symbol: "U", bg: "bg-blue-100 dark:bg-blue-200", hoverRing: "hover:ring-blue-400", text: "text-blue-900" },
  Black: { symbol: "B", bg: "bg-gray-300 dark:bg-gray-400", hoverRing: "hover:ring-gray-600", text: "text-gray-900" },
  Red:   { symbol: "R", bg: "bg-red-100 dark:bg-red-200", hoverRing: "hover:ring-red-400", text: "text-red-900" },
  Green: { symbol: "G", bg: "bg-green-100 dark:bg-green-200", hoverRing: "hover:ring-green-500", text: "text-green-900" },
};

function manaSymbolUrl(symbol: string): string {
  return `https://svgs.scryfall.io/card-symbols/${encodeURIComponent(symbol)}.svg`;
}

export function ChooseColorModal({ validColors, sourceCardName, onConfirm }: ChooseColorModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dialogRef.current?.focus();
  }, [validColors]);

  return (
    <Modal maxWidth="max-w-sm" maxHeight="" className="outline-none">
      <div ref={dialogRef} tabIndex={-1} className="outline-none" role="dialog" aria-modal="true">
        <Modal.Header>
          <div>
            <h2 className="font-semibold text-base">Choose a Color</h2>
            {sourceCardName && <p className="text-xs text-muted-foreground font-medium">{sourceCardName}</p>}
          </div>
        </Modal.Header>

        <Modal.Instructions>Click a color to choose it.</Modal.Instructions>

        <div className="p-4 flex flex-wrap gap-3 justify-center">
          {validColors.map((color) => {
            const info = COLOR_INFO[color] ?? { symbol: color[0], bg: "bg-muted", hoverRing: "hover:ring-border", text: "text-foreground" };
            return (
              <button
                key={color}
                onClick={() => onConfirm(color)}
                className={cn(
                  "flex flex-col items-center gap-1 px-4 py-3 rounded-lg border transition-all",
                  "hover:ring-2 hover:scale-105 active:scale-95",
                  info.bg, info.text, info.hoverRing,
                )}
              >
                <img
                  src={manaSymbolUrl(info.symbol)}
                  alt={`{${info.symbol}}`}
                  className="w-10 h-10"
                />
                <span className="text-xs font-semibold">{color}</span>
              </button>
            );
          })}
        </div>
      </div>
    </Modal>
  );
}
