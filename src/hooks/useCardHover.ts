import { useRef, useState, useEffect } from "react";
import type { Card } from "@/types/openmagic";

/**
 * Manages the delayed card hover preview with a 500ms debounce.
 * Automatically dismisses when any dependency in `dismissDeps` changes.
 */
export function useCardHover(dismissDeps: unknown[]) {
  const [hoveredCard, setHoveredCard] = useState<Card | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });
  const [showBackFace, setShowBackFace] = useState(false);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function dismissHover() {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    setHoveredCard(null);
  }

  function handleFlipCard() {
    setShowBackFace((prev) => !prev);
  }

  function handleHoverCard(card: Card | null, e?: React.MouseEvent) {
    // Never schedule hover preview while the mouse button is held (e.g. drag).
    if (e && e.buttons !== 0) {
      dismissHover();
      return;
    }
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    if (!card) {
      setHoveredCard(null);
      setShowBackFace(false);
      return;
    }
    if (e) setMousePos({ x: e.clientX, y: e.clientY });
    // If a card is already showing, switch instantly; only debounce the initial show
    if (hoveredCard) {
      setHoveredCard(card);
      setShowBackFace(false);
    } else {
      hoverTimerRef.current = setTimeout(() => {
        setHoveredCard(card);
        setShowBackFace(false);
        hoverTimerRef.current = null;
      }, 500);
    }
  }

  // Dismiss preview when modals open or prompt changes
  useEffect(() => {
    dismissHover();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, dismissDeps);

  return {
    hoveredCard,
    mousePos,
    showBackFace,
    dismissHover,
    handleFlipCard,
    handleHoverCard,
  };
}
