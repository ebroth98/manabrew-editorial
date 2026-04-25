import { create } from "zustand";
import { devtools } from "zustand/middleware";

interface StackUIState {
  hoveredStackObjectId: string | null;
  setHoveredStackObjectId: (id: string | null) => void;
  reset: () => void;
}

export const useStackUIStore = create<StackUIState>()(
  devtools(
    (set) => ({
      hoveredStackObjectId: null,
      setHoveredStackObjectId: (id) => set({ hoveredStackObjectId: id }),
      reset: () => set({ hoveredStackObjectId: null }),
    }),
    { name: "stackUI", enabled: import.meta.env.DEV },
  ),
);
