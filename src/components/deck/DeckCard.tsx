import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DeckLabelBadge } from "@/components/deck/DeckLabelBadge";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { FormatBadge } from "@/components/game/FormatBadge";
import { Pencil, Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { extractColors } from "@/views/myDecks.utils";
import { ManaSymbols } from "@/components/game/ManaSymbols";
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
  const [confirmOpen, setConfirmOpen] = useState(false);

  return (
    <>
      <div
        className={cn(
          "flex items-center gap-2 px-3 py-2 cursor-pointer group",
          isSelected
            ? "bg-secondary text-secondary-foreground"
            : "hover:bg-muted/60",
        )}
        onClick={onSelect}
      >
        {/* Color identity */}
        <div className="w-16 shrink-0">
          {deckColors.length > 0 ? (
            <ManaSymbols cost={deckColors.map((c) => `{${c}}`).join("")} size="sm" />
          ) : (
            <span className="text-xs text-muted-foreground italic">—</span>
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
              {deck.deck.format === "commander"
                ? deck.deck.cards.length + (deck.deck.commanders?.length ?? 0)
                : deck.deck.cards.length} cards
            </span>
            <FormatBadge formatId={deck.deck.format ?? "constructed"} />
            {deck.deck.labels?.map((label) => (
              <DeckLabelBadge key={label.name} label={label} size="sm" />
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
              setConfirmOpen(true);
            }}
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      </div>

      <Dialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Delete Deck</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete &ldquo;{deck.deck.name}&rdquo;? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="gap-2">
            <Button variant="outline" size="sm" onClick={() => setConfirmOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => {
                setConfirmOpen(false);
                onDelete();
              }}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
