import { useState } from "react";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useServerStore } from "@/stores/useServerStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";

const FLASH_MIN = 200;
const FLASH_MAX = 2000;
const FLASH_STEP = 100;

export default function Settings() {
  const prefs = usePreferencesStore();
  const { flashDurationMs, setFlashDurationMs } = prefs;
  const server = useServerStore();

  const [host, setHost] = useState(prefs.serverHost);
  const [port, setPort] = useState(String(prefs.serverPort));
  const [username, setUsername] = useState(prefs.serverUsername);
  const [password, setPassword] = useState(prefs.serverPassword);

  const hasChanges =
    host !== prefs.serverHost ||
    port !== String(prefs.serverPort) ||
    username !== prefs.serverUsername ||
    password !== prefs.serverPassword;

  async function handleSave() {
    prefs.setServerHost(host);
    prefs.setServerPort(Number(port));
    prefs.setServerUsername(username);
    prefs.setServerPassword(password);

    // Always disconnect first (kills any existing WS connection)
    await server.disconnect();

    if (username) {
      await server.connect(host, Number(port), username, password);
    }
  }

  return (
    <div className="max-w-xl mx-auto py-8 px-4 space-y-8">
      <h1 className="text-2xl font-bold">Preferences</h1>

      <section className="space-y-4">
        <h2 className="text-lg font-semibold">Server</h2>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-1">
            <Label htmlFor="server-host">Host</Label>
            <Input id="server-host" value={host} onChange={(e) => setHost(e.target.value)} placeholder="localhost" />
          </div>
          <div className="space-y-1">
            <Label htmlFor="server-port">Port</Label>
            <Input id="server-port" type="number" value={port} onChange={(e) => setPort(e.target.value)} placeholder="9443" />
          </div>
          <div className="space-y-1">
            <Label htmlFor="server-username">Username</Label>
            <Input id="server-username" value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Player1" />
          </div>
          <div className="space-y-1">
            <Label htmlFor="server-password">Password</Label>
            <Input id="server-password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="forge" />
          </div>
        </div>
        <div className="flex items-center gap-3">
          <Button onClick={handleSave} disabled={!hasChanges && !server.error}>
            Save & Reconnect
          </Button>
          {server.connected && (
            <span className="text-xs text-green-600 dark:text-green-400 flex items-center gap-1">
              <span className="h-2 w-2 rounded-full bg-green-500" />
              Connected as {server.username}
            </span>
          )}
          {server.connecting && (
            <span className="text-xs text-muted-foreground">Connecting...</span>
          )}
          {server.error && (
            <span className="text-xs text-red-600 dark:text-red-400">{server.error}</span>
          )}
        </div>
        <p className="text-xs text-muted-foreground">
          Server connection settings. Saving will disconnect and reconnect with the new credentials.
        </p>
      </section>

      <section className="space-y-4">
        <h2 className="text-lg font-semibold">Game Animations</h2>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label htmlFor="flash-duration">Flash duration</Label>
            <span className="text-sm font-mono text-muted-foreground">
              {flashDurationMs}ms
            </span>
          </div>
          <input
            id="flash-duration"
            type="range"
            min={FLASH_MIN}
            max={FLASH_MAX}
            step={FLASH_STEP}
            value={flashDurationMs}
            onChange={(e) => setFlashDurationMs(Number(e.target.value))}
            className="w-full accent-primary"
          />
          <p className="text-xs text-muted-foreground">
            How long card-play and turn-start flashes stay on screen.
          </p>
        </div>
      </section>
    </div>
  );
}
