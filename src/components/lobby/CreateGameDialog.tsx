import { useState, useEffect } from "react";
import { tauriApi, type PresetDeckInfo } from "@/api/tauri";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import { useDeckStore } from "@/stores/useDeckStore";
import { GAME_FORMATS, validateDeck, type GameFormat } from "@/lib/formats";
import { FormatBadge } from "@/components/game/FormatBadge";
import { DeckSelectionCard } from "./DeckSelectionCard";
import { cn } from "@/lib/utils";
import { Search, Shuffle, Swords } from "lucide-react";

interface CreateGameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Visual mode: full game creation (Play) or deck-only selection (Lobby). */
  mode?: "play" | "lobby";
  /** Optional fixed format id (e.g. "constructed" | "commander"). */
  forcedFormatId?: string;
  /** Pre-select a saved deck by ID (e.g. when launched from MyDecks) */
  preSelectedDeckId?: string;
  /** Called with the deck card names, format ID, optional commander name, and player count when Create is confirmed */
  onStart: (
    deckList: { name: string, setCode: string }[],
    formatId: string,
    commanderName?: string,
    playerCount?: number,
    deckName?: string
  ) => void;
}

export function CreateGameDialog({
  open,
  onOpenChange,
  mode = "play",
  forcedFormatId,
  preSelectedDeckId,
  onStart,
}: CreateGameDialogProps) {
  const { savedDecks, currentDeck } = useDeckStore();
  const isLobbyMode = mode === "lobby";

  const initialFormat =
    GAME_FORMATS.find((f) => f.id === forcedFormatId) ?? GAME_FORMATS[0];
  const [selectedFormat, setSelectedFormat] = useState<GameFormat>(initialFormat);
  const [selectedDeck, setSelectedDeck] = useState<string>(preSelectedDeckId ?? "current");
  const [selectedCommander, setSelectedCommander] = useState<string>(
    currentDeck.commander?.name ?? ""
  );
  const [presetDecks, setPresetDecks] = useState<PresetDeckInfo[]>([]);
  const [playerCount, setPlayerCount] = useState(2);
  const [deckSearch, setDeckSearch] = useState("");

  useEffect(() => {
    tauriApi.deck.getPresetDecks()
      .then(setPresetDecks)
      .catch((e) => console.error("[CreateGameDialog] Failed to load preset decks:", e));
  }, []);

  useEffect(() => {
    if (!forcedFormatId) return;
    const forced = GAME_FORMATS.find((f) => f.id === forcedFormatId);
    if (forced) setSelectedFormat(forced);
  }, [forcedFormatId]);

  // User-built decks
  const userDecks = [
    {
      id: "current",
      name: currentDeck.name,
      badge: "editing",
      labels: currentDeck.labels,
      deckList: [
        ...currentDeck.cards.map((c) => ({ name: c.name, setCode: c.setCode || "" })),
        ...(currentDeck.commander ? [{ name: currentDeck.commander.name, setCode: currentDeck.commander.setCode || "" }] : []),
      ],
      isPreset: false as const,
      cards: currentDeck.cards,
      commanderName: currentDeck.commander?.name,
    },
    ...savedDecks.map((s) => ({
      id: s.id,
      name: s.deck.name,
      badge: null as string | null,
      labels: s.deck.labels,
      deckList: [
        ...s.deck.cards.map((c) => ({ name: c.name, setCode: c.setCode || "" })),
        ...(s.deck.commander ? [{ name: s.deck.commander.name, setCode: s.deck.commander.setCode || "" }] : []),
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
    deckList: [{ name: deck.id, setCode: "" }],
    isPreset: true as const,
    cards: [],
    commanderName: undefined as string | undefined,
  }));

  const allDecks = [...userDecks, ...presetDeckEntries];

  // Filter decks by search query (matches name or description)
  const searchLower = deckSearch.toLowerCase();
  const filteredPresetEntries = searchLower
    ? presetDeckEntries.filter(
        (d) =>
          d.name.toLowerCase().includes(searchLower) ||
          d.desc?.toLowerCase().includes(searchLower),
      )
    : presetDeckEntries;
  const filteredUserDecks = searchLower
    ? userDecks.filter((d) => d.name.toLowerCase().includes(searchLower))
    : userDecks;

  // Auto-populate commander when the selected deck changes
  useEffect(() => {
    const entry = allDecks.find((d) => d.id === selectedDeck);
    setSelectedCommander(entry?.commanderName ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedDeck]);

  const selectedDeckEntry = allDecks.find((d) => d.id === selectedDeck);
  const selectedDeckList = selectedDeckEntry?.deckList ?? [];
  const selectedDeckValidation = selectedDeckEntry?.isPreset
    ? { legal: true, errors: [] as string[] }
    : validateDeck(selectedDeckList.map(c => c.name), selectedFormat);

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

  const needsCommander = !isLobbyMode && selectedFormat.deckRules.requiresCommander;
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
    onStart(
      selectedDeckList,
      selectedFormat.id,
      selectedFormat.deckRules.requiresCommander
        ? (needsCommander ? selectedCommander : selectedDeckEntry.commanderName)
        : undefined,
      playerCount,
      selectedDeckEntry.name,
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl p-0 gap-0 overflow-hidden">

        {/* ── Header ── */}
        <div className="px-6 py-4 border-b">
          <DialogTitle className="text-lg font-semibold">
            {isLobbyMode ? "Choose Deck" : "New Game"}
          </DialogTitle>
          <p className="text-sm text-muted-foreground mt-0.5">
            {isLobbyMode
              ? "Select the deck you will play in this lobby."
              : "Pick a deck and battle a random AI opponent"}
          </p>
        </div>

        {/* ── Body: left panel (settings) + right panel (deck picker) ── */}
        <div className="flex overflow-hidden" style={{ maxHeight: "65vh" }}>

          {/* Left panel — Format & options */}
          {!isLobbyMode && (
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
                <span className="ml-1 text-[9px] font-mono text-warning bg-warning/10 px-1 rounded">
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
                        ? "border-warning bg-warning/10 text-warning font-semibold"
                        : "border-border hover:bg-muted/60"
                    )}
                  >
                    {n - 1}v1
                  </button>
                ))}
              </div>
              </div>
            </div>
          )}

          {/* Right panel — Deck picker */}
          <div className="flex-1 overflow-y-auto">

            {/* Search bar */}
            <div className="px-4 pt-4 pb-2 sticky top-0 bg-background z-10">
              <div className="relative">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground pointer-events-none" />
                <input
                  type="text"
                  placeholder="Filter decks..."
                  value={deckSearch}
                  onChange={(e) => setDeckSearch(e.target.value)}
                  className="w-full pl-8 pr-3 py-1.5 rounded-md border bg-background text-sm focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>
            </div>

            {/* Preset decks */}
            <div className="p-4 pt-2">
              <SectionLabel>Preset Decks</SectionLabel>
              <p className="text-[11px] text-muted-foreground mt-0.5 mb-3">
                Pre-built themed decks — always legal, great for testing mechanics.
              </p>
              {filteredPresetEntries.length === 0 ? (
                <p className="text-xs text-muted-foreground italic">
                  No preset decks match your search.
                </p>
              ) : (
              <div className="grid grid-cols-3 gap-2">
                {filteredPresetEntries.map((deck) => (
                  <DeckSelectionCard
                    key={deck.id}
                    id={deck.id}
                    name={deck.name}
                    desc={deck.desc}
                    color={deck.color}
                    deckList={deck.deckList}
                    cards={deck.cards}
                    isPreset={deck.isPreset}
                    isSelected={selectedDeck === deck.id}
                    isLegal={true}
                    onSelect={() => setSelectedDeck(deck.id)}
                  />
                ))}
              </div>
              )}
            </div>

            {/* Divider */}
            <div className="mx-4 border-t" />

            {/* User decks */}
            <div className="p-4">
              <SectionLabel>Your Decks</SectionLabel>
              <p className="text-[11px] text-muted-foreground mt-0.5 mb-3">
                Decks you've built in the editor.
              </p>
              {filteredUserDecks.length === 0 ? (
                <p className="text-xs text-muted-foreground italic">
                  {searchLower ? "No saved decks match your search." : "No saved decks. Build one in the Deck Editor."}
                </p>
              ) : (
                <div className="grid grid-cols-3 gap-2">
                  {filteredUserDecks.map((d) => {
                    const validation = validateDeck(d.deckList.map((c) => c.name), selectedFormat);
                    return (
                      <DeckSelectionCard
                        key={d.id}
                        id={d.id}
                        name={d.name}
                        badge={d.badge}
                        labels={d.labels}
                        deckList={d.deckList}
                        cards={d.cards}
                        isPreset={d.isPreset}
                        isSelected={selectedDeck === d.id}
                        isLegal={validation.legal}
                        validationError={validation.errors[0]}
                        onSelect={() => setSelectedDeck(d.id)}
                      />
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
            {!isLobbyMode && selectedDeckEntry ? (
              <>
                <span className="text-muted-foreground shrink-0">Playing</span>
                <span className="font-medium truncate">{selectedDeckEntry.name}</span>
                <span className="text-muted-foreground shrink-0">vs</span>
                <span className="inline-flex items-center gap-1 text-muted-foreground shrink-0">
                  <Shuffle className="h-3 w-3" />
                  Random AI
                </span>
              </>
            ) : selectedDeckEntry ? (
              <span className="text-sm text-muted-foreground truncate">
                Selected: <span className="font-medium text-foreground">{selectedDeckEntry.name}</span>
              </span>
            ) : (
              <span className="text-muted-foreground italic text-xs">No deck selected</span>
            )}
          </div>
          <div className="flex gap-2 shrink-0">
            <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button size="sm" onClick={handleCreate} disabled={!isReady} className="gap-1.5">
              {!isLobbyMode && <Swords className="h-3.5 w-3.5" />}
              {isLobbyMode ? "Select Deck" : "Play"}
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


