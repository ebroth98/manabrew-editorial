import { useState } from 'react';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useServerStore } from '@/stores/useServerStore';
import type { GameFormat } from '@/types/server';
import { cn } from '@/lib/utils';

interface CreateRoomDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CreateRoomDialog({ open, onOpenChange }: CreateRoomDialogProps) {
  const { createRoom, username } = useServerStore();
  const [roomName, setRoomName] = useState('');
  const [maxPlayers, setMaxPlayers] = useState(4);
  const [format, setFormat] = useState<GameFormat>('Standard');

  const defaultName = `${username ?? 'Player'}'s Room`;

  async function handleCreate() {
    await createRoom(roomName.trim() || defaultName, maxPlayers, format);
    onOpenChange(false);
    setRoomName('');
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogTitle>Create Room</DialogTitle>
        <div className="space-y-4">
          <div className="space-y-1">
            <Label htmlFor="room-name">Room Name</Label>
            <Input
              id="room-name"
              value={roomName}
              onChange={(e) => setRoomName(e.target.value)}
              placeholder={defaultName}
            />
          </div>
          <div className="space-y-1">
            <Label>Max Players</Label>
            <div className="flex gap-2">
              {[2, 3, 4].map((n) => (
                <Button
                  key={n}
                  size="sm"
                  variant={maxPlayers === n ? 'default' : 'outline'}
                  onClick={() => setMaxPlayers(n)}
                  className="flex-1"
                >
                  {n}
                </Button>
              ))}
            </div>
          </div>
          <div className="space-y-1">
            <Label htmlFor="room-format">Format</Label>
            <select
              id="room-format"
              className={cn(
                "w-full h-9 rounded-md border border-input bg-background px-2 text-sm",
                "focus:outline-none focus:ring-2 focus:ring-ring"
              )}
              value={format}
              onChange={(e) => setFormat(e.target.value as GameFormat)}
            >
              <option value="Standard">Standard</option>
              <option value="Commander">Commander</option>
            </select>
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={handleCreate}>
              Create
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
