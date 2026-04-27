import { Modal } from "./Modal";
import { cn } from "@/lib/utils";
import { useEffect, useRef } from "react";
import { manaSymbolUrl, normalizeManaCode } from "@/api/scryfall";
import type { ManaCode } from "@/types/scryfall";

interface ChooseColorModalProps {
  validColors: string[];
  sourceCardName?: string;
  onConfirm: (color: string) => void;
}

/** Per-colour picker cell — each uses its `mana-<letter>` theme token for
 *  the background so a preset can retone the whole set at once.  */
const COLOR_INFO: Record<string, { symbol: ManaCode; bg: string }> = {
  White: { symbol: "W", bg: "bg-mana-w" },
  Blue: { symbol: "U", bg: "bg-mana-u" },
  Black: { symbol: "B", bg: "bg-mana-b" },
  Red: { symbol: "R", bg: "bg-mana-r" },
  Green: { symbol: "G", bg: "bg-mana-g" },
  Colorless: { symbol: "C", bg: "bg-mana-c" },
};

export function ChooseColorModal({
  validColors,
  sourceCardName,
  onConfirm,
}: ChooseColorModalProps) {
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
            {sourceCardName && (
              <p className="text-xs text-muted-foreground font-medium">{sourceCardName}</p>
            )}
          </div>
        </Modal.Header>

        <Modal.Instructions>Click a color to choose it.</Modal.Instructions>

        <div className="p-4 flex flex-wrap gap-3 justify-center">
          {validColors.map((color) => {
            const info = COLOR_INFO[color] ?? {
              symbol: normalizeManaCode(color[0] ?? "") ?? "C",
              bg: "bg-muted",
            };
            return (
              <button
                key={color}
                onClick={() => onConfirm(color)}
                className={cn(
                  "flex flex-col items-center gap-1 px-4 py-3 rounded-lg border transition-all",
                  "hover:ring-2 hover:ring-ring hover:scale-105 active:scale-95",
                  "text-foreground",
                  info.bg,
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
