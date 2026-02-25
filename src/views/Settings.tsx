import { useState } from "react";
import { usePreferencesStore, type ZonePanelItem } from "@/stores/usePreferencesStore";
import { useServerStore } from "@/stores/useServerStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { useTheme } from "next-themes";

const FLASH_MIN = 200;
const FLASH_MAX = 2000;
const FLASH_STEP = 100;

export default function Settings() {
  const prefs = usePreferencesStore();
  const { flashDurationMs, setFlashDurationMs } = prefs;
  const server = useServerStore();
  const { theme, setTheme } = useTheme();

  const zoneOrder = prefs.zonePanelOrder;

  function setZoneSlot(index: number, value: ZonePanelItem) {
    const next = [...zoneOrder] as ZonePanelItem[];
    const existingIndex = next.indexOf(value);
    if (existingIndex !== -1 && existingIndex !== index) {
      const prevValue = next[index]!;
      next[index] = value;
      next[existingIndex] = prevValue;
    } else {
      next[index] = value;
    }
    prefs.setZonePanelOrder(next);
  }

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
        <h2 className="text-lg font-semibold">Gameplay</h2>

        <div className="flex items-start gap-3">
          <input
            id="auto-pass"
            type="checkbox"
            checked={prefs.autoPassEnabled}
            onChange={(e) => prefs.setAutoPassEnabled(e.target.checked)}
            className="mt-1 accent-primary h-4 w-4"
          />
          <div className="space-y-1">
            <Label htmlFor="auto-pass">Auto-pass when no actions</Label>
            <p className="text-xs text-muted-foreground">
              Automatically pass priority when you have no playable cards.
              Uses a random delay to prevent information leaking in multiplayer.
            </p>
          </div>
        </div>

        <div className="space-y-2 pt-2">
          <Label>Battlefield Zone Column Side</Label>
          <div className="flex items-center gap-2">
            <Button
              variant={prefs.zonePanelSide === "left" ? "default" : "outline"}
              size="sm"
              onClick={() => prefs.setZonePanelSide("left")}
            >
              Left
            </Button>
            <Button
              variant={prefs.zonePanelSide === "right" ? "default" : "outline"}
              size="sm"
              onClick={() => prefs.setZonePanelSide("right")}
            >
              Right
            </Button>
          </div>
        </div>

        <div className="space-y-2">
          <Label>Battlefield Zone Column Order</Label>
          <div className="grid grid-cols-3 gap-2">
            {(["Top", "Middle", "Bottom"] as const).map((slot, index) => (
              <div key={slot} className="space-y-1">
                <Label htmlFor={`zone-order-${index}`} className="text-xs text-muted-foreground">
                  {slot}
                </Label>
                <select
                  id={`zone-order-${index}`}
                  value={zoneOrder[index]}
                  onChange={(e) => setZoneSlot(index, e.target.value as ZonePanelItem)}
                  className="w-full h-9 rounded-md border border-input bg-background px-2 text-sm"
                >
                  <option value="library">Library</option>
                  <option value="graveyard">Graveyard</option>
                  <option value="exile">Exile</option>
                </select>
              </div>
            ))}
          </div>
          <p className="text-xs text-muted-foreground">
            Controls placement of Library / Graveyard / Exile in the in-field zone column.
          </p>
        </div>
      </section>

      <section className="space-y-4">
        <h2 className="text-lg font-semibold">Appearance</h2>
        <div className="space-y-2">
          <Label>Theme</Label>
          <div className="flex items-center gap-2">
            <Button
              variant={theme === "light" ? "default" : "outline"}
              size="sm"
              onClick={() => setTheme("light")}
            >
              Light
            </Button>
            <Button
              variant={theme === "dark" ? "default" : "outline"}
              size="sm"
              onClick={() => setTheme("dark")}
            >
              Dark
            </Button>
            <Button
              variant={theme === "system" ? "default" : "outline"}
              size="sm"
              onClick={() => setTheme("system")}
            >
              System
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Controls app theme preference.
          </p>
        </div>
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
