import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TablesList } from "@/components/lobby/TablesList";
import { UserList } from "@/components/lobby/UserList";
import { ChatComponent } from "@/components/lobby/ChatComponent";
import { CreateRoomDialog } from "@/components/lobby/CreateRoomDialog";
import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useServerStore } from "@/stores/useServerStore";
import { usePreferencesStore } from "@/stores/usePreferencesStore";

export default function Lobby() {
  const navigate = useNavigate();
  const {
    connected, connecting, error, username, rooms, currentRoom, players,
    gameStarted, playerOrder,
    connect, joinRoom, leaveRoom,
    setReady, startGame,
  } = useServerStore();
  const prefs = usePreferencesStore();
  const [createRoomOpen, setCreateRoomOpen] = useState(false);

  // Auto-connect on mount if not connected/connecting and username is set
  useEffect(() => {
    if (!connected && !connecting && prefs.serverUsername) {
      connect(prefs.serverHost, prefs.serverPort, prefs.serverUsername, prefs.serverPassword);
    }
  }, []);

  // Navigate to game when server starts a game
  useEffect(() => {
    if (gameStarted) {
      navigate('/play', { state: { multiplayer: true, playerOrder } });
    }
  }, [gameStarted]);

  return (
    <div className="h-full w-full flex flex-col">
      {/* Connection status bar */}
      {connected && (
        <div className="px-4 py-1.5 bg-green-500/10 border-b flex items-center gap-2 shrink-0">
          <span className="h-2 w-2 rounded-full bg-green-500" />
          <span className="text-xs text-green-700 dark:text-green-400">
            Connected as {username}
          </span>
        </div>
      )}
      {!connected && error && (
        <div className="px-4 py-2 bg-red-500/10 border-b flex items-center justify-between shrink-0">
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-red-500" />
            <span className="text-sm text-red-700 dark:text-red-400">
              Connection failed — {error}. Check your server settings.
            </span>
          </div>
          <div className="flex gap-2">
            <Button size="sm" variant="outline" onClick={() => connect(prefs.serverHost, prefs.serverPort, prefs.serverUsername, prefs.serverPassword)}>
              Retry
            </Button>
            <Button size="sm" variant="outline" onClick={() => navigate('/settings')}>
              Settings
            </Button>
          </div>
        </div>
      )}
      {!connected && !error && (
        <div className="px-4 py-2 bg-muted/30 border-b flex items-center justify-between shrink-0">
          <span className="text-sm text-muted-foreground">
            {connecting
              ? 'Connecting to server...'
              : prefs.serverUsername
                ? 'Not connected.'
                : 'Set your username in Settings to connect.'}
          </span>
          <Button size="sm" variant="outline" onClick={() => navigate('/settings')}>
            Settings
          </Button>
        </div>
      )}

      {/* Main content */}
      <div className="flex-1 min-h-0">
        <ResizablePanelGroup orientation="horizontal">
          <ResizablePanel defaultSize={75}>
            <ResizablePanelGroup orientation="vertical">
              <ResizablePanel defaultSize={70}>
                <TablesList
                  rooms={rooms}
                  currentRoom={currentRoom}
                  username={username}
                  onNewGame={() => setCreateRoomOpen(true)}
                  onJoinRoom={joinRoom}
                  onLeaveRoom={leaveRoom}
                  onSetReady={setReady}
                  onStartGame={startGame}
                />
              </ResizablePanel>
              <ResizableHandle />
              <ResizablePanel defaultSize={30}>
                <ChatComponent channelId="Lobby" />
              </ResizablePanel>
            </ResizablePanelGroup>
          </ResizablePanel>
          <ResizableHandle />
          <ResizablePanel defaultSize={25} minSize={20}>
            <UserList players={players} />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>

      <CreateRoomDialog open={createRoomOpen} onOpenChange={setCreateRoomOpen} />
    </div>
  );
}
