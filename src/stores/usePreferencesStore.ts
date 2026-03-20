import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { STORAGE_KEYS } from '@/lib/constants';

export type ZonePanelSide = 'left' | 'right';
export type ZonePanelItem = 'library' | 'graveyard' | 'exile';
export type HandDisplayMode = 'cool' | 'normal';

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

  /** Hand display layout style in game */
  handDisplayMode: HandDisplayMode;
  setHandDisplayMode: (mode: HandDisplayMode) => void;

  /** Game UI color overrides by dot-path key */
  gameThemeColorOverrides: Record<string, string>;
  setGameThemeColorOverride: (path: string, color: string) => void;
  resetGameThemeColorOverrides: () => void;
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

      handDisplayMode: 'cool',
      setHandDisplayMode: (handDisplayMode) => set({ handDisplayMode }),

      gameThemeColorOverrides: {},
      setGameThemeColorOverride: (path, color) =>
        set((state) => ({
          gameThemeColorOverrides: {
            ...state.gameThemeColorOverrides,
            [path]: color,
          },
        })),
      resetGameThemeColorOverrides: () => set({ gameThemeColorOverrides: {} }),
    }),
    { name: STORAGE_KEYS.PREFERENCES },
  ),
);
