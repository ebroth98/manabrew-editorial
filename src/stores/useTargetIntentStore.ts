import { create } from "zustand";

export type TargetIntentKind = "card" | "player" | "spell";

export interface TargetIntent {
  kind: TargetIntentKind;
  id: string;
}

interface TargetIntentStoreState {
  intents: Record<string, TargetIntent>;
  setIntent: (sourceCardId: string, intent: TargetIntent) => void;
  clearIntent: (sourceCardId: string) => void;
  clearAll: () => void;
}

export const useTargetIntentStore = create<TargetIntentStoreState>((set) => ({
  intents: {},
  setIntent: (sourceCardId, intent) =>
    set((s) => ({ intents: { ...s.intents, [sourceCardId]: intent } })),
  clearIntent: (sourceCardId) =>
    set((s) => {
      const next = { ...s.intents };
      delete next[sourceCardId];
      return { intents: next };
    }),
  clearAll: () => set({ intents: {} }),
}));
