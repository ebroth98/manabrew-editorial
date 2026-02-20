import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { CardSearch } from "@/components/editor/CardSearch";
import { DeckBuilder } from "@/components/editor/DeckBuilder";
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
import { useState } from "react";
import type { Card as XMageCard } from "@/types/xmage";
import { Card } from "@/components/game/Card";

export default function DeckEditor() {
  const { addToMain, addToSide, removeFromMain, removeFromSide, currentDeck } = useDeckStore();
  const [draggedCard, setDraggedCard] = useState<XMageCard | null>(null);

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
    const isFromSearch = dragData.type !== "deck-card";

    if (isFromSearch) {
      // Drag from CardSearch → add a new copy
      if (overId === "drop-side") {
        addToSide({ ...card, id: crypto.randomUUID() });
      } else {
        addToMain({ ...card, id: crypto.randomUUID() });
      }
    } else {
      // Drag within deck → move between main and sideboard
      const cardName = dragData.name as string;
      if (overId === "drop-side") {
        // Move all copies from main to side
        const copies = currentDeck.cards.filter((c) => c.name === cardName);
        for (const c of copies) {
          removeFromMain(c.id);
          addToSide({ ...c, id: crypto.randomUUID() });
        }
      } else if (overId === "drop-main") {
        // Move all copies from side to main
        const copies = currentDeck.sideboard.filter((c) => c.name === cardName);
        for (const c of copies) {
          removeFromSide(c.id);
          addToMain({ ...c, id: crypto.randomUUID() });
        }
      }
    }
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={pointerWithin}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <div className="h-full w-full overflow-hidden">
        <ResizablePanelGroup orientation="horizontal">
          <ResizablePanel defaultSize={45} minSize={30}>
            <CardSearch />
          </ResizablePanel>

          <ResizableHandle withHandle />

          <ResizablePanel defaultSize={55} minSize={30}>
            <DeckBuilder />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>

      {/* Drag ghost overlay */}
      <DragOverlay dropAnimation={null}>
        {draggedCard && (
          <div className="w-24 opacity-90 rotate-3 shadow-2xl pointer-events-none">
            <Card card={draggedCard} className="w-full" />
          </div>
        )}
      </DragOverlay>
    </DndContext>
  );
}
