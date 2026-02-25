import { useState } from "react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Loader2, RefreshCw, Users } from "lucide-react";
import type { RoomInfo } from "@/types/server";
import { cn } from "@/lib/utils";

interface TablesListProps {
  rooms: RoomInfo[];
  currentRoom: RoomInfo | null;
  username: string | null;
  onNewGame: () => void;
  onRefresh: () => void;
  refreshing: boolean;
  refreshDisabled: boolean;
  onJoinRoom: (roomId: string) => Promise<void>;
  onLeaveRoom: () => void;
  onSetReady: (ready: boolean) => void;
  onOpenDeckDialog: () => void;
  onStartGame: () => void;
}

export function TablesList({
  rooms,
  currentRoom,
  username,
  onNewGame,
  onRefresh,
  refreshing,
  refreshDisabled,
  onJoinRoom,
  onLeaveRoom,
  onSetReady,
  onOpenDeckDialog,
  onStartGame,
}: TablesListProps) {
  const getPlayerDeckName = (player: RoomInfo["players"][number]) =>
    player.selected_deck_name ?? player.selectedDeckName;

  const inRoom = currentRoom != null;
  const myPlayer = currentRoom?.players.find((p) => p.username === username);
  const myPlayerHasDeck = !!(myPlayer && getPlayerDeckName(myPlayer));
  const isHost = currentRoom?.host === username;
  const allReady = currentRoom ? currentRoom.players.length >= 2 && currentRoom.players.every((p) => p.ready) : false;
  const [joiningRoomId, setJoiningRoomId] = useState<string | null>(null);
  const orderedCurrentRoomPlayers = currentRoom
    ? [...currentRoom.players].sort((a, b) => {
      if (a.username === currentRoom.host) return -1;
      if (b.username === currentRoom.host) return 1;
      return a.username.localeCompare(b.username);
    })
    : [];

  async function handleJoinRoom(roomId: string) {
    if (joiningRoomId) return;
    setJoiningRoomId(roomId);
    try {
      await onJoinRoom(roomId);
    } finally {
      setJoiningRoomId(null);
    }
  }

  return (
    <div className="h-full flex flex-col">
      <div className="flex flex-wrap items-center justify-between gap-2 p-4 border-b">
        <div>
          <h2 className="text-lg font-semibold">Rooms</h2>
          <p className="text-xs text-muted-foreground">Join an open lobby or host a new match.</p>
        </div>
        <div className="flex items-center gap-2 ml-auto">
          <Button
            size="sm"
            variant="outline"
            onClick={onRefresh}
            disabled={refreshDisabled || refreshing}
          >
            {refreshing ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
            Refresh
          </Button>
          <Button size="sm" onClick={onNewGame} disabled={inRoom}>
            Create Room
          </Button>
        </div>
      </div>

      {/* Current room banner */}
      {currentRoom && (
        <div className="px-4 py-3 border-b bg-primary/5 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div>
              <span className="font-semibold text-sm">{currentRoom.room_name}</span>
              <span className="text-xs text-muted-foreground ml-2">{currentRoom.format}</span>
            </div>
            <Badge variant={currentRoom.status === 'Lobby' ? 'outline' : 'secondary'}>
              {currentRoom.status}
            </Badge>
          </div>
          <div className="flex flex-col gap-2">
            {orderedCurrentRoomPlayers.map((p) => (
              <div
                key={p.username}
                className="rounded-md border bg-background/70 px-2.5 py-2 text-xs w-full max-w-lg"
              >
                <div className="flex items-center gap-2">
                  <span
                    className={cn(
                      "h-2 w-2 rounded-full",
                      p.ready ? "bg-emerald-500" : "bg-amber-500",
                    )}
                  />
                  <span className="font-medium">{p.username}</span>
                  {p.username === currentRoom.host && (
                    <Badge variant="outline" className="h-5 px-1.5 text-[10px]">Host</Badge>
                  )}
                  <Badge
                    variant={p.ready ? "default" : "outline"}
                    className={cn(
                      "h-5 px-1.5 text-[10px] ml-auto",
                      p.ready ? "bg-emerald-600 border-emerald-600 text-white hover:bg-emerald-600" : "text-muted-foreground",
                    )}
                  >
                    {p.ready ? "Ready" : "Not Ready"}
                  </Badge>
                </div>
                <div className="mt-1 text-[11px] text-muted-foreground truncate">
                  Deck: {getPlayerDeckName(p) ?? "No deck selected"}
                </div>
              </div>
            ))}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {myPlayer && !myPlayer.ready && (
              <Button size="sm" onClick={() => onSetReady(true)} disabled={!myPlayerHasDeck}>
                Ready
              </Button>
            )}
            {myPlayer && myPlayer.ready && (
              <Button size="sm" variant="outline" onClick={() => onSetReady(false)}>Unready</Button>
            )}
            <Button size="sm" variant="outline" onClick={onOpenDeckDialog}>
              Deck
            </Button>
            <Button size="sm" variant="outline" onClick={onLeaveRoom}>Leave</Button>
            {isHost && (
              <Button size="sm" className="ml-auto" onClick={onStartGame} disabled={!allReady}>
                Start Game
              </Button>
            )}
          </div>
        </div>
      )}

      <div className="flex-1 overflow-auto">
        <div className="hidden md:block">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[100px]">Format</TableHead>
                <TableHead>Room</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Players</TableHead>
                <TableHead className="text-right">Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rooms.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                    No rooms found. Create one to get started.
                  </TableCell>
                </TableRow>
              ) : (
                rooms.map((room) => {
                  const isMyRoom = room.room_id === currentRoom?.room_id;
                  const canJoin = !inRoom && room.status === 'Lobby' && room.players.length < room.max_players;
                  return (
                    <TableRow key={room.room_id} className={isMyRoom ? 'bg-primary/5' : undefined}>
                      <TableCell className="font-medium">{room.format}</TableCell>
                      <TableCell>
                        <div className="flex flex-col">
                          <span>{room.room_name}</span>
                          <span className="text-xs text-muted-foreground">Host: {room.host}</span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant={room.status === 'Lobby' ? 'outline' : 'secondary'}>
                          {room.status}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          <Users className="w-3 h-3 text-muted-foreground" />
                          <span>{room.players.length}/{room.max_players}</span>
                        </div>
                      </TableCell>
                      <TableCell className="text-right">
                        {isMyRoom ? (
                          <Badge variant="secondary" className="text-xs">Joined</Badge>
                        ) : canJoin ? (
                          <Button
                            size="sm"
                            variant="secondary"
                            className="transition-transform duration-75 active:scale-95"
                            disabled={joiningRoomId === room.room_id}
                            onClick={() => { void handleJoinRoom(room.room_id); }}
                          >
                            {joiningRoomId === room.room_id ? "Joining..." : "Join"}
                          </Button>
                        ) : room.status === 'InGame' ? (
                          <Button size="sm" variant="ghost" disabled>In Game</Button>
                        ) : (
                          <Button size="sm" variant="ghost" disabled>Full</Button>
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>

        <div className="md:hidden p-3 space-y-3">
          {rooms.length === 0 ? (
            <div className="rounded-lg border p-6 text-center text-sm text-muted-foreground">
              No rooms found. Create one to get started.
            </div>
          ) : (
            rooms.map((room) => {
              const isMyRoom = room.room_id === currentRoom?.room_id;
              const canJoin = !inRoom && room.status === 'Lobby' && room.players.length < room.max_players;
              return (
                <div key={room.room_id} className={cn("rounded-lg border p-3 space-y-3", isMyRoom && "bg-primary/5 border-primary/30")}>
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="font-medium truncate">{room.room_name}</div>
                      <div className="text-xs text-muted-foreground">Host: {room.host}</div>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      <Badge variant="outline">{room.format}</Badge>
                      <Badge variant={room.status === 'Lobby' ? 'outline' : 'secondary'}>
                        {room.status}
                      </Badge>
                    </div>
                  </div>
                  <div className="flex items-center gap-1 text-sm text-muted-foreground">
                    <Users className="w-3 h-3" />
                    <span>{room.players.length}/{room.max_players} players</span>
                  </div>
                  {isMyRoom ? (
                    <Badge variant="secondary" className="text-xs">Joined</Badge>
                  ) : canJoin ? (
                    <Button
                      size="sm"
                      className="w-full transition-transform duration-75 active:scale-[0.98]"
                      disabled={joiningRoomId === room.room_id}
                      onClick={() => { void handleJoinRoom(room.room_id); }}
                    >
                      {joiningRoomId === room.room_id ? "Joining..." : "Join Room"}
                    </Button>
                  ) : room.status === 'InGame' ? (
                    <Button size="sm" className="w-full" variant="ghost" disabled>
                      In Game
                    </Button>
                  ) : (
                    <Button size="sm" className="w-full" variant="ghost" disabled>
                      Room Full
                    </Button>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
