import { useEffect, useRef, useState } from "react";
import { useLocation } from "react-router-dom";
import { useGameStore } from "@/stores/useGameStore";
import { CreateGameDialog } from "@/components/lobby/CreateGameDialog";
import { Button } from "@/components/ui/button";
import { Swords } from "lucide-react";
import Game from "./Game";

interface MultiplayerLocationState {
  multiplayer: true;
  playerOrder: string[];
  isHost: boolean;
  myPlayerSlot: string;
}

export default function Play() {
  const location = useLocation();
  const { isGameActive, startGame, startMultiplayerGame, setMultiplayerState } = useGameStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const multiplayerStarted = useRef(false);

  const mpState = location.state as MultiplayerLocationState | null;

  // Handle multiplayer game start from lobby navigation
  useEffect(() => {
    if (!mpState?.multiplayer || multiplayerStarted.current) return;
    multiplayerStarted.current = true;

    const { playerOrder, isHost, myPlayerSlot } = mpState;

    if (isHost) {
      // Host: start the engine with all player names
      const hostIndex = parseInt(myPlayerSlot.replace('player-', ''), 10);
      startMultiplayerGame(playerOrder, hostIndex, 20);
    } else {
      // Non-host: set multiplayer state and wait for remote prompts
      setMultiplayerState(true, false, myPlayerSlot);
      // Mark game as active so the Game component renders
      useGameStore.setState({ isGameActive: true });
    }
  }, [mpState]);

  if (isGameActive) {
    return <Game />;
  }

  // Multiplayer: show waiting state while game starts
  if (mpState?.multiplayer) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <div className="text-center space-y-2">
          <h1 className="text-2xl font-bold">Starting multiplayer game...</h1>
          <p className="text-muted-foreground">
            {mpState.isHost ? "Setting up the game engine..." : "Waiting for host to start..."}
          </p>
        </div>
      </div>
    );
  }

  // Single-player: show deck selection
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
