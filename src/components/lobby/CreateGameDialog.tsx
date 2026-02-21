import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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

interface PresetDeckInfo {
  id: string;
  label: string;
  desc: string;
  color: string;
}

interface CreateGameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Pre-select a saved deck by ID (e.g. when launched from MyDecks) */
  preSelectedDeckId?: string;
  /** Called with the deck card names, format ID, optional commander name, and player count when Create is confirmed */
  onStart: (cardNames: string[], formatId: string, commanderName?: string, playerCount?: number) => void;
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
  const [selectedCommander, setSelectedCommander] = useState<string>(
    currentDeck.commander?.name ?? ""
  );
  const [presetDecks, setPresetDecks] = useState<PresetDeckInfo[]>([]);
  const [playerCount, setPlayerCount] = useState(2);

  useEffect(() => {
    invoke<PresetDeckInfo[]>("get_preset_decks")
      .then(setPresetDecks)
      .catch((e) => console.error("[CreateGameDialog] Failed to load preset decks:", e));
  }, []);

  const allDecks = [
    {
      id: "current",
      name: `${currentDeck.name} (current)`,
      // Include the designated commander in the card list sent to the engine
      cardNames: [
        ...currentDeck.cards.map((c) => c.name),
        ...(currentDeck.commander ? [currentDeck.commander.name] : []),
      ],
      isPreset: false as const,
      cards: currentDeck.cards,
      commanderName: currentDeck.commander?.name,
    },
    ...savedDecks.map((s) => ({
      id: s.id,
      name: s.deck.name,
      cardNames: [
        ...s.deck.cards.map((c) => c.name),
        ...(s.deck.commander ? [s.deck.commander.name] : []),
      ],
      isPreset: false as const,
      cards: s.deck.cards,
      commanderName: s.deck.commander?.name,
    })),
    ...presetDecks.map((deck) => ({
      id: `preset__${deck.id}`,
      name: deck.label,
      cardNames: [deck.id],
      isPreset: true as const,
      cards: [],
      commanderName: undefined as string | undefined,
    })),
  ];

  // Auto-populate commander when the selected deck changes
  useEffect(() => {
    const entry = allDecks.find((d) => d.id === selectedDeck);
    setSelectedCommander(entry?.commanderName ?? "");
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedDeck]);

  const selectedDeckEntry = allDecks.find((d) => d.id === selectedDeck);
  const selectedDeckNames = selectedDeckEntry?.cardNames ?? [];
  const selectedDeckValidation = selectedDeckEntry?.isPreset
    ? { legal: true, errors: [] as string[] }
    : validateDeck(selectedDeckNames, selectedFormat);

  // Get unique legendary creatures from the selected deck for the commander picker.
  // Also include the deck's designated commander even if it's stored separately.
  const legendaryCreatures = selectedDeckEntry
    ? Array.from(
        new Map([
          ...(selectedDeckEntry.commanderName
            ? [[selectedDeckEntry.commanderName, selectedDeckEntry.commanderName] as [string, string]]
            : []),
          ...selectedDeckEntry.cards
            .filter(
              (c) =>
                c.supertypes?.includes("Legendary") &&
                c.types?.includes("Creature")
            )
            .map((c) => [c.name, c.name] as [string, string]),
        ]).values()
      )
    : [];

  const needsCommander = selectedFormat.deckRules.requiresCommander;
  const commanderValid = !needsCommander || selectedCommander !== "";

  function handleCreate() {
    if (!selectedDeckEntry) {
      toast.error("Please select a deck");
      return;
    }
    if (!selectedDeckValidation.legal) {
      toast.error(selectedDeckValidation.errors[0] ?? "Deck is not legal in this format");
      return;
    }
    if (needsCommander && !selectedCommander) {
      toast.error("Please select a commander");
      return;
    }
    onOpenChange(false);
    onStart(selectedDeckNames, selectedFormat.id, needsCommander ? selectedCommander : undefined, playerCount);
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

            {/* DEV: Player count */}
            <div className="space-y-1">
              <Label className="flex items-center gap-1.5">
                Player Count
                <span className="text-[10px] font-mono text-orange-500 bg-orange-50 dark:bg-orange-950/30 px-1 rounded">DEV</span>
              </Label>
              <div className="flex gap-1">
                {[2, 3, 4].map((n) => (
                  <button
                    key={n}
                    type="button"
                    onClick={() => setPlayerCount(n)}
                    className={cn(
                      "px-3 py-1 rounded border text-sm transition-colors",
                      playerCount === n
                        ? "border-orange-400 bg-orange-50 dark:bg-orange-950/30 text-orange-700 dark:text-orange-400 font-semibold"
                        : "border-border hover:bg-muted/60"
                    )}
                  >
                    {n}
                  </button>
                ))}
                <span className="text-xs text-muted-foreground self-center ml-1">
                  {playerCount === 2 ? "1 opponent" : `${playerCount - 1} opponents (${playerCount - 2} simulated)`}
                </span>
              </div>
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
                  const validation = d.isPreset
                    ? { legal: true, errors: [] as string[] }
                    : validateDeck(d.cardNames, selectedFormat);
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
                        {d.isPreset ? (
                          <span className="text-xs text-muted-foreground bg-muted px-1 py-0.5 rounded">
                            Preset
                          </span>
                        ) : (
                          <span className="text-xs text-muted-foreground">
                            {d.cardNames.length} cards
                          </span>
                        )}
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

            {/* Commander picker — shown only for Commander format */}
            {needsCommander && (
              <div className="space-y-1">
                <Label>Commander</Label>
                {legendaryCreatures.length === 0 ? (
                  <p className="text-xs text-muted-foreground italic">
                    No legendary creatures found in selected deck. Type a card name below.
                  </p>
                ) : null}
                <select
                  className="w-full rounded border border-border bg-background px-2 py-1.5 text-sm"
                  value={selectedCommander}
                  onChange={(e) => setSelectedCommander(e.target.value)}
                >
                  <option value="">— Choose commander —</option>
                  {legendaryCreatures.map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
                </select>
                {legendaryCreatures.length === 0 && (
                  <input
                    className="w-full rounded border border-border bg-background px-2 py-1.5 text-sm"
                    placeholder="Type commander card name"
                    value={selectedCommander}
                    onChange={(e) => setSelectedCommander(e.target.value)}
                  />
                )}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={!selectedDeckValidation.legal || !commanderValid}
            >
              Create Game
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
  );
}
