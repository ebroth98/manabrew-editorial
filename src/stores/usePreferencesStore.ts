import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type ZonePanelSide = 'left' | 'right';
export type ZonePanelItem = 'library' | 'graveyard' | 'exile';

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

  /** Auto-pass priority when no legal actions are available */
  autoPassEnabled: boolean;
  setAutoPassEnabled: (enabled: boolean) => void;

  /** Battlefield zone column placement + order */
  zonePanelSide: ZonePanelSide;
  zonePanelOrder: ZonePanelItem[];
  setZonePanelSide: (side: ZonePanelSide) => void;
  setZonePanelOrder: (order: ZonePanelItem[]) => void;
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

      autoPassEnabled: true,
      setAutoPassEnabled: (autoPassEnabled) => set({ autoPassEnabled }),

      zonePanelSide: 'left',
      zonePanelOrder: ['library', 'graveyard', 'exile'],
      setZonePanelSide: (zonePanelSide) => set({ zonePanelSide }),
      setZonePanelOrder: (zonePanelOrder) => set({ zonePanelOrder }),
    }),
    { name: 'xmage-preferences' },
  ),
);
