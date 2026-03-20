import { useRef, useState } from "react";
import type { Card } from "@/types/openmagic";

interface UseHandDragOptions {
  battlefieldContainerRef: React.RefObject<HTMLDivElement | null>;
  handContainerRef: React.RefObject<HTMLDivElement | null>;
  onCastSpell: (cardId: string) => void;
  dismissHover: () => void;
}

export function useHandDrag({
  battlefieldContainerRef,
  handContainerRef,
  onCastSpell,
  dismissHover,
}: UseHandDragOptions) {
  const [draggingHandCard, setDraggingHandCard] = useState<Card | null>(null);
  const [ghostPos, setGhostPos] = useState({ x: 0, y: 0 });
  const [isOverBattlefield, setIsOverBattlefield] = useState(false);
  const isOverBattlefieldRef = useRef(false);

  function startHandCardDrag(card: Card, e: React.MouseEvent) {
    if (!card.isPlayable) return;
    dismissHover();
    setDraggingHandCard(card);
    setGhostPos({ x: e.clientX, y: e.clientY });

    let moved = false;
    const handleMouseMove = (me: MouseEvent) => {
      moved = true;
      // Hard-disable hover preview during drag; hover timers can be re-armed by
      // underlying mouseenter events while the cursor crosses cards.
      dismissHover();
      setGhostPos({ x: me.clientX, y: me.clientY });

      if (battlefieldContainerRef.current) {
        const rect = battlefieldContainerRef.current.getBoundingClientRect();
        let over =
          me.clientX >= rect.left &&
          me.clientX <= rect.right &&
          me.clientY >= rect.top &&
          me.clientY <= rect.bottom;

        // Hand is inside battlefield container — exclude it so dropping
        // back on the hand cancels instead of casting
        if (over && handContainerRef.current) {
          const handRect = handContainerRef.current.getBoundingClientRect();
          const overHand =
            me.clientX >= handRect.left &&
            me.clientX <= handRect.right &&
            me.clientY >= handRect.top &&
            me.clientY <= handRect.bottom;
          if (overHand) over = false;
        }

        isOverBattlefieldRef.current = over;
        setIsOverBattlefield(over);
      }
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);

      if (!moved) {
        onCastSpell(card.id);
      } else if (isOverBattlefieldRef.current) {
        onCastSpell(card.id);
      }

      setDraggingHandCard(null);
      setIsOverBattlefield(false);
      isOverBattlefieldRef.current = false;
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }

  return { draggingHandCard, ghostPos, isOverBattlefield, startHandCardDrag };
}
