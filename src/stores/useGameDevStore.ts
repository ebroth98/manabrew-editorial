import { create } from "zustand";
import { devtools } from "zustand/middleware";

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
] as const;

export type PromptActionViewKey =
  (typeof PROMPT_ACTION_VIEW_KEYS)[number];

export const DEV_PROMPT_ACTION_OVERRIDES = [
  "chooseAction",
  "chooseAttackers",
  "chooseBlockers",
  "chooseTargetSpell",
  "payManaCost",
  "noAction",
] as const;

export type DevPromptActionOverride =
  (typeof DEV_PROMPT_ACTION_OVERRIDES)[number];

interface PixiPerfStats {
  fps: number;
  minFps: number;
  maxFps: number;
  deltaMs: number;
}

interface GameDevState {
  promptActionOverride: DevPromptActionOverride | null;
  devToolsEnabled: boolean;
  pixiPerfStats: PixiPerfStats | null;
  setPromptActionOverride: (value: DevPromptActionOverride | null) => void;
  setDevToolsEnabled: (value: boolean) => void;
  clearPromptActionOverride: () => void;
  setPixiPerfStats: (stats: PixiPerfStats | null) => void;
  resetDevSettings: () => void;
}

export const useGameDevStore = create<GameDevState>()(devtools((set) => ({
  promptActionOverride: null,
  devToolsEnabled: false,
  pixiPerfStats: null,
  setPromptActionOverride: (value) => set({ promptActionOverride: value }),
  setDevToolsEnabled: (value) => set({ devToolsEnabled: value }),
  clearPromptActionOverride: () => set({ promptActionOverride: null }),
  setPixiPerfStats: (stats) => set({ pixiPerfStats: stats }),
  resetDevSettings: () =>
    set({
      promptActionOverride: null,
      devToolsEnabled: false,
    }),
}), { name: "gameDev", enabled: import.meta.env.DEV }));
