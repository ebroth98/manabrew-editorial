import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { FormatBadge } from "@/components/game/FormatBadge";
import { Pencil, Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { extractColors, COLOR_MAP } from "@/views/myDecks.utils";
import { inferFormats } from "@/lib/formats";
import type { SavedDeck } from "@/stores/useDeckStore";

interface DeckCardProps {
  deck: SavedDeck;
  isSelected: boolean;
  isEditing: boolean;
  editName: string;
  onSelect: () => void;
  onRename: (name: string) => void;
  onStartRename: () => void;
  onConfirmRename: () => void;
  onCancelRename: () => void;
  onDelete: () => void;
  onEditNameChange: (name: string) => void;
}

function ColorPip({ color }: { color: string }) {
  const style = COLOR_MAP[color];
  if (!style) return null;
  return (
    <span
      className={cn(
        "inline-flex items-center justify-center w-5 h-5 rounded-full border text-xs font-bold",
        style.bg,
        style.border,
        color === "B" ? "text-gray-100" : "text-gray-700",
      )}
    >
      {style.label}
    </span>
  );
}

export function DeckCard({
  deck,
  isSelected,
  isEditing,
  editName,
  onSelect,
  onStartRename,
  onConfirmRename,
  onCancelRename,
  onDelete,
  onEditNameChange,
}: DeckCardProps) {
  const deckColors = extractColors(deck.deck.cards);
  const deckFormats = inferFormats(deck.deck.cards.map((c: { name: string }) => c.name));

  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-2 cursor-pointer group",
        isSelected
          ? "bg-secondary text-secondary-foreground"
          : "hover:bg-muted/60",
      )}
      onClick={onSelect}
    >
      {/* Color identity pips */}
      <div className="flex gap-0.5 w-16 shrink-0">
        {deckColors.length > 0 ? (
          deckColors.map((c) => <ColorPip key={c} color={c} />)
        ) : (
          <span className="text-xs text-muted-foreground italic">
            —
          </span>
        )}
      </div>

      {/* Name + count + format badges */}
      <div className="flex-1 min-w-0">
        {isEditing ? (
          <Input
            autoFocus
            value={editName}
            className="h-6 text-sm px-1"
            onChange={(e) => onEditNameChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") onConfirmRename();
              if (e.key === "Escape") onCancelRename();
            }}
            onBlur={onConfirmRename}
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <p className="text-sm font-medium truncate">
            {deck.deck.name}
          </p>
        )}
        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-xs text-muted-foreground">
            {deck.deck.cards.length} cards
          </span>
          {deckFormats.map((f) => (
            <FormatBadge key={f.id} formatId={f.id} />
          ))}
        </div>
      </div>

      {/* Actions (visible on hover or selection) */}
      <div
        className={cn(
          "flex gap-1 shrink-0 transition-opacity",
          isSelected
            ? "opacity-100"
            : "opacity-0 group-hover:opacity-100",
        )}
      >
        <Button
          size="icon"
          variant="ghost"
          className="h-6 w-6"
          title="Rename"
          onClick={(e) => {
            e.stopPropagation();
            onStartRename();
          }}
        >
          <Pencil className="h-3 w-3" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="h-6 w-6 text-destructive hover:text-destructive"
          title="Delete"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
        >
          <Trash2 className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}
