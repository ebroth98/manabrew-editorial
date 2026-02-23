import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { PlayerInfo } from "@/types/server";
import { cn } from "@/lib/utils";

interface UserListProps {
  players: PlayerInfo[];
}

export function UserList({ players }: UserListProps) {
  return (
    <div className="flex flex-col h-full border-l">
      <div className="p-4 border-b">
        <h3 className="font-semibold text-sm">Online Players ({players.length})</h3>
      </div>
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-3">
          {players.length === 0 ? (
            <p className="text-xs text-muted-foreground italic">No players online.</p>
          ) : (
            players.map((player) => (
              <div key={player.player_id} className="flex items-center space-x-3">
                <div className="relative">
                  <Avatar className="h-8 w-8">
                    <AvatarFallback>{player.username.slice(0, 2).toUpperCase()}</AvatarFallback>
                  </Avatar>
                  <span
                    className={cn(
                      "absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-2 border-background",
                      player.connected ? "bg-green-500" : "bg-gray-400"
                    )}
                  />
                </div>
                <div className="flex flex-col gap-0.5 min-w-0">
                  <span className="text-sm font-medium leading-none truncate">{player.username}</span>
                  <span className="text-xs text-muted-foreground">
                    {player.room_id ? 'In room' : 'In lobby'}
                  </span>
                </div>
              </div>
            ))
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
