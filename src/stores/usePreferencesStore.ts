import { create } from 'zustand';
import { persist, devtools } from 'zustand/middleware';
import { STORAGE_KEYS } from '@/lib/constants';

export type ZonePanelSide = 'left' | 'right';
export type ZonePanelItem = 'library' | 'graveyard' | 'exile';
export type HandDisplayMode = 'cool' | 'normal';
export type HandSize = 'small' | 'medium' | 'large';
export type CardPreviewMode = 'hover' | 'shift' | 'alt' | 'ctrl';

interface PreferencesState {
  /** App color theme preset id */
  appThemePreset: string;
  setAppThemePreset: (id: string) => void;

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

  /** Hand card size */
  handSize: HandSize;
  setHandSize: (size: HandSize) => void;

  /** Card preview trigger mode */
  cardPreviewMode: CardPreviewMode;
  setCardPreviewMode: (mode: CardPreviewMode) => void;

  /** Card hover preview delay in ms */
  cardHoverDelayMs: number;
  setCardHoverDelayMs: (ms: number) => void;

  /** App theme color overrides (CSS variable name → HSL value) */
  appThemeColorOverrides: Record<string, string>;
  setAppThemeColorOverride: (key: string, hsl: string) => void;
  resetAppThemeColorOverrides: () => void;

  /** Use PixiJS canvas renderer for the game board (experimental) */
  pixiEnabled: boolean;
  setPixiEnabled: (enabled: boolean) => void;

  /** Game UI color overrides by dot-path key */
  gameThemeColorOverrides: Record<string, string>;
  setGameThemeColorOverride: (path: string, color: string) => void;
  resetGameThemeColorOverrides: () => void;
}

export const usePreferencesStore = create<PreferencesState>()(
  devtools(persist(
    (set) => ({
      appThemePreset: "default",
      setAppThemePreset: (appThemePreset) => set({ appThemePreset, appThemeColorOverrides: {}, gameThemeColorOverrides: {} }),

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

      handSize: 'medium',
      setHandSize: (handSize) => set({ handSize }),

      cardPreviewMode: 'hover',
      setCardPreviewMode: (cardPreviewMode) => set({ cardPreviewMode }),

      cardHoverDelayMs: 500,
      setCardHoverDelayMs: (ms) => set({ cardHoverDelayMs: ms }),

      appThemeColorOverrides: {},
      setAppThemeColorOverride: (key, hsl) =>
        set((state) => ({
          appThemeColorOverrides: { ...state.appThemeColorOverrides, [key]: hsl },
        })),
      resetAppThemeColorOverrides: () => set({ appThemeColorOverrides: {} }),

      pixiEnabled: false,
      setPixiEnabled: (pixiEnabled) => set({ pixiEnabled }),

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
  ), { name: "preferences", enabled: import.meta.env.DEV }),
);
