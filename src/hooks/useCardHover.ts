import { useRef, useState, useEffect, useCallback } from "react";
import type { Card } from "@/types/openmagic";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import type { CardPreviewMode } from "@/stores/usePreferencesStore";

/** Check whether the required modifier key is held for the given preview mode. */
function isModifierHeld(e: React.MouseEvent, mode: CardPreviewMode): boolean {
  switch (mode) {
    case "hover": return true;
    case "shift": return e.shiftKey;
    case "alt":   return e.altKey;
    case "ctrl":  return e.ctrlKey || e.metaKey;
  }
}

/**
 * Manages the delayed card hover preview with a 500ms debounce.
 * Automatically dismisses when any dependency in `dismissDeps` changes.
 *
 * When the preview is sticky (has actions), mouse-leave is ignored entirely.
 * The preview is only dismissed via dismissHover() (Escape, outside click, action).
 */
export function useCardHover(dismissDeps: unknown[]) {
  const [hoveredCard, setHoveredCard] = useState<Card | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });
  const [showBackFace, setShowBackFace] = useState(false);
  const [sticky, setSticky] = useState(false);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cardPreviewMode = usePreferencesStore((s) => s.cardPreviewMode);

  const dismissHover = useCallback(() => {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    setHoveredCard(null);
    setSticky(false);
  }, []);

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
      // When sticky, ignore mouse-leave — only dismissHover() can close it
      if (sticky) return;
      setHoveredCard(null);
      setShowBackFace(false);
      return;
    }
    // Check modifier key requirement
    if (e && !isModifierHeld(e, cardPreviewMode)) {
      return;
    }
    // Don't switch cards while sticky — dismiss first
    if (sticky) return;
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

  /** Make the current hover preview sticky (won't dismiss on mouse-leave). */
  function makeSticky() {
    if (hoveredCard) setSticky(true);
  }

  /** Programmatically show the preview for a card and lock it (for click-to-open).
   *  If no position is provided, uses the last known mouse position. */
  function showStickyPreview(card: Card, x?: number, y?: number) {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    if (x != null && y != null) setMousePos({ x, y });
    setHoveredCard(card);
    setShowBackFace(false);
    setSticky(true);
  }

  // Dismiss preview when modals open or prompt changes
  useEffect(() => {
    dismissHover();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, dismissDeps);

  // Dismiss when modifier key is released (for non-hover modes)
  useEffect(() => {
    if (cardPreviewMode === "hover") return;
    function handleKeyUp(e: KeyboardEvent) {
      const keyMap: Record<string, CardPreviewMode[]> = {
        Shift: ["shift"],
        Alt: ["alt"],
        Control: ["ctrl"],
        Meta: ["ctrl"],
      };
      if (keyMap[e.key]?.includes(cardPreviewMode) && !sticky) {
        dismissHover();
      }
    }
    window.addEventListener("keyup", handleKeyUp);
    return () => window.removeEventListener("keyup", handleKeyUp);
  }, [cardPreviewMode, sticky, dismissHover]);

  return {
    hoveredCard,
    mousePos,
    showBackFace,
    isSticky: sticky,
    dismissHover,
    handleFlipCard,
    handleHoverCard,
    makeSticky,
    showStickyPreview,
  };
}
