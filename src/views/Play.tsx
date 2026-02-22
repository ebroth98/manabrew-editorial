import { useState } from "react";
import { useGameStore } from "@/stores/useGameStore";
import { CreateGameDialog } from "@/components/lobby/CreateGameDialog";
import { Button } from "@/components/ui/button";
import { Swords } from "lucide-react";
import Game from "./Game";

export default function Play() {
  const { isGameActive, startGame } = useGameStore();
  const [dialogOpen, setDialogOpen] = useState(false);

  if (isGameActive) {
    return <Game />;
  }

  return (
    <div className="flex flex-col items-center justify-center h-full gap-6">
      <div className="text-center space-y-2">
        <h1 className="text-3xl font-bold">Play vs AI</h1>
        <p className="text-muted-foreground">
          Choose a deck and battle a random AI opponent — completely offline.
        </p>
      </div>
      <Button size="lg" className="gap-2" onClick={() => setDialogOpen(true)}>
        <Swords className="h-5 w-5" />
        New Game
      </Button>
      <CreateGameDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onStart={(cardNames, formatId, commanderName) => {
          startGame(cardNames, formatId, commanderName);
        }}
      />
    </div>
  );
}
