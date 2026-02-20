import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import { useDeckStore } from "@/stores/useDeckStore";
import { GAME_FORMATS, validateDeck, type GameFormat } from "@/lib/formats";
import { FormatBadge } from "@/components/game/FormatBadge";
import { cn } from "@/lib/utils";
import { AlertCircle } from "lucide-react";

interface CreateGameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Pre-select a saved deck by ID (e.g. when launched from MyDecks) */
  preSelectedDeckId?: string;
  /** Called with the deck card names and chosen format ID when Create is confirmed */
  onStart: (cardNames: string[], formatId: string) => void;
}

export function CreateGameDialog({
  open,
  onOpenChange,
  preSelectedDeckId,
  onStart,
}: CreateGameDialogProps) {
  const { savedDecks, currentDeck } = useDeckStore();

  const [gameName, setGameName] = useState("");
  const [selectedFormat, setSelectedFormat] = useState<GameFormat>(
    GAME_FORMATS[0]
  );
  const [selectedDeck, setSelectedDeck] = useState<string>(
    preSelectedDeckId ?? "current"
  );

  const allDecks = [
    {
      id: "current",
      name: `${currentDeck.name} (current)`,
      cardNames: currentDeck.cards.map((c) => c.name),
    },
    ...savedDecks.map((s) => ({
      id: s.id,
      name: s.deck.name,
      cardNames: s.deck.cards.map((c) => c.name),
    })),
  ];

  const selectedDeckEntry = allDecks.find((d) => d.id === selectedDeck);
  const selectedDeckNames = selectedDeckEntry?.cardNames ?? [];
  const selectedDeckValidation = validateDeck(selectedDeckNames, selectedFormat);

  function handleCreate() {
    if (!selectedDeckEntry) {
      toast.error("Please select a deck");
      return;
    }
    if (!selectedDeckValidation.legal) {
      toast.error(selectedDeckValidation.errors[0] ?? "Deck is not legal in this format");
      return;
    }
    onOpenChange(false);
    onStart(selectedDeckNames, selectedFormat.id);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Create New Game</DialogTitle>
          </DialogHeader>

          <div className="space-y-4 py-2">
            {/* Game name */}
            <div className="space-y-1">
              <Label htmlFor="gameName">Game Name</Label>
              <Input
                id="gameName"
                placeholder="My Game"
                value={gameName}
                onChange={(e) => setGameName(e.target.value)}
              />
            </div>

            {/* Format picker */}
            <div className="space-y-1.5">
              <Label>Format</Label>
              <div className="flex gap-2">
                {GAME_FORMATS.map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    onClick={() => setSelectedFormat(f)}
                    className={cn(
                      "flex-1 rounded border px-3 py-2 text-left transition-colors",
                      selectedFormat.id === f.id
                        ? "border-primary bg-secondary"
                        : "border-border hover:bg-muted/60"
                    )}
                  >
                    <div className="flex items-center gap-1.5 mb-0.5">
                      <FormatBadge formatId={f.id} />
                      <span className="text-sm font-medium">{f.name}</span>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {f.description}
                    </p>
                  </button>
                ))}
              </div>

              {/* Format rules summary */}
              <div className="text-xs text-muted-foreground border rounded px-2 py-1.5 bg-muted/30">
                <span className="font-medium">Rules: </span>
                {selectedFormat.deckRules.minDeckSize}
                {selectedFormat.deckRules.maxDeckSize !== null
                  ? `–${selectedFormat.deckRules.maxDeckSize}`
                  : "+"}
                {" "}cards · max {selectedFormat.deckRules.maxCopies}{" "}
                {selectedFormat.deckRules.maxCopies === 1 ? "copy" : "copies"} ·{" "}
                {selectedFormat.deckRules.startingLife} starting life
              </div>
            </div>

            {/* Deck picker */}
            <div className="space-y-1">
              <Label>Deck</Label>
              <div className="space-y-1 max-h-40 overflow-y-auto border rounded p-2">
                {allDecks.map((d) => {
                  const validation = validateDeck(d.cardNames, selectedFormat);
                  const isSelected = selectedDeck === d.id;
                  return (
                    <div
                      key={d.id}
                      className={cn(
                        "flex items-center justify-between p-1.5 rounded text-sm",
                        validation.legal
                          ? "cursor-pointer"
                          : "cursor-not-allowed opacity-50",
                        isSelected && validation.legal
                          ? "bg-secondary"
                          : validation.legal
                          ? "hover:bg-muted"
                          : ""
                      )}
                      onClick={() => {
                        if (validation.legal) setSelectedDeck(d.id);
                      }}
                    >
                      <span className="truncate">{d.name}</span>
                      <div className="flex items-center gap-1.5 shrink-0 ml-2">
                        <span className="text-xs text-muted-foreground">
                          {d.cardNames.length} cards
                        </span>
                        {!validation.legal && (
                          <span title={validation.errors.slice(0, 3).join("\n")}>
                            <AlertCircle className="h-3.5 w-3.5 text-destructive" />
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
              {/* Validation error for selected deck */}
              {!selectedDeckValidation.legal && selectedDeckEntry && (
                <p className="text-xs text-destructive">
                  {selectedDeckValidation.errors[0]}
                </p>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={!selectedDeckValidation.legal}
            >
              Create Game
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
  );
}
