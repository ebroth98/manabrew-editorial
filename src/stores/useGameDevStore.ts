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

interface GameDevState {
  promptActionOverride: DevPromptActionOverride | null;
  devToolsEnabled: boolean;
  setPromptActionOverride: (value: DevPromptActionOverride | null) => void;
  setDevToolsEnabled: (value: boolean) => void;
  clearPromptActionOverride: () => void;
  resetDevSettings: () => void;
}

export const useGameDevStore = create<GameDevState>()(devtools((set) => ({
  promptActionOverride: null,
  devToolsEnabled: false,
  setPromptActionOverride: (value) => set({ promptActionOverride: value }),
  setDevToolsEnabled: (value) => set({ devToolsEnabled: value }),
  clearPromptActionOverride: () => set({ promptActionOverride: null }),
  resetDevSettings: () =>
    set({
      promptActionOverride: null,
      devToolsEnabled: false,
    }),
}), { name: "gameDev", enabled: import.meta.env.DEV }));
