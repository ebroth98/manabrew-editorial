import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { TargetingIntent } from "@/types/promptType";
import type { ArrowType } from "@/pixi/types";

export const PROMPT_ACTION_VIEW_KEYS = [
  "chooseAction",
  "chooseAttackers",
  "chooseBlockers",
  "chooseTargetSpell",
  "payManaCost",
  "promptRequired",
  "promptLabel",
  "passingUntilEot",
  "autoPassing",
  "noAction",
  "mulligan",
  "mulliganPutBack",
] as const;

export type PromptActionViewKey = (typeof PROMPT_ACTION_VIEW_KEYS)[number];

export const DEV_PROMPT_ACTION_OVERRIDES = [
  "chooseAction",
  "chooseAttackers",
  "chooseBlockers",
  "chooseTargetSpell",
  "payManaCost",
  "noAction",
] as const;

export type DevPromptActionOverride = (typeof DEV_PROMPT_ACTION_OVERRIDES)[number];

interface PixiPerfStats {
  fps: number;
  minFps: number;
  maxFps: number;
  deltaMs: number;
}

/** Dev-only overrides applied to the local player's panel so every
 *  badge/visual state can be inspected without running a real game. All
 *  fields default to neutral (`false` / `null`) = pass the real game
 *  value through. When a numeric field is non-null it overrides the
 *  engine value unconditionally. */
export interface DevPlayerOverrides {
  forceMonarch: boolean;
  forceInitiative: boolean;
  forceCityBlessing: boolean;
  poison: number | null;
  energy: number | null;
  radiation: number | null;
  ringLevel: number | null;
  speed: number | null;
  cmdDamage: number | null;
  life: number | null;
  handCount: number | null;
}

export const DEFAULT_DEV_PLAYER_OVERRIDES: DevPlayerOverrides = {
  forceMonarch: false,
  forceInitiative: false,
  forceCityBlessing: false,
  poison: null,
  energy: null,
  radiation: null,
  ringLevel: null,
  speed: null,
  cmdDamage: null,
  life: null,
  handCount: null,
};

interface GameDevState {
  promptActionOverride: DevPromptActionOverride | null;
  devToolsEnabled: boolean;
  pixiPerfStats: PixiPerfStats | null;
  playerOverrides: DevPlayerOverrides;
  /** Pointer intent the operator has force-enabled to inspect its glyph
   *  / glow on the live board. At most one at a time — the panel acts
   *  as a radio so swapping intents lets you compare them side-by-side
   *  without overlap. Renders a debug pointer from the local player's
   *  avatar to the first opponent's. */
  debugPointerIntent: TargetingIntent | null;
  /** Arrow type the operator has force-enabled to inspect on the live
   *  board (combat / placement). Same radio behavior as
   *  `debugPointerIntent`. */
  debugArrowType: ArrowType | null;
  setPromptActionOverride: (value: DevPromptActionOverride | null) => void;
  setDevToolsEnabled: (value: boolean) => void;
  clearPromptActionOverride: () => void;
  setPixiPerfStats: (stats: PixiPerfStats | null) => void;
  setPlayerOverride: <K extends keyof DevPlayerOverrides>(
    key: K,
    value: DevPlayerOverrides[K],
  ) => void;
  resetPlayerOverrides: () => void;
  setDebugPointerIntent: (intent: TargetingIntent | null) => void;
  setDebugArrowType: (type: ArrowType | null) => void;
  resetDevSettings: () => void;
}

export const useGameDevStore = create<GameDevState>()(
  devtools(
    (set) => ({
      promptActionOverride: null,
      devToolsEnabled: false,
      pixiPerfStats: null,
      playerOverrides: DEFAULT_DEV_PLAYER_OVERRIDES,
      debugPointerIntent: null,
      debugArrowType: null,
      setPromptActionOverride: (value) => set({ promptActionOverride: value }),
      setDevToolsEnabled: (value) => set({ devToolsEnabled: value }),
      clearPromptActionOverride: () => set({ promptActionOverride: null }),
      setPixiPerfStats: (stats) => set({ pixiPerfStats: stats }),
      setPlayerOverride: (key, value) =>
        set((state) => ({
          playerOverrides: { ...state.playerOverrides, [key]: value },
        })),
      resetPlayerOverrides: () => set({ playerOverrides: DEFAULT_DEV_PLAYER_OVERRIDES }),
      setDebugPointerIntent: (intent) => set({ debugPointerIntent: intent }),
      setDebugArrowType: (type) => set({ debugArrowType: type }),
      resetDevSettings: () =>
        set({
          promptActionOverride: null,
          devToolsEnabled: false,
          playerOverrides: DEFAULT_DEV_PLAYER_OVERRIDES,
          debugPointerIntent: null,
          debugArrowType: null,
        }),
    }),
    { name: "gameDev", enabled: import.meta.env.DEV },
  ),
);
