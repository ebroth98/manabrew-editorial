import { useEffect, useRef } from "react";
import { useLocation } from "react-router-dom";
import { useGameStore } from "@/stores/useGameStore";
import { DeckVsSelector } from "@/components/lobby/DeckVsSelector";
import Game from "./Game";
import type { PlayerDeckInfo } from "@/types/server";

interface MultiplayerLocationState {
  multiplayer: true;
  playerOrder: string[];
  playerDecks: PlayerDeckInfo[];
  isHost: boolean;
  startingLife: number;
  myPlayerSlot: string;
}

export default function Play() {
  const location = useLocation();
  const { isGameActive, startGame, startMultiplayerGame, setMultiplayerState } = useGameStore();
  const multiplayerStarted = useRef(false);

  const mpState = location.state as MultiplayerLocationState | null;

  // Handle multiplayer game start from lobby navigation
  useEffect(() => {
    if (!mpState?.multiplayer || multiplayerStarted.current) return;
    multiplayerStarted.current = true;

    const { playerOrder, playerDecks, isHost, startingLife, myPlayerSlot } = mpState;
    const engineIndex = parseInt(myPlayerSlot.replace('player-', ''), 10);
    if (Number.isNaN(engineIndex) || engineIndex < 0) return;
    const deckListsByPlayer = playerOrder.map((playerName) => {
      const selected = (playerDecks ?? []).find((entry) => entry.username === playerName);
      return selected?.deck_list ?? [];
    });
    setMultiplayerState(true, isHost, myPlayerSlot);
    startMultiplayerGame(playerOrder, deckListsByPlayer, engineIndex, isHost, startingLife);
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
            Waiting for game synchronization...
          </p>
        </div>
      </div>
    );
  }

  // Single-player: fighting-game style deck selector
  return (
    <DeckVsSelector
      onStart={(playerDeck, opponentDeck) => {
        startGame(playerDeck, undefined, undefined, opponentDeck);
      }}
    />
  );
}
