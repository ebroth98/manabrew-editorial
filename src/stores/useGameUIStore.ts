import { create } from "zustand";
import type { Card, ActivatableAbilityInfo } from "@/types/xmage";

// ── State Shapes ──────────────────────────────────────────────────────────────

interface AbilityPickerState {
  cardId: string;
  cardName: string;
  abilities: ActivatableAbilityInfo[];
}

interface PlayModePickerState {
  cardId: string;
  cardName: string;
  options: { cardId: string; mode: string; modeLabel: string }[];
}

interface ViewingZoneState {
  title: string;
  cards: Card[];
  onClickCard?: (cardId: string) => void;
}

// ── Store Interface ───────────────────────────────────────────────────────────

interface GameUIState {
  // Modal states (all transient, not persisted)
  abilityPicker: AbilityPickerState | null;
  playModePicker: PlayModePickerState | null;
  viewingZone: ViewingZoneState | null;
  isActionPanelCollapsed: boolean;

  // Actions
  openAbilityPicker: (state: AbilityPickerState) => void;
  closeAbilityPicker: () => void;
  openPlayModePicker: (state: PlayModePickerState) => void;
  closePlayModePicker: () => void;
  openZoneViewer: (state: ViewingZoneState) => void;
  closeZoneViewer: () => void;
  toggleActionPanel: () => void;
  setActionPanelCollapsed: (collapsed: boolean) => void;
  resetAll: () => void;
}

// ── Store Implementation ──────────────────────────────────────────────────────

export const useGameUIStore = create<GameUIState>((set) => ({
  // Initial state
  abilityPicker: null,
  playModePicker: null,
  viewingZone: null,
  isActionPanelCollapsed: false,

  // Actions
  openAbilityPicker: (state) => set({ abilityPicker: state }),
  closeAbilityPicker: () => set({ abilityPicker: null }),

  openPlayModePicker: (state) => set({ playModePicker: state }),
  closePlayModePicker: () => set({ playModePicker: null }),

  openZoneViewer: (state) => set({ viewingZone: state }),
  closeZoneViewer: () => set({ viewingZone: null }),

  toggleActionPanel: () =>
    set((state) => ({ isActionPanelCollapsed: !state.isActionPanelCollapsed })),
  setActionPanelCollapsed: (collapsed) =>
    set({ isActionPanelCollapsed: collapsed }),

  resetAll: () =>
    set({
      abilityPicker: null,
      playModePicker: null,
      viewingZone: null,
      isActionPanelCollapsed: false,
    }),
}));
