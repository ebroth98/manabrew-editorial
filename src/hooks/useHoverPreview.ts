import { useState, useCallback } from "react";
import type { Card } from "@/types/xmage";

/**
 * Manages hovered-card + mouse-position state for card preview overlays.
 * Returns handlers to spread onto card container elements.
 */
export function useHoverPreview() {
  const [hoveredCard, setHoveredCard] = useState<Card | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  const onMouseEnter = useCallback((card: Card, e: React.MouseEvent) => {
    setHoveredCard(card);
    setMousePos({ x: e.clientX, y: e.clientY });
  }, []);

  const onMouseLeave = useCallback(() => {
    setHoveredCard(null);
  }, []);

  return { hoveredCard, mousePos, onMouseEnter, onMouseLeave } as const;
}
