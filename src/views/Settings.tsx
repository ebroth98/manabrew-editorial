import { useState } from "react";
import { usePreferencesStore, type ZonePanelItem } from "@/stores/usePreferencesStore";
import { THEME_PRESETS, type ThemeColors } from "@/themes";
import { useServerStore } from "@/stores/useServerStore";
import { useGameStore } from "@/stores/useGameStore";
import { getDefaultGameThemeColorMap, toPickerHexColor } from "@/components/game/game.theme";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { useTheme } from "next-themes";
import { Navigate } from "react-router-dom";

/** Human-readable labels for theme color keys */
const APP_THEME_COLOR_LABELS: Record<string, string> = {
  background: "Background",
  foreground: "Text",
  card: "Card Surface",
  "card-foreground": "Card Text",
  popover: "Popover Surface",
  "popover-foreground": "Popover Text",
  primary: "Primary",
  "primary-foreground": "Primary Text",
  secondary: "Secondary",
  "secondary-foreground": "Secondary Text",
  muted: "Muted Surface",
  "muted-foreground": "Muted Text",
  accent: "Accent",
  "accent-foreground": "Accent Text",
  destructive: "Destructive",
  "destructive-foreground": "Destructive Text",
  border: "Border",
  input: "Input",
  ring: "Focus Ring",
  selection: "Selection",
  "selection-foreground": "Selection Text",
  commander: "Commander",
  warning: "Warning",
  overlay: "Overlay",
};

function hslToHex(hsl: string): string {
  const parts = hsl.trim().split(/\s+/).map((s) => parseFloat(s));
  if (parts.length < 3 || parts.some(isNaN)) return "#808080";
  const [h, s, l] = parts;
  const sn = s / 100;
  const ln = l / 100;
  const a = sn * Math.min(ln, 1 - ln);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const color = ln - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
    return Math.round(255 * color).toString(16).padStart(2, "0");
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

function hexToHsl(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return `0 0% ${Math.round(l * 100)}%`;
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return `${Math.round(h * 360)} ${Math.round(s * 100)}% ${Math.round(l * 100)}%`;
}

const FLASH_MIN = 200;
const FLASH_MAX = 2000;
const FLASH_STEP = 100;
export default function Settings() {
  const isGameActive = useGameStore((s) => s.isGameActive);
  const prefs = usePreferencesStore();
  const { flashDurationMs, setFlashDurationMs } = prefs;
  const server = useServerStore();
  const { theme, setTheme, resolvedTheme } = useTheme();
  const [activeTab, setActiveTab] = useState<"server" | "preferences" | "theme">("preferences");
  const [presetOpen, setPresetOpen] = useState(false);
  const [editingThemeColorPath, setEditingThemeColorPath] = useState<string | null>(null);
  const [editingThemeColorValue, setEditingThemeColorValue] = useState("");
  const DEFAULT_GAME_THEME_COLOR_MAP = getDefaultGameThemeColorMap();

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

  function beginThemeColorEdit(path: string, value: string) {
    setEditingThemeColorPath(path);
    setEditingThemeColorValue(value);
  }

  function commitThemeColorEdit(path: string, fallbackValue: string) {
    const next = editingThemeColorValue.trim() || fallbackValue;
    prefs.setGameThemeColorOverride(path, next);
    setEditingThemeColorPath(null);
    setEditingThemeColorValue("");
  }

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

  if (isGameActive) {
    return <Navigate to="/play" replace />;
  }

  return (
    <div className="max-w-xl mx-auto py-8 px-4 space-y-8">
      <h1 className="text-2xl font-bold">Preferences</h1>

      <section className="space-y-4">
        <div className="flex items-center gap-6 border-b">
          <button
            type="button"
            onClick={() => setActiveTab("preferences")}
            className={
              "pb-2 text-sm font-medium transition-colors border-b-2 " +
              (activeTab === "preferences"
                ? "border-primary text-foreground"
                : "border-transparent text-muted-foreground hover:text-foreground")
            }
          >
            Preferences
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("theme")}
            className={
              "pb-2 text-sm font-medium transition-colors border-b-2 " +
              (activeTab === "theme"
                ? "border-primary text-foreground"
                : "border-transparent text-muted-foreground hover:text-foreground")
            }
          >
            Theme
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("server")}
            className={
              "pb-2 text-sm font-medium transition-colors border-b-2 " +
              (activeTab === "server"
                ? "border-primary text-foreground"
                : "border-transparent text-muted-foreground hover:text-foreground")
            }
          >
            Server
          </button>
        </div>
      </section>

      {activeTab === "server" && (
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
      )}

      {activeTab === "preferences" && (
        <section className="space-y-4">
          <h2 className="text-lg font-semibold">Preferences</h2>

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

          <div className="space-y-2">
            <Label>Hand Display Mode</Label>
            <div className="flex items-center gap-2">
              <Button
                variant={prefs.handDisplayMode === "cool" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setHandDisplayMode("cool")}
              >
                Cool
              </Button>
              <Button
                variant={prefs.handDisplayMode === "normal" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setHandDisplayMode("normal")}
              >
                Normal
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Switch between curved fan hand layout (Cool) and flat hand layout (Normal).
            </p>
          </div>

          <div className="space-y-2">
            <Label>Hand Card Size</Label>
            <div className="flex items-center gap-2">
              <Button
                variant={prefs.handSize === "small" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setHandSize("small")}
              >
                Small
              </Button>
              <Button
                variant={prefs.handSize === "medium" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setHandSize("medium")}
              >
                Medium
              </Button>
              <Button
                variant={prefs.handSize === "large" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setHandSize("large")}
              >
                Large
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Controls the size of cards displayed in your hand.
            </p>
          </div>

          <div className="space-y-2">
            <Label>Card Preview Trigger</Label>
            <div className="flex items-center gap-2">
              <Button
                variant={prefs.cardPreviewMode === "hover" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setCardPreviewMode("hover")}
              >
                Hover
              </Button>
              <Button
                variant={prefs.cardPreviewMode === "shift" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setCardPreviewMode("shift")}
              >
                Shift + Hover
              </Button>
              <Button
                variant={prefs.cardPreviewMode === "alt" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setCardPreviewMode("alt")}
              >
                Alt + Hover
              </Button>
              <Button
                variant={prefs.cardPreviewMode === "ctrl" ? "default" : "outline"}
                size="sm"
                onClick={() => prefs.setCardPreviewMode("ctrl")}
              >
                Ctrl + Hover
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Controls when the card preview and ability panel appears. "Hover" shows on mouse over, others require holding a modifier key.
            </p>
          </div>

          <div className="space-y-4">
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
          </div>
        </section>
      )}

      {activeTab === "theme" && (
        <section className="space-y-4">
          <h2 className="text-lg font-semibold">Theme</h2>

          <div className="space-y-2">
            <Label>App Theme</Label>
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

          <div className="space-y-2 pt-2">
            <Label>Color Preset</Label>
            {(() => {
              const active = THEME_PRESETS.find((p) => p.id === prefs.appThemePreset);
              const mode = resolvedTheme === "dark" ? "dark" : "light";
              return (
                <div className="relative">
                  <button
                    type="button"
                    onClick={() => setPresetOpen((v) => !v)}
                    className="w-full flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors hover:bg-muted/30"
                  >
                    {active && (
                      <div className="flex gap-1 shrink-0">
                        {[active[mode].background, active[mode].primary, active[mode].accent, active[mode].destructive].map((hsl, i) => (
                          <div key={i} className="w-4 h-4 rounded-full border border-border/50" style={{ backgroundColor: `hsl(${hsl})` }} />
                        ))}
                      </div>
                    )}
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium">{active?.name ?? "Select preset"}</div>
                    </div>
                    <svg className="h-4 w-4 text-muted-foreground shrink-0" viewBox="0 0 16 16" fill="none"><path d="M4 6l4 4 4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
                  </button>
                  {presetOpen && (
                    <div className="absolute z-50 top-full left-0 right-0 mt-1 bg-popover border rounded-lg shadow-lg max-h-64 overflow-y-auto">
                      {THEME_PRESETS.map((preset) => (
                        <button
                          key={preset.id}
                          type="button"
                          onClick={() => { prefs.setAppThemePreset(preset.id); setPresetOpen(false); }}
                          className={
                            "w-full flex items-center gap-3 px-3 py-2 text-left transition-colors hover:bg-muted/40 " +
                            (prefs.appThemePreset === preset.id ? "bg-primary/5" : "")
                          }
                        >
                          <div className="flex gap-1 shrink-0">
                            {[preset[mode].background, preset[mode].primary, preset[mode].accent, preset[mode].destructive].map((hsl, i) => (
                              <div key={i} className="w-3.5 h-3.5 rounded-full border border-border/50" style={{ backgroundColor: `hsl(${hsl})` }} />
                            ))}
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="text-sm font-medium">{preset.name}</div>
                            <div className="text-[11px] text-muted-foreground">{preset.description}</div>
                          </div>
                          {prefs.appThemePreset === preset.id && (
                            <div className="text-[10px] text-primary font-medium shrink-0">Active</div>
                          )}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              );
            })()}
            <p className="text-xs text-muted-foreground">
              Choose a color preset. Works with both light and dark modes.
            </p>
          </div>

          <div className="space-y-3 pt-2">
            <div className="flex items-center justify-between gap-2">
              <Label>App Theme Colors</Label>
              <Button
                size="sm"
                variant="outline"
                onClick={prefs.resetAppThemeColorOverrides}
                disabled={Object.keys(prefs.appThemeColorOverrides).length === 0}
              >
                Reset Colors
              </Button>
            </div>
            <div className="space-y-2">
              {Object.keys(APP_THEME_COLOR_LABELS).map((key) => {
                const activePreset = THEME_PRESETS.find((p) => p.id === prefs.appThemePreset);
                const mode = resolvedTheme === "dark" ? "dark" : "light";
                const presetValue = activePreset?.[mode]?.[key as keyof ThemeColors] ?? "";
                const activeValue = prefs.appThemeColorOverrides[key] ?? presetValue;
                const hexValue = hslToHex(activeValue);

                return (
                  <div key={key} className="flex items-center gap-3 rounded-md border px-2 py-1.5">
                    <Label className="flex-1 text-xs font-mono">
                      {APP_THEME_COLOR_LABELS[key]}
                    </Label>
                    <input
                      type="color"
                      value={hexValue}
                      onChange={(e) => prefs.setAppThemeColorOverride(key, hexToHsl(e.target.value))}
                      className="h-8 w-10 rounded border border-input bg-transparent p-0.5"
                    />
                    <button
                      type="button"
                      className="w-24 text-right text-[11px] font-mono text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
                      onClick={() => {
                        beginThemeColorEdit(`app.${key}`, activeValue);
                      }}
                      title="Click to edit color value"
                    >
                      {hexValue}
                    </button>
                  </div>
                );
              })}
            </div>
            <p className="text-xs text-muted-foreground">
              Override individual colors from the active preset.
            </p>
          </div>

          <div className="space-y-3 pt-2">
            <div className="flex items-center justify-between gap-2">
              <Label>Game Theme Colors</Label>
              <Button
                size="sm"
                variant="outline"
                onClick={prefs.resetGameThemeColorOverrides}
              >
                Reset Colors
              </Button>
            </div>
            <div className="space-y-2">
              {Object.entries(DEFAULT_GAME_THEME_COLOR_MAP).map(([path, defaultColor]) => {
                const activeColor = prefs.gameThemeColorOverrides[path] ?? defaultColor;
                return (
                  <div key={path} className="flex items-center gap-3 rounded-md border px-2 py-1.5">
                    <Label htmlFor={`theme-color-${path}`} className="flex-1 text-xs font-mono">
                      {path}
                    </Label>
                    <input
                      id={`theme-color-${path}`}
                      type="color"
                      value={toPickerHexColor(activeColor)}
                      onChange={(e) => prefs.setGameThemeColorOverride(path, e.target.value)}
                      className="h-8 w-10 rounded border border-input bg-transparent p-0.5"
                    />
                    {editingThemeColorPath === path ? (
                      <input
                        autoFocus
                        value={editingThemeColorValue}
                        onChange={(e) => setEditingThemeColorValue(e.target.value)}
                        onBlur={() => commitThemeColorEdit(path, defaultColor)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            commitThemeColorEdit(path, defaultColor);
                          }
                          if (e.key === "Escape") {
                            setEditingThemeColorPath(null);
                            setEditingThemeColorValue("");
                          }
                        }}
                        className="w-24 h-7 rounded border border-input bg-background px-1.5 text-right text-[11px] font-mono"
                      />
                    ) : (
                      <button
                        type="button"
                        className="w-24 text-right text-[11px] font-mono text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
                        onClick={() => beginThemeColorEdit(path, activeColor)}
                        title="Click to edit color value"
                      >
                        {activeColor}
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
            <p className="text-xs text-muted-foreground">
              Generated from game theme keys. Defaults come from current game theme values.
            </p>
          </div>
        </section>
      )}
    </div>
  );
}
