import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TablesList } from "@/components/lobby/TablesList";
import { UserList } from "@/components/lobby/UserList";
import { ChatComponent } from "@/components/lobby/ChatComponent";
import { CreateGameDialog } from "@/components/lobby/CreateGameDialog";
import { useState } from "react";
import type { Table, User } from "@/types/xmage";

export default function Lobby() {
  const [tables] = useState<Table[]>([
    { id: '1', name: 'Standard Match', gameType: 'Standard', deckType: 'Constructed', state: 'WAITING', numPlayers: 2, players: [], isTournament: false },
    { id: '2', name: 'Commander Night', gameType: 'Commander', deckType: 'EDH', state: 'DUELING', numPlayers: 4, players: [], isTournament: false },
  ]);
  const [users] = useState<User[]>([
    { username: 'Jace', serverAddress: 'xmage.de', flag: 'us' },
    { username: 'Chandra', serverAddress: 'xmage.de', flag: 'in' },
  ]);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);

  return (
    <div className="h-full w-full">
      <ResizablePanelGroup orientation="horizontal">
        <ResizablePanel defaultSize={75}>
          <ResizablePanelGroup orientation="vertical">
            <ResizablePanel defaultSize={70}>
              <TablesList tables={tables} onNewGame={() => setCreateDialogOpen(true)} />
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel defaultSize={30}>
              <ChatComponent channelId="Lobby" />
            </ResizablePanel>
          </ResizablePanelGroup>
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel defaultSize={25} minSize={20}>
          <UserList users={users} />
        </ResizablePanel>
      </ResizablePanelGroup>

      <CreateGameDialog open={createDialogOpen} onOpenChange={setCreateDialogOpen} />
    </div>
  );
}
