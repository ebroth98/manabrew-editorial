import { useState } from 'react';
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useServerStore } from '@/stores/useServerStore';
import type { GameFormat } from '@/types/server';
import { cn } from '@/lib/utils';
import { Swords, Crown, Users, Loader2 } from 'lucide-react';

const FORMATS: { value: GameFormat; label: string; icon: typeof Swords; description: string }[] = [
  { value: 'Standard', label: 'Standard', icon: Swords, description: '60-card constructed, 1v1 or multiplayer' },
  { value: 'Commander', label: 'Commander', icon: Crown, description: '100-card singleton, multiplayer focused' },
];

const PLAYER_OPTIONS = [2, 3, 4] as const;

interface CreateRoomDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CreateRoomDialog({ open, onOpenChange }: CreateRoomDialogProps) {
  const { createRoom, username } = useServerStore();
  const [roomName, setRoomName] = useState('');
  const [maxPlayers, setMaxPlayers] = useState(4);
  const [format, setFormat] = useState<GameFormat>('Standard');
  const [creating, setCreating] = useState(false);

  const defaultName = `${username ?? 'Player'}'s Room`;

  async function handleCreate() {
    setCreating(true);
    try {
      await createRoom(roomName.trim() || defaultName, maxPlayers, format);
      onOpenChange(false);
      setRoomName('');
    } finally {
      setCreating(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md p-0 gap-0 overflow-hidden">
        <div className="px-6 pt-6 pb-4">
          <DialogTitle className="text-lg">Create Room</DialogTitle>
          <DialogDescription className="text-sm text-muted-foreground">
            Set up a new game room for others to join.
          </DialogDescription>
        </div>

        <div className="px-6 pb-6 space-y-5">
          {/* Room name */}
          <div className="space-y-1.5">
            <Label htmlFor="room-name" className="text-xs font-medium">Room Name</Label>
            <Input
              id="room-name"
              value={roomName}
              onChange={(e) => setRoomName(e.target.value)}
              placeholder={defaultName}
              className="h-9"
              onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
            />
          </div>

          {/* Format */}
          <div className="space-y-1.5">
            <Label className="text-xs font-medium">Format</Label>
            <div className="grid grid-cols-2 gap-2">
              {FORMATS.map((f) => {
                const Icon = f.icon;
                return (
                  <button
                    key={f.value}
                    type="button"
                    onClick={() => setFormat(f.value)}
                    className={cn(
                      "flex flex-col items-start gap-1 rounded-lg border p-3 text-left transition-colors",
                      format === f.value
                        ? "border-primary bg-primary/5"
                        : "border-border hover:border-primary/30 hover:bg-muted/30",
                    )}
                  >
                    <div className="flex items-center gap-2">
                      <Icon className={cn("h-4 w-4", format === f.value ? "text-primary" : "text-muted-foreground")} />
                      <span className="text-sm font-medium">{f.label}</span>
                    </div>
                    <span className="text-[11px] text-muted-foreground leading-tight">{f.description}</span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Max players */}
          <div className="space-y-1.5">
            <Label className="text-xs font-medium">Players</Label>
            <div className="flex items-center gap-2">
              {PLAYER_OPTIONS.map((n) => (
                <button
                  key={n}
                  type="button"
                  onClick={() => setMaxPlayers(n)}
                  className={cn(
                    "flex-1 h-10 rounded-lg border flex items-center justify-center gap-1.5 transition-colors",
                    maxPlayers === n
                      ? "border-primary bg-primary/5 text-primary"
                      : "border-border hover:border-primary/30 text-muted-foreground hover:text-foreground",
                  )}
                >
                  <Users className="h-3.5 w-3.5" />
                  <span className="text-sm font-medium">{n}</span>
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t bg-muted/20 flex items-center justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button size="sm" onClick={handleCreate} disabled={creating} className="gap-1.5 min-w-[100px]">
            {creating ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Swords className="h-3.5 w-3.5" />
            )}
            {creating ? 'Creating...' : 'Create Room'}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
