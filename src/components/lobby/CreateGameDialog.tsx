import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import { useDeckStore } from "@/stores/useDeckStore";
import { GAME_FORMATS, validateDeck, type GameFormat } from "@/lib/formats";
import { FormatBadge } from "@/components/game/FormatBadge";
import { cn } from "@/lib/utils";
import { AlertCircle, Check, Shuffle, Swords } from "lucide-react";

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

  const [selectedFormat, setSelectedFormat] = useState<GameFormat>(GAME_FORMATS[0]);
  const [selectedDeck, setSelectedDeck] = useState<string>(preSelectedDeckId ?? "current");
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

  // User-built decks
  const userDecks = [
    {
      id: "current",
      name: currentDeck.name,
      badge: "editing",
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
      badge: null as string | null,
      cardNames: [
        ...s.deck.cards.map((c) => c.name),
        ...(s.deck.commander ? [s.deck.commander.name] : []),
      ],
      isPreset: false as const,
      cards: s.deck.cards,
      commanderName: s.deck.commander?.name,
    })),
  ];

  // Preset deck entries
  const presetDeckEntries = presetDecks.map((deck) => ({
    id: `preset__${deck.id}`,
    name: deck.label,
    desc: deck.desc,
    color: deck.color,
    cardNames: [deck.id],
    isPreset: true as const,
    cards: [],
    commanderName: undefined as string | undefined,
  }));

  const allDecks = [...userDecks, ...presetDeckEntries];

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

  const legendaryCreatures = selectedDeckEntry
    ? Array.from(
        new Map([
          ...(selectedDeckEntry.commanderName
            ? [[selectedDeckEntry.commanderName, selectedDeckEntry.commanderName] as [string, string]]
            : []),
          ...selectedDeckEntry.cards
            .filter(
              (c) =>
                c.supertypes?.includes("Legendary") && c.types?.includes("Creature")
            )
            .map((c) => [c.name, c.name] as [string, string]),
        ]).values()
      )
    : [];

  const needsCommander = selectedFormat.deckRules.requiresCommander;
  const commanderValid = !needsCommander || selectedCommander !== "";
  const isReady = !!selectedDeckEntry && selectedDeckValidation.legal && commanderValid;

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
      <DialogContent className="max-w-2xl p-0 gap-0 overflow-hidden">

        {/* ── Header ── */}
        <div className="px-6 py-4 border-b">
          <DialogTitle className="text-lg font-semibold">New Game</DialogTitle>
          <p className="text-sm text-muted-foreground mt-0.5">
            Pick a deck and battle a random AI opponent
          </p>
        </div>

        {/* ── Body: left panel (settings) + right panel (deck picker) ── */}
        <div className="flex overflow-hidden" style={{ maxHeight: "65vh" }}>

          {/* Left panel — Format & options */}
          <div className="w-48 border-r flex-shrink-0 p-4 space-y-5 overflow-y-auto bg-muted/20">

            {/* Format */}
            <div>
              <SectionLabel>Format</SectionLabel>
              <div className="mt-2 space-y-2">
                {GAME_FORMATS.map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    onClick={() => setSelectedFormat(f)}
                    className={cn(
                      "w-full rounded-lg border p-2.5 text-left transition-colors",
                      selectedFormat.id === f.id
                        ? "border-primary bg-primary/5"
                        : "border-border hover:bg-muted/60"
                    )}
                  >
                    <div className="mb-1">
                      <FormatBadge formatId={f.id} />
                    </div>
                    <p className="font-medium text-xs">{f.name}</p>
                    <p className="text-[10px] text-muted-foreground mt-0.5 leading-tight">
                      {f.description}
                    </p>
                  </button>
                ))}
              </div>
            </div>

            {/* Rules summary */}
            <div>
              <SectionLabel>Rules</SectionLabel>
              <div className="mt-2 space-y-1.5">
                <RulePill
                  label="Deck"
                  value={
                    selectedFormat.deckRules.minDeckSize +
                    (selectedFormat.deckRules.maxDeckSize
                      ? `–${selectedFormat.deckRules.maxDeckSize}`
                      : "+") +
                    " cards"
                  }
                />
                <RulePill
                  label="Copies"
                  value={
                    selectedFormat.deckRules.maxCopies === 1
                      ? "Singleton"
                      : `Max ${selectedFormat.deckRules.maxCopies}`
                  }
                />
                <RulePill
                  label="Life"
                  value={`${selectedFormat.deckRules.startingLife}`}
                />
              </div>
            </div>

            {/* Commander picker — only for Commander format */}
            {needsCommander && (
              <div>
                <SectionLabel>Commander</SectionLabel>
                <div className="mt-2 space-y-1.5">
                  {legendaryCreatures.length === 0 && (
                    <p className="text-[10px] text-muted-foreground italic">
                      No legendaries in deck — type a name below.
                    </p>
                  )}
                  {legendaryCreatures.length > 0 ? (
                    <select
                      className="w-full rounded border border-border bg-background px-2 py-1.5 text-xs"
                      value={selectedCommander}
                      onChange={(e) => setSelectedCommander(e.target.value)}
                    >
                      <option value="">— Choose —</option>
                      {legendaryCreatures.map((name) => (
                        <option key={name} value={name}>
                          {name}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      className="w-full rounded border border-border bg-background px-2 py-1.5 text-xs"
                      placeholder="Card name"
                      value={selectedCommander}
                      onChange={(e) => setSelectedCommander(e.target.value)}
                    />
                  )}
                </div>
              </div>
            )}

            {/* DEV: player count */}
            <div>
              <SectionLabel>
                Opponents
                <span className="ml-1 text-[9px] font-mono text-orange-500 bg-orange-50 dark:bg-orange-950/30 px-1 rounded">
                  DEV
                </span>
              </SectionLabel>
              <div className="mt-2 flex gap-1">
                {[2, 3, 4].map((n) => (
                  <button
                    key={n}
                    type="button"
                    onClick={() => setPlayerCount(n)}
                    className={cn(
                      "flex-1 py-1 rounded border text-xs transition-colors",
                      playerCount === n
                        ? "border-orange-400 bg-orange-50 dark:bg-orange-950/30 text-orange-700 dark:text-orange-400 font-semibold"
                        : "border-border hover:bg-muted/60"
                    )}
                  >
                    {n - 1}v1
                  </button>
                ))}
              </div>
            </div>
          </div>

          {/* Right panel — Deck picker */}
          <div className="flex-1 overflow-y-auto">

            {/* Preset decks */}
            <div className="p-4">
              <SectionLabel>Preset Decks</SectionLabel>
              <p className="text-[11px] text-muted-foreground mt-0.5 mb-3">
                Pre-built themed decks — always legal, great for testing mechanics.
              </p>
              <div className="grid grid-cols-3 gap-2">
                {presetDeckEntries.map((deck) => {
                  const isSelected = selectedDeck === deck.id;
                  return (
                    <button
                      key={deck.id}
                      type="button"
                      onClick={() => setSelectedDeck(deck.id)}
                      className={cn(
                        "rounded-lg border p-2.5 text-left transition-all",
                        isSelected
                          ? "border-primary bg-primary/5 ring-1 ring-primary"
                          : "border-border hover:bg-muted/40 hover:shadow-sm"
                      )}
                    >
                      <div className="flex items-start justify-between gap-1 mb-1">
                        <span className={cn("font-semibold text-xs leading-tight", deck.color)}>
                          {deck.name}
                        </span>
                        {isSelected && (
                          <Check className="h-3 w-3 text-primary shrink-0 mt-0.5" />
                        )}
                      </div>
                      <p className="text-[10px] text-muted-foreground leading-tight line-clamp-2">
                        {deck.desc}
                      </p>
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Divider */}
            <div className="mx-4 border-t" />

            {/* User decks */}
            <div className="p-4">
              <SectionLabel>Your Decks</SectionLabel>
              <p className="text-[11px] text-muted-foreground mt-0.5 mb-3">
                Decks you've built in the editor.
              </p>
              {userDecks.length === 0 ? (
                <p className="text-xs text-muted-foreground italic">
                  No saved decks. Build one in the Deck Editor.
                </p>
              ) : (
                <div className="grid grid-cols-3 gap-2">
                  {userDecks.map((d) => {
                    const validation = validateDeck(d.cardNames, selectedFormat);
                    const isSelected = selectedDeck === d.id;
                    const colorPips = getDeckColors(d.cards);
                    const breakdown = getDeckTypeBreakdown(d.cards);
                    return (
                      <button
                        key={d.id}
                        type="button"
                        onClick={() => { if (validation.legal) setSelectedDeck(d.id); }}
                        disabled={!validation.legal}
                        title={!validation.legal ? validation.errors[0] : undefined}
                        className={cn(
                          "rounded-lg border p-2.5 text-left transition-all",
                          validation.legal ? "cursor-pointer" : "cursor-not-allowed opacity-50",
                          isSelected && validation.legal
                            ? "border-primary bg-primary/5 ring-1 ring-primary"
                            : validation.legal
                            ? "border-border hover:bg-muted/40 hover:shadow-sm"
                            : "border-border"
                        )}
                      >
                        {/* Name row */}
                        <div className="flex items-start justify-between gap-1 mb-1.5">
                          <span className="font-semibold text-xs leading-tight truncate">
                            {d.name}
                          </span>
                          <div className="flex items-center gap-0.5 shrink-0 mt-0.5">
                            {isSelected && <Check className="h-3 w-3 text-primary" />}
                            {!validation.legal && <AlertCircle className="h-3 w-3 text-destructive" />}
                          </div>
                        </div>
                        {/* Color pips */}
                        <div className="flex items-center gap-1 mb-1.5 min-h-[10px]">
                          {colorPips.length > 0
                            ? colorPips.map((c) => <ColorPip key={c} color={c} />)
                            : <span className="text-[10px] text-muted-foreground">Colorless</span>}
                        </div>
                        {/* Type breakdown */}
                        <p className="text-[10px] text-muted-foreground leading-tight line-clamp-2">
                          {!validation.legal ? validation.errors[0] : breakdown}
                        </p>
                        {/* Footer: card count + badge */}
                        <div className="flex items-center justify-between mt-1.5">
                          <span className="text-[10px] text-muted-foreground">
                            {d.cardNames.length} cards
                          </span>
                          {d.badge && (
                            <Badge variant="outline" className="text-[9px] h-4 px-1">
                              {d.badge}
                            </Badge>
                          )}
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── Footer ── */}
        <div className="px-6 py-3 border-t flex items-center justify-between gap-4 bg-muted/10">
          {/* Selected deck summary */}
          <div className="flex items-center gap-2 text-sm min-w-0">
            {selectedDeckEntry ? (
              <>
                <span className="text-muted-foreground shrink-0">Playing</span>
                <span className="font-medium truncate">{selectedDeckEntry.name}</span>
                <span className="text-muted-foreground shrink-0">vs</span>
                <span className="inline-flex items-center gap-1 text-muted-foreground shrink-0">
                  <Shuffle className="h-3 w-3" />
                  Random AI
                </span>
              </>
            ) : (
              <span className="text-muted-foreground italic text-xs">No deck selected</span>
            )}
          </div>
          <div className="flex gap-2 shrink-0">
            <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button size="sm" onClick={handleCreate} disabled={!isReady} className="gap-1.5">
              <Swords className="h-3.5 w-3.5" />
              Play
            </Button>
          </div>
        </div>

      </DialogContent>
    </Dialog>
  );
}

// ── Small helpers ──────────────────────────────────────────────────

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <Label className="text-[10px] uppercase tracking-wider text-muted-foreground font-semibold">
      {children}
    </Label>
  );
}

function RulePill({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between text-xs">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium">{value}</span>
    </div>
  );
}

// ── Deck description helpers ───────────────────────────────────────

const COLOR_PIP: Record<string, string> = {
  W: "bg-amber-100 border border-amber-400",
  U: "bg-blue-500",
  B: "bg-gray-800 border border-gray-600",
  R: "bg-red-500",
  G: "bg-green-600",
};

function ColorPip({ color }: { color: string }) {
  return (
    <span
      className={cn("inline-block w-2.5 h-2.5 rounded-full shrink-0", COLOR_PIP[color] ?? "bg-gray-400")}
      title={color}
    />
  );
}

/** Extract unique WUBRG colors present in a card list, in canonical order. */
function getDeckColors(cards: { color: string }[]): string[] {
  const seen = new Set<string>();
  for (const card of cards) {
    for (const ch of card.color) {
      if ("WUBRG".includes(ch)) seen.add(ch);
    }
  }
  return "WUBRG".split("").filter((c) => seen.has(c));
}

/** Short card-type breakdown string, e.g. "14 creatures · 8 spells · 20 lands". */
function getDeckTypeBreakdown(cards: { types?: string[] }[]): string {
  if (cards.length === 0) return "Empty deck";
  const creatures = cards.filter((c) => c.types?.includes("Creature")).length;
  const lands = cards.filter((c) => c.types?.includes("Land")).length;
  const spells = cards.length - creatures - lands;
  const parts: string[] = [];
  if (creatures > 0) parts.push(`${creatures} creature${creatures === 1 ? "" : "s"}`);
  if (spells > 0) parts.push(`${spells} spell${spells === 1 ? "" : "s"}`);
  if (lands > 0) parts.push(`${lands} land${lands === 1 ? "" : "s"}`);
  return parts.join(" · ");
}
