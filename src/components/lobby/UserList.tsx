import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { PlayerInfo } from "@/types/server";
import { cn } from "@/lib/utils";

interface UserListProps {
  players: PlayerInfo[];
}

export function UserList({ players }: UserListProps) {
  return (
    <div className="flex flex-col h-full">
      <div className="px-4 py-3 border-b flex items-center justify-between">
        <h3 className="font-semibold text-sm">Players</h3>
        <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
          {players.length}
        </span>
      </div>
      <ScrollArea className="flex-1">
        <div className="p-3 space-y-1">
          {players.length === 0 ? (
            <p className="text-xs text-muted-foreground italic text-center py-6">No players online</p>
          ) : (
            players.map((player) => (
              <div
                key={player.player_id}
                className="flex items-center gap-2.5 px-2 py-1.5 rounded-md hover:bg-muted/40 transition-colors"
              >
                <div className="relative shrink-0">
                  <Avatar className="h-7 w-7">
                    <AvatarFallback className="text-[10px]">
                      {player.username.slice(0, 2).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <span
                    className={cn(
                      "absolute -bottom-0.5 -right-0.5 w-2 h-2 rounded-full border-2 border-background",
                      player.connected ? "bg-primary" : "bg-muted-foreground/40"
                    )}
                  />
                </div>
                <div className="flex-1 min-w-0">
                  <span className="text-xs font-medium leading-none truncate block">{player.username}</span>
                  <span className="text-[10px] text-muted-foreground">
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
