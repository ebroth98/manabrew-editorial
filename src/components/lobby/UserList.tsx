import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { User } from "@/types/xmage";

interface UserListProps {
  users: User[];
}

export function UserList({ users }: UserListProps) {
  return (
    <div className="flex flex-col h-full border-l">
      <div className="p-4 border-b">
        <h3 className="font-semibold text-sm">Online Users ({users.length})</h3>
      </div>
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-4">
          {users.map((user) => (
            <div key={user.username} className="flex items-center space-x-4">
              <Avatar className="h-8 w-8">
                <AvatarImage src={`https://flagsapi.com/${user.flag?.toUpperCase() || 'US'}/flat/64.png`} />
                <AvatarFallback>{user.username.slice(0, 2).toUpperCase()}</AvatarFallback>
              </Avatar>
              <div className="flex flex-col gap-1">
                <span className="text-sm font-medium leading-none">{user.username}</span>
                <span className="text-xs text-muted-foreground">{user.serverAddress}</span>
              </div>
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
