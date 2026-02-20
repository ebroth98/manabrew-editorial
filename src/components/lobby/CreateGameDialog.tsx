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

const FORMATS = ["Standard", "Modern", "Legacy", "Vintage", "Commander", "Pioneer", "Pauper", "Draft", "Sealed"];
const DECK_TYPES = ["Constructed", "EDH", "Limited"];

interface CreateGameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CreateGameDialog({ open, onOpenChange }: CreateGameDialogProps) {
  const { savedDecks, currentDeck } = useDeckStore();
  const [gameName, setGameName] = useState("");
  const [format, setFormat] = useState("Standard");
  const [deckType, setDeckType] = useState("Constructed");
  const [startingLife, setStartingLife] = useState(20);
  const [numPlayers, setNumPlayers] = useState(2);
  const [selectedDeck, setSelectedDeck] = useState<string>("current");

  function handleCreate() {
    if (!gameName.trim()) {
      toast.error("Please enter a game name");
      return;
    }
    // TODO: wire up to WebSocket/middleware
    toast.success(`Game "${gameName}" created`);
    onOpenChange(false);
    setGameName("");
  }

  const allDecks = [
    { id: "current", name: `${currentDeck.name} (current)`, cardCount: currentDeck.cards.length },
    ...savedDecks.map((s) => ({ id: s.id, name: s.deck.name, cardCount: s.deck.cards.length })),
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Create New Game</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="space-y-1">
            <Label htmlFor="gameName">Game Name</Label>
            <Input
              id="gameName"
              placeholder="My Game"
              value={gameName}
              onChange={(e) => setGameName(e.target.value)}
            />
          </div>

          <div className="space-y-1">
            <Label>Format</Label>
            <div className="flex flex-wrap gap-1">
              {FORMATS.map((f) => (
                <Button
                  key={f}
                  size="sm"
                  variant={format === f ? "secondary" : "outline"}
                  className="h-7 text-xs"
                  onClick={() => setFormat(f)}
                >
                  {f}
                </Button>
              ))}
            </div>
          </div>

          <div className="space-y-1">
            <Label>Deck Type</Label>
            <div className="flex gap-1">
              {DECK_TYPES.map((d) => (
                <Button
                  key={d}
                  size="sm"
                  variant={deckType === d ? "secondary" : "outline"}
                  className="h-7 text-xs"
                  onClick={() => setDeckType(d)}
                >
                  {d}
                </Button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1">
              <Label htmlFor="startingLife">Starting Life</Label>
              <Input
                id="startingLife"
                type="number"
                min={1}
                max={40}
                value={startingLife}
                onChange={(e) => setStartingLife(Number(e.target.value))}
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="numPlayers">Players</Label>
              <Input
                id="numPlayers"
                type="number"
                min={2}
                max={8}
                value={numPlayers}
                onChange={(e) => setNumPlayers(Number(e.target.value))}
              />
            </div>
          </div>

          <div className="space-y-1">
            <Label>Deck</Label>
            <div className="space-y-1 max-h-32 overflow-y-auto border rounded p-2">
              {allDecks.map((d) => (
                <div
                  key={d.id}
                  className={`flex items-center justify-between p-1.5 rounded cursor-pointer text-sm ${
                    selectedDeck === d.id ? "bg-secondary" : "hover:bg-muted"
                  }`}
                  onClick={() => setSelectedDeck(d.id)}
                >
                  <span className="truncate">{d.name}</span>
                  <span className="text-xs text-muted-foreground shrink-0 ml-2">{d.cardCount} cards</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={handleCreate}>Create Game</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
