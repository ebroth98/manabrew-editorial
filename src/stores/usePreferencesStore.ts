import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface PreferencesState {
  /** Duration of card-play and turn-start flash animations in ms */
  flashDurationMs: number;
  setFlashDurationMs: (ms: number) => void;

  /** Server connection defaults */
  serverHost: string;
  serverPort: number;
  serverUsername: string;
  serverPassword: string;
  setServerHost: (host: string) => void;
  setServerPort: (port: number) => void;
  setServerUsername: (username: string) => void;
  setServerPassword: (password: string) => void;
}

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      flashDurationMs: 1000,
      setFlashDurationMs: (ms) => set({ flashDurationMs: ms }),

      serverHost: 'localhost',
      serverPort: 9443,
      serverUsername: '',
      serverPassword: 'forge',
      setServerHost: (serverHost) => set({ serverHost }),
      setServerPort: (serverPort) => set({ serverPort }),
      setServerUsername: (serverUsername) => set({ serverUsername }),
      setServerPassword: (serverPassword) => set({ serverPassword }),
    }),
    { name: 'xmage-preferences' },
  ),
);
