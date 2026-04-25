import { useEffect, useRef } from "react";
import { useLocation } from "react-router-dom";
import { useGameStore } from "@/stores/useGameStore";
import { DeckVsSelector } from "@/components/lobby/DeckVsSelector";
import Game from "./Game";
import type { PlayerDeckInfo } from "@/types/server";
import type { GameView } from "@/types/openmagic";

interface MultiplayerLocationState {
  multiplayer: true;
  playerOrder: string[];
  playerDecks: PlayerDeckInfo[];
  isHost: boolean;
  startingLife: number;
  myPlayerSlot: string;
}

interface ManualTabletopLocationState {
  manualTabletop: true;
  playerOrder: string[];
  isHost: boolean;
  startingLife: number;
  myPlayerSlot: string;
  initialGameView?: GameView;
}

export default function Play() {
  const location = useLocation();
  const {
    isGameActive,
    startGame,
    startManualTabletopGame,
    startManualRoomClient,
    startMultiplayerGame,
    setMultiplayerState,
  } = useGameStore();
  const multiplayerStarted = useRef(false);

  const routeState = location.state as
    | MultiplayerLocationState
    | ManualTabletopLocationState
    | null;
  const mpState = routeState && "multiplayer" in routeState ? routeState : null;
  const tabletopState = routeState && "manualTabletop" in routeState ? routeState : null;

  // Handle multiplayer game start from lobby navigation
  useEffect(() => {
    if (!mpState?.multiplayer || multiplayerStarted.current) return;
    multiplayerStarted.current = true;

    const { playerOrder, playerDecks, isHost, startingLife, myPlayerSlot } = mpState;
    const engineIndex = parseInt(myPlayerSlot.replace("player-", ""), 10);
    if (Number.isNaN(engineIndex) || engineIndex < 0) return;
    const deckListsByPlayer = playerOrder.map((playerName) => {
      const selected = (playerDecks ?? []).find((entry) => entry.username === playerName);
      return selected?.deck_list ?? [];
    });
    const commanderNamesByPlayer = playerOrder.map((playerName) => {
      const selected = (playerDecks ?? []).find((entry) => entry.username === playerName);
      return selected?.commander_name ?? null;
    });
    setMultiplayerState(true, isHost, myPlayerSlot);
    startMultiplayerGame(
      playerOrder,
      deckListsByPlayer,
      commanderNamesByPlayer,
      engineIndex,
      isHost,
      startingLife,
    );
  }, [mpState, setMultiplayerState, startMultiplayerGame]);

  useEffect(() => {
    if (!tabletopState?.manualTabletop || multiplayerStarted.current) return;
    multiplayerStarted.current = true;
    if (tabletopState.isHost) {
      setMultiplayerState(true, true, tabletopState.myPlayerSlot);
      return;
    }
    void startManualRoomClient(tabletopState.myPlayerSlot, tabletopState.initialGameView);
  }, [setMultiplayerState, startManualRoomClient, tabletopState]);

  if (isGameActive) {
    return (
      <div className="h-full min-h-0 no-scrollbar">
        <Game />
      </div>
    );
  }

  // Multiplayer: show waiting state while game starts
  if (mpState?.multiplayer || tabletopState?.manualTabletop) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <div className="text-center space-y-2">
          <h1 className="text-2xl font-bold">
            {tabletopState?.manualTabletop
              ? "Starting tabletop room..."
              : "Starting multiplayer game..."}
          </h1>
          <p className="text-muted-foreground">Waiting for game synchronization...</p>
        </div>
      </div>
    );
  }

  // Single-player: fighting-game style deck selector
  return (
    <div className="relative h-full min-h-0">
      <DeckVsSelector
        onStart={(playerDeck, opponentDeck, formatId, commanderName) => {
          startGame(playerDeck, formatId, commanderName, opponentDeck);
        }}
        onStartTabletop={(deck) => void startManualTabletopGame(deck)}
      />
    </div>
  );
}
