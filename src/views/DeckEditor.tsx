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
import { DROP_ZONE } from "@/lib/constants";
import { useState } from "react";
import type { Card as XMageCard } from "@/types/xmage";
import { Card } from "@/components/game/Card";

export default function DeckEditor() {
  const { addToMain, addToSide, removeFromMain, removeFromSide, currentDeck, tagCard } = useDeckStore();
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
    const cardName = (dragData.name as string) ?? card.name;

    if (overId.startsWith(DROP_ZONE.TAG_PREFIX)) {
      const tag = overId.slice(DROP_ZONE.TAG_PREFIX.length);
      tagCard(cardName, tag);
    } else if (overId === DROP_ZONE.SIDE) {
      const copies = currentDeck.cards.filter((c) => c.name === cardName);
      for (const c of copies) {
        removeFromMain(c.id);
        addToSide({ ...c, id: crypto.randomUUID() });
      }
    } else if (overId === DROP_ZONE.MAIN) {
      const copies = currentDeck.sideboard.filter((c) => c.name === cardName);
      for (const c of copies) {
        removeFromSide(c.id);
        addToMain({ ...c, id: crypto.randomUUID() });
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
  );
}
