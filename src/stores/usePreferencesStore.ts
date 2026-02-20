import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface PreferencesState {
  /** Duration of card-play and turn-start flash animations in ms */
  flashDurationMs: number;
  setFlashDurationMs: (ms: number) => void;
}

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      flashDurationMs: 1000,
      setFlashDurationMs: (ms) => set({ flashDurationMs: ms }),
    }),
    { name: 'xmage-preferences' },
  ),
);
