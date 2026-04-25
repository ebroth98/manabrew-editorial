import { DeckBuilder, useDeckUnsavedChanges, revertDeckToLastSaved } from "@/components/editor/DeckBuilder";
import { CardSearch } from "@/components/editor/CardSearch";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  pointerWithin,
} from "@dnd-kit/core";
import type { DragEndEvent, DragStartEvent } from "@dnd-kit/core";
import { useDeckStore } from "@/stores/useDeckStore";
import { DROP_ZONE, DEFAULT_DECK_NAME } from "@/lib/constants";
import { useState } from "react";
import type { Card as XMageCard } from "@/types/openmagic";
import { Card } from "@/components/game/Card";
import { useBlocker, useLocation } from "react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { DeckGridCard } from "@/components/deck/DeckGridCard";
import { DeckListControls } from "@/components/deck/DeckListControls";
import { cn } from "@/lib/utils";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import { applyDeckFilters } from "@/views/myDecks.utils";
import type { SortBy } from "@/views/myDecks.utils";

export default function DeckEditor() {
  const {
    addToMain, addToSide, removeFromMain, removeFromSide,
    currentDeck, tagCard, untagCard,
    savedDecks, loadSavedDeck, clearDeck, setDeckName, deleteSavedDeck,
    currentDeckId: _currentDeckId,
  } = useDeckStore();
  const [draggedCard, setDraggedCard] = useState<XMageCard | null>(null);
  const [showSearch, setShowSearch] = useState(false);
  const hasUnsavedChanges = useDeckUnsavedChanges();
  const location = useLocation();

  const [view, setView] = useState<"list" | "editor">(() =>
    (location.state as { directToEditor?: boolean } | null)?.directToEditor ? "editor" : "list"
  );
  const [showBackConfirm, setShowBackConfirm] = useState(false);

  // Deck list filter/sort state
  const [search, setSearch] = useState("");
  const [formatFilter, setFormatFilter] = useState("");
  const [colorFilter, setColorFilter] = useState<string[]>([]);
  const [sortBy, setSortBy] = useState<SortBy>("name");

  // Rename dialog state
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameInput, setRenameInput] = useState("");

  const blocker = useBlocker(hasUnsavedChanges && view === "editor");

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } })
  );

  const supplementaryCards = [
    ...currentDeck.sideboard,
    ...(currentDeck.attractions ?? []),
    ...(currentDeck.contraptions ?? []),
    ...(currentDeck.schemes ?? []),
    ...(currentDeck.planes ?? []),
  ];

  // ── Deck list handlers ────────────────────────────────────────────────────

  function toggleColor(color: string) {
    setColorFilter((prev) =>
      prev.includes(color) ? prev.filter((c) => c !== color) : [...prev, color]
    );
  }

  const { valid: filteredValid, drafts: filteredDrafts } = applyDeckFilters(savedDecks, {
    search, formatFilter, colorFilter, sortBy,
  });

  function handleSelectDeck(id: string) {
    loadSavedDeck(id);
    setView("editor");
  }

  function handleNewDeck() {
    clearDeck();
    setDeckName(DEFAULT_DECK_NAME);
    setView("editor");
  }

  function handleBack() {
    if (hasUnsavedChanges) {
      setShowBackConfirm(true);
    } else {
      setView("list");
    }
  }

  function handleDelete(id: string) {
    deleteSavedDeck(id);
    toast.success("Deck deleted");
  }

  function startRename(id: string, name: string) {
    setRenamingId(id);
    setRenameInput(name);
  }

  function confirmRename() {
    if (!renamingId || !renameInput.trim()) return;
    useDeckStore.setState((state) => ({
      savedDecks: state.savedDecks.map((s) =>
        s.id === renamingId ? { ...s, deck: { ...s.deck, name: renameInput.trim() } } : s,
      ),
    }));
    setRenamingId(null);
    toast.success("Deck renamed");
  }

  // ── DnD handlers ─────────────────────────────────────────────────────────

  function handleDragStart(event: DragStartEvent) {
    const data = event.active.data.current;
    if (data?.card) setDraggedCard(data.card as XMageCard);
  }

  function handleDragEnd(event: DragEndEvent) {
    setDraggedCard(null);
    const { active, over } = event;
    if (!over) return;

    const dragData = active.data.current;
    if (!dragData?.card) return;

    const card = dragData.card as XMageCard;
    const overId = String(over.id);
    const activeId = String(active.id);
    const cardName = (dragData.name as string) ?? card.name;

    const sourceTagMatch = activeId.match(/^deck-tag-(.+?)-(?:.+)$/);
    const sourceTag = sourceTagMatch?.[1] ?? null;

    if (overId.startsWith(DROP_ZONE.TAG_PREFIX)) {
      const destTag = overId.slice(DROP_ZONE.TAG_PREFIX.length);
      if (sourceTag && sourceTag !== destTag) {
        untagCard(cardName, sourceTag);
      }
      tagCard(cardName, destTag);
    } else if (overId === DROP_ZONE.SIDE) {
      if (sourceTag) untagCard(cardName, sourceTag);
      const copies = currentDeck.cards.filter((c) => c.name === cardName);
      for (const c of copies) {
        removeFromMain(c.id);
        addToSide({ ...c, id: crypto.randomUUID() });
      }
    } else if (overId === DROP_ZONE.MAIN) {
      if (sourceTag) untagCard(cardName, sourceTag);
      const copies = supplementaryCards.filter((c) => c.name === cardName);
      for (const c of copies) {
        removeFromSide(c.id);
        addToMain({ ...c, id: crypto.randomUUID() });
      }
    }
  }

  // ── List view ─────────────────────────────────────────────────────────────

  if (view === "list") {
    return (
      <>
        <div className="h-full flex flex-col">
          <div className="px-4 py-3 border-b shrink-0 flex items-center">
            <h2 className="text-lg font-semibold flex-1">My Decks</h2>
          </div>

          <DeckListControls
            search={search}
            onSearchChange={setSearch}
            formatFilter={formatFilter}
            onFormatChange={setFormatFilter}
            colorFilter={colorFilter}
            onColorToggle={toggleColor}
            sortBy={sortBy}
            onSortChange={setSortBy}
          />

          <ScrollArea className="flex-1">
            <div className="p-4">
              {/* Grid: first cell = New Deck button, then valid decks */}
              <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
                {/* New Deck slot */}
                <button
                  type="button"
                  onClick={handleNewDeck}
                  className={cn(
                    "aspect-[4/3] rounded-lg border-2 border-dashed border-muted-foreground/30",
                    "flex flex-col items-center justify-center gap-1.5",
                    "text-muted-foreground hover:text-foreground hover:border-primary",
                    "transition-all cursor-pointer bg-muted/30 hover:bg-muted/60",
                  )}
                >
                  <Plus className="h-6 w-6" />
                  <span className="text-xs font-medium">New Deck</span>
                </button>

                {filteredValid.map((s) => (
                  <DeckGridCard
                    key={s.id}
                    deck={s}
                    onOpen={() => handleSelectDeck(s.id)}
                    onDelete={() => handleDelete(s.id)}
                    onRename={() => startRename(s.id, s.deck.name)}
                  />
                ))}
              </div>

              {/* Drafts section */}
              {filteredDrafts.length > 0 && (
                <div className={cn("mt-4", filteredValid.length > 0 && "border-t pt-4")}>
                  <div className="flex items-center gap-2 mb-3">
                    <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                      Drafts
                    </span>
                    <span className="text-[10px] text-muted-foreground">
                      ({filteredDrafts.length})
                    </span>
                  </div>
                  <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
                    {filteredDrafts.map((s) => (
                      <DeckGridCard
                        key={s.id}
                        deck={s}
                        onOpen={() => handleSelectDeck(s.id)}
                        onDelete={() => handleDelete(s.id)}
                        onRename={() => startRename(s.id, s.deck.name)}
                      />
                    ))}
                  </div>
                </div>
              )}

              {/* Empty state */}
              {filteredValid.length === 0 && filteredDrafts.length === 0 && savedDecks.length > 0 && (
                <p className="col-span-5 pt-6 text-center text-sm text-muted-foreground">
                  No decks match your filters.
                </p>
              )}
            </div>
          </ScrollArea>
        </div>

        {/* Rename dialog */}
        <Dialog open={renamingId !== null} onOpenChange={(open) => { if (!open) setRenamingId(null); }}>
          <DialogContent className="max-w-sm">
            <DialogHeader>
              <DialogTitle>Rename Deck</DialogTitle>
            </DialogHeader>
            <Input
              value={renameInput}
              onChange={(e) => setRenameInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") confirmRename(); }}
              placeholder="Deck name"
              autoFocus
            />
            <DialogFooter className="gap-2">
              <Button variant="outline" size="sm" onClick={() => setRenamingId(null)}>
                Cancel
              </Button>
              <Button size="sm" onClick={confirmRename} disabled={!renameInput.trim()}>
                Rename
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  // ── Editor view ───────────────────────────────────────────────────────────

  return (
    <>
      <DndContext
        sensors={sensors}
        collisionDetection={pointerWithin}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div className="h-full w-full overflow-hidden flex">
          <div className={cn("h-full overflow-hidden transition-all", showSearch ? "w-1/2" : "w-full")}>
            <DeckBuilder
              onToggleSearch={() => setShowSearch((v) => !v)}
              onBack={handleBack}
            />
          </div>
          {showSearch && (
            <div className="w-1/2 h-full border-l overflow-hidden">
              <CardSearch onClose={() => setShowSearch(false)} />
            </div>
          )}
        </div>

        <DragOverlay dropAnimation={null}>
          {draggedCard && (
            <div className="w-24 opacity-90 rotate-3 shadow-2xl pointer-events-none">
              <Card card={draggedCard} className="w-full" />
            </div>
          )}
        </DragOverlay>
      </DndContext>

      {/* Unsaved changes — back to list */}
      {showBackConfirm && (
        <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-overlay/50 backdrop-blur-sm">
          <div className="bg-card border rounded-xl shadow-xl p-6 max-w-sm space-y-4">
            <h3 className="text-lg font-semibold">Unsaved Changes</h3>
            <p className="text-sm text-muted-foreground">
              You have unsaved changes to your deck. Do you want to go back without saving?
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" size="sm" onClick={() => setShowBackConfirm(false)}>
                Stay
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={() => {
                  revertDeckToLastSaved();
                  setShowBackConfirm(false);
                  setView("list");
                }}
              >
                Leave Without Saving
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Unsaved changes — route navigation */}
      {blocker.state === "blocked" && (
        <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-overlay/50 backdrop-blur-sm">
          <div className="bg-card border rounded-xl shadow-xl p-6 max-w-sm space-y-4">
            <h3 className="text-lg font-semibold">Unsaved Changes</h3>
            <p className="text-sm text-muted-foreground">
              You have unsaved changes to your deck. Do you want to leave without saving?
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" size="sm" onClick={() => blocker.reset()}>
                Stay
              </Button>
              <Button variant="destructive" size="sm" onClick={() => { revertDeckToLastSaved(); blocker.proceed(); }}>
                Leave Without Saving
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
