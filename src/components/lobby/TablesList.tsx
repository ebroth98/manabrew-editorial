import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Users } from "lucide-react";
import type { RoomInfo } from "@/types/server";

interface TablesListProps {
  rooms: RoomInfo[];
  currentRoom: RoomInfo | null;
  username: string | null;
  onNewGame: () => void;
  onJoinRoom: (roomId: string) => void;
  onLeaveRoom: () => void;
  onSetReady: (ready: boolean) => void;
  onStartGame: () => void;
}

export function TablesList({
  rooms,
  currentRoom,
  username,
  onNewGame,
  onJoinRoom,
  onLeaveRoom,
  onSetReady,
  onStartGame,
}: TablesListProps) {
  const inRoom = currentRoom != null;
  const myPlayer = currentRoom?.players.find((p) => p.username === username);
  const isHost = currentRoom?.host === username;
  const allReady = currentRoom ? currentRoom.players.length >= 2 && currentRoom.players.every((p) => p.ready) : false;

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between p-4 border-b">
        <h2 className="text-lg font-semibold">Rooms</h2>
        <Button size="sm" onClick={onNewGame} disabled={inRoom}>
          Create Room
        </Button>
      </div>

      {/* Current room banner */}
      {currentRoom && (
        <div className="px-4 py-3 border-b bg-primary/5 space-y-2">
          <div className="flex items-center justify-between">
            <div>
              <span className="font-semibold text-sm">{currentRoom.room_name}</span>
              <span className="text-xs text-muted-foreground ml-2">{currentRoom.format}</span>
            </div>
            <Badge variant={currentRoom.status === 'Lobby' ? 'outline' : 'secondary'}>
              {currentRoom.status}
            </Badge>
          </div>
          <div className="flex items-center gap-2 text-xs">
            {currentRoom.players.map((p) => (
              <span
                key={p.username}
                className={p.ready ? 'text-green-600 dark:text-green-400 font-medium' : 'text-muted-foreground'}
              >
                {p.username}{p.ready ? ' [Ready]' : ''}
              </span>
            ))}
          </div>
          <div className="flex gap-2">
            {myPlayer && !myPlayer.ready && (
              <Button size="sm" onClick={() => onSetReady(true)}>Ready</Button>
            )}
            {myPlayer && myPlayer.ready && (
              <Button size="sm" variant="outline" onClick={() => onSetReady(false)}>Unready</Button>
            )}
            {isHost && (
              <Button size="sm" onClick={onStartGame} disabled={!allReady}>
                Start Game
              </Button>
            )}
            <Button size="sm" variant="outline" onClick={onLeaveRoom}>Leave</Button>
          </div>
        </div>
      )}

      <div className="flex-1 overflow-auto">
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
                        <Button size="sm" variant="secondary" onClick={() => onJoinRoom(room.room_id)}>
                          Join
                        </Button>
                      ) : room.status === 'InGame' ? (
                        <Button size="sm" variant="ghost" disabled>Watch</Button>
                      ) : null}
                    </TableCell>
                  </TableRow>
                );
              })
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
