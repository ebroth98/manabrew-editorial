import { useDeckStore } from "@/stores/useDeckStore";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card } from "@/components/game/Card";
import { Button } from "@/components/ui/button";
import { X } from "lucide-react";
import { DeckStats } from "./DeckStats";

export function DeckBuilder() {
  const { currentDeck, removeFromMain, removeFromSide } = useDeckStore();

  return (
    <div className="flex flex-col h-full w-full">
      <div className="p-4 border-b">
        <h2 className="text-lg font-bold">{currentDeck.name || "New Deck"}</h2>
        <div className="flex gap-4 text-sm text-muted-foreground">
          <span>Main: {currentDeck.cards.length}</span>
          <span>Side: {currentDeck.sideboard.length}</span>
        </div>
      </div>
      
      <ScrollArea className="flex-1 p-4">
        <div className="space-y-6">
          <div>
            <h3 className="font-semibold mb-2">Mainboard</h3>
            <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
              {currentDeck.cards.map((card, index) => (
                <div key={`${card.id}-${index}`} className="relative group">
                  <Card card={card} className="w-full h-auto aspect-[5/7]" />
                  <Button
                    size="icon"
                    variant="destructive"
                    className="absolute -top-2 -right-2 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => removeFromMain(card.id)}
                  >
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              ))}
            </div>
          </div>

          <div>
            <h3 className="font-semibold mb-2">Sideboard</h3>
            <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
              {currentDeck.sideboard.map((card, index) => (
                <div key={`${card.id}-${index}`} className="relative group">
                  <Card card={card} className="w-full h-auto aspect-[5/7]" />
                  <Button
                    size="icon"
                    variant="destructive"
                    className="absolute -top-2 -right-2 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => removeFromSide(card.id)}
                  >
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              ))}
            </div>
          </div>
        </div>
      </ScrollArea>
      <DeckStats />
    </div>
  );
}
