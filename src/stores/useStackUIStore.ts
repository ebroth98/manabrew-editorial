import { create } from "zustand";

interface StackUIState {
  hoveredStackObjectId: string | null;
  setHoveredStackObjectId: (id: string | null) => void;
  reset: () => void;
}

export const useStackUIStore = create<StackUIState>((set) => ({
  hoveredStackObjectId: null,
  setHoveredStackObjectId: (id) => set({ hoveredStackObjectId: id }),
  reset: () => set({ hoveredStackObjectId: null }),
}));

