import { DeckBuilder, useDeckUnsavedChanges } from "@/components/editor/DeckBuilder";
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
import { DROP_ZONE } from "@/lib/constants";
import { useState } from "react";
import type { Card as XMageCard } from "@/types/openmagic";
import { Card } from "@/components/game/Card";
import { useBlocker } from "react-router";
import { Button } from "@/components/ui/button";

export default function DeckEditor() {
  const { addToMain, addToSide, removeFromMain, removeFromSide, currentDeck, tagCard, untagCard } = useDeckStore();
  const [draggedCard, setDraggedCard] = useState<XMageCard | null>(null);
  const hasUnsavedChanges = useDeckUnsavedChanges();

  const blocker = useBlocker(hasUnsavedChanges);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } })
  );

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

    // Detect if dragged from a tag section (id format: deck-tag-{tag}-{cardName})
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
      const copies = currentDeck.sideboard.filter((c) => c.name === cardName);
      for (const c of copies) {
        removeFromSide(c.id);
        addToMain({ ...c, id: crypto.randomUUID() });
      }
    }
  }

  return (
    <>
      <DndContext
        sensors={sensors}
        collisionDetection={pointerWithin}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div className="h-full w-full overflow-hidden">
          <DeckBuilder />
        </div>

        <DragOverlay dropAnimation={null}>
          {draggedCard && (
            <div className="w-24 opacity-90 rotate-3 shadow-2xl pointer-events-none">
              <Card card={draggedCard} className="w-full" />
            </div>
          )}
        </DragOverlay>
      </DndContext>

      {/* Unsaved changes dialog */}
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
              <Button variant="destructive" size="sm" onClick={() => blocker.proceed()}>
                Leave Without Saving
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
