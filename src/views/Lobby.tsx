import { TablesList } from "@/components/lobby/TablesList";
import { UserList } from "@/components/lobby/UserList";
import { ChatComponent } from "@/components/lobby/ChatComponent";
import { CreateRoomDialog } from "@/components/lobby/CreateRoomDialog";
import { CreateGameDialog } from "@/components/lobby/CreateGameDialog";
import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useServerStore } from "@/stores/useServerStore";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import type { CardIdentity } from "@/types/server";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { Wifi, WifiOff, Loader2, Settings, RefreshCw, MessageSquare, Users } from "lucide-react";

export default function Lobby() {
  const navigate = useNavigate();
  const {
    connected, connecting, error, username, rooms, currentRoom, players,
    gameStarted, playerOrder, playerDecks, startingLife,
    connect, listRooms, listPlayers, joinRoom, leaveRoom,
    setDeckSelection,
    setReady, startGame,
  } = useServerStore();
  const prefs = usePreferencesStore();
  const [createRoomOpen, setCreateRoomOpen] = useState(false);
  const [deckDialogOpen, setDeckDialogOpen] = useState(false);
  const [refreshingLobby, setRefreshingLobby] = useState(false);
  const [sidePanel, setSidePanel] = useState<"chat" | "players" | null>(null);

  useEffect(() => {
    if (!connected && !connecting && prefs.serverUsername) {
      connect(prefs.serverHost, prefs.serverPort, prefs.serverUsername, prefs.serverPassword);
    }
  }, []);

  // Poll lobby data every 5s while connected
  useEffect(() => {
    if (!connected) return;
    const id = setInterval(() => {
      listRooms();
      listPlayers();
    }, 5000);
    return () => clearInterval(id);
  }, [connected, listRooms, listPlayers]);

  useEffect(() => {
    if (gameStarted && playerOrder.length > 0) {
      const isHost = currentRoom?.host === username;
      const myIndex = playerOrder.indexOf(username ?? '');
      if (myIndex < 0) {
        toast.error("Could not determine your player slot for this game.");
        return;
      }
      useServerStore.setState({ gameStarted: false });
      navigate('/play', {
        state: {
          multiplayer: true,
          playerOrder,
          playerDecks,
          isHost,
          startingLife,
          myPlayerSlot: `player-${myIndex}`,
        },
      });
    }
  }, [gameStarted]);

  async function refreshLobbyData() {
    if (!connected || refreshingLobby) return;
    setRefreshingLobby(true);
    try {
      await Promise.all([listRooms(), listPlayers()]);
    } finally {
      setRefreshingLobby(false);
    }
  }

  async function handleDeckSelection(deckName: string, deckList: CardIdentity[], commanderName?: string) {
    try {
      await setDeckSelection(deckName, deckList, commanderName);
      toast.success(`Selected deck: ${deckName}`);
    } catch (error) {
      toast.error(`Failed to set deck: ${String(error)}`);
    }
  }

  return (
    <div className="h-full w-full flex flex-col">
      {/* ── Header ── */}
      <div className="px-4 py-3 border-b shrink-0 flex items-center gap-3">
        <div className="flex-1" />

        {/* Connection status */}
        <div className={cn(
          "flex items-center gap-2 text-xs px-2.5 py-1 rounded-full border",
          connected && "text-primary border-primary/30 bg-primary/5",
          !connected && error && "text-destructive border-destructive/30 bg-destructive/5",
          !connected && !error && "text-muted-foreground border-border",
        )}>
          {connecting ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : connected ? (
            <Wifi className="h-3 w-3" />
          ) : (
            <WifiOff className="h-3 w-3" />
          )}
          <span>
            {connecting ? "Connecting..." : connected ? username : error ? "Disconnected" : "Not connected"}
          </span>
        </div>

        {!connected && error && (
          <Button size="sm" variant="outline" className="h-7 text-xs" onClick={() => connect(prefs.serverHost, prefs.serverPort, prefs.serverUsername, prefs.serverPassword)}>
            Retry
          </Button>
        )}
        {!connected && !connecting && (
          <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => navigate('/settings')}>
            <Settings className="h-3 w-3 mr-1" /> Settings
          </Button>
        )}

        {connected && (
          <div className="flex items-center gap-1">
            <Button
              size="sm"
              variant="ghost"
              className="h-7 text-xs"
              onClick={refreshLobbyData}
              disabled={refreshingLobby}
            >
              <RefreshCw className={cn("h-3 w-3 mr-1", refreshingLobby && "animate-spin")} />
              Refresh
            </Button>
            <Button size="sm" className="h-7 text-xs" onClick={() => setCreateRoomOpen(true)} disabled={currentRoom != null}>
              New Room
            </Button>
            <div className="w-px h-4 bg-border mx-1" />
            <Button
              size="icon"
              variant={sidePanel === "chat" ? "secondary" : "ghost"}
              className="h-7 w-7 relative"
              title="Toggle chat"
              onClick={() => setSidePanel((v) => v === "chat" ? null : "chat")}
            >
              <MessageSquare className="h-3.5 w-3.5" />
            </Button>
            <Button
              size="icon"
              variant={sidePanel === "players" ? "secondary" : "ghost"}
              className="h-7 w-7 relative"
              title="Toggle online players"
              onClick={() => setSidePanel((v) => v === "players" ? null : "players")}
            >
              <Users className="h-3.5 w-3.5" />
              {players.length > 0 && (
                <span className="absolute -top-0.5 -right-0.5 bg-primary text-primary-foreground text-[8px] rounded-full w-3.5 h-3.5 flex items-center justify-center">
                  {players.length}
                </span>
              )}
            </Button>
          </div>
        )}
      </div>

      {/* ── Main content ── */}
      <div className="flex-1 min-h-0 flex">
        {/* Rooms — takes full width when panels are closed */}
        <div className="flex-1 min-w-0 h-full">
          <TablesList
            rooms={rooms}
            currentRoom={currentRoom}
            username={username}
            onNewGame={() => setCreateRoomOpen(true)}
            onRefresh={refreshLobbyData}
            refreshing={refreshingLobby}
            refreshDisabled={!connected || connecting}
            onJoinRoom={joinRoom}
            onLeaveRoom={leaveRoom}
            onSetReady={setReady}
            onOpenDeckDialog={() => setDeckDialogOpen(true)}
            onStartGame={startGame}
          />
        </div>

        {/* Toggleable side panel */}
        {sidePanel && (
          <div className="w-72 shrink-0 border-l h-full">
            {sidePanel === "chat" && <ChatComponent channelId="Lobby" />}
            {sidePanel === "players" && <UserList players={players} />}
          </div>
        )}
      </div>

      <CreateRoomDialog open={createRoomOpen} onOpenChange={setCreateRoomOpen} />
      <CreateGameDialog
        open={deckDialogOpen}
        onOpenChange={setDeckDialogOpen}
        mode="lobby"
        forcedFormatId={currentRoom?.format ? currentRoom.format.toLowerCase() : "standard"}
        onStart={(deckList, _formatId, commanderName, _playerCount, deckName) => {
          void handleDeckSelection(deckName ?? "Selected Deck", deckList, commanderName);
        }}
      />
    </div>
  );
}
