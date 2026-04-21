import React, { memo, useState, useRef, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/game/Card";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useHandScale } from "@/hooks/useHandScale";
import type { HandDisplayProps } from "../game.types";
import type { Card as XMageCard } from "@/types/openmagic";
import type { HandActionOption } from "@/stores/useGameUIStore";
import { HAND_CARD_BASES, ZONE_LABEL } from "../game.styles";
import { HandCardActions } from "./HandCardActions";

const TUG_LIMIT = 100;
const HOVER_SCALE = 1.8;

const CARD_SCALE_HOVERED: React.CSSProperties = {
  transform: `scale(${HOVER_SCALE})`,
  transformOrigin: "bottom center",
  transition: "transform 250ms cubic-bezier(0.23, 0.63, 0.32, 1)",
};
const CARD_SCALE_DEFAULT: React.CSSProperties = {
  transform: "scale(1)",
  transformOrigin: "bottom center",
  transition: "transform 250ms cubic-bezier(0.23, 0.63, 0.32, 1)",
};
// Both CSS vars are always defined by `Game.tsx` from the active theme,
// so this style just dereferences them — no literal colour fallback.
const PLAYABLE_GLOW_STYLE: React.CSSProperties = {
  backgroundColor: "var(--playable-glow-color)",
  border: "2px solid var(--playable-ring-color-strong)",
};
const EMPTY_ACTIONS: HandActionOption[] = [];

interface HandCardItemProps {
  card: XMageCard;
  cardW: number;
  cardH: number;
  isHovered: boolean;
  isTugging: boolean;
  tugOffset: { x: number; y: number };
  isCasting: boolean;
  isDragging: boolean;
  isRejected: boolean;
  actions: HandActionOption[];
  onFlipCard?: () => void;
  showBackFace?: boolean;
  onSelectAction?: (action: HandActionOption) => void;
  onMouseDown: (card: XMageCard, e: React.MouseEvent) => void;
  onMouseEnter: (card: XMageCard, e: React.MouseEvent) => void;
  onMouseLeave: () => void;
}

const HandCardItem = memo(function HandCardItem({
  card, cardW, cardH, isHovered, isTugging, tugOffset,
  isCasting, isDragging, isRejected, actions,
  onFlipCard, showBackFace, onSelectAction,
  onMouseDown, onMouseEnter, onMouseLeave,
}: HandCardItemProps) {
  const scale = isHovered ? HOVER_SCALE : 1;

  return (
    <div
      className={cn(
        "relative group shrink-0",
        !isTugging && "transition-[transform,z-index] duration-250 ease-[cubic-bezier(0.23,0.63,0.32,1)]",
        isHovered && !isTugging && "-translate-y-3 z-30",
        card.isPlayable && "cursor-grab",
        (isDragging || isCasting) && "opacity-0",
      )}
      style={{
        width: cardW,
        height: cardH,
        ...(isTugging ? { transform: `translate(${tugOffset.x}px, ${tugOffset.y}px)`, zIndex: 100 } : {}),
      }}
      onMouseDown={(e) => onMouseDown(card, e)}
      onMouseEnter={(e) => onMouseEnter(card, e)}
      onMouseLeave={onMouseLeave}
    >
      <div className="w-full h-full relative" style={isHovered ? CARD_SCALE_HOVERED : CARD_SCALE_DEFAULT}>
        <Card
          card={card}
          className={cn(
            "w-full h-full",
            card.isPlayable && cn("playable-card", isHovered && "is-hovered"),
            isRejected && "animate-reject-flash",
          )}
          isHovered={isHovered}
          onFlip={onFlipCard}
          showBackFace={showBackFace}
          resolution="large"
        />
        {card.isPlayable && (
          <div
            className="absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"
            style={PLAYABLE_GLOW_STYLE}
            title={`Play ${card.name}`}
          />
        )}

        {isHovered && actions.length > 0 && onSelectAction && (
          <div style={{
            position: "absolute",
            top: 0,
            left: "100%",
            transform: `scale(${1 / scale})`,
            transformOrigin: "top left",
          }}>
            <div
              style={{
                position: "absolute",
                top: 0,
                left: -cardW * scale,
                width: cardW * scale + 24 + 220,
                height: cardH * scale,
                backgroundColor: "transparent",
                borderBottomRightRadius: "100%",
                zIndex: -1,
              }}
            />
            <div style={{ paddingLeft: 24 }}>
              <HandCardActions
                actions={actions}
                onSelectAction={onSelectAction}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}, (prev, next) => {
  if (prev.isHovered !== next.isHovered || prev.isTugging !== next.isTugging ||
      prev.isCasting !== next.isCasting || prev.isDragging !== next.isDragging ||
      prev.isRejected !== next.isRejected ||
      prev.cardW !== next.cardW || prev.cardH !== next.cardH ||
      prev.onMouseDown !== next.onMouseDown || prev.onMouseEnter !== next.onMouseEnter ||
      prev.onMouseLeave !== next.onMouseLeave ||
      prev.onFlipCard !== next.onFlipCard || prev.showBackFace !== next.showBackFace ||
      prev.onSelectAction !== next.onSelectAction) return false;
  if (prev.isTugging && prev.tugOffset !== next.tugOffset) return false;
  if (prev.isHovered && prev.actions.length !== next.actions.length) return false;
  const pc = prev.card, nc = next.card;
  if (pc === nc) return true;
  return pc.id === nc.id && pc.isPlayable === nc.isPlayable && pc.name === nc.name;
});

export function HandDisplayNormal({
  cards,
  onHoverCard,
  onStartDrag,
  onClickCard,
  onFlipCard,
  showBackFace,
  draggingCardId,
  castingCardId,
  getActions,
  onSelectAction,
}: HandDisplayProps) {
  const handSize = usePreferencesStore((s) => s.handSize);
  const vScale = useHandScale();
  const base = HAND_CARD_BASES[handSize];
  const cardW = Math.round(base.cardW * vScale);
  const cardH = Math.round(base.cardH * vScale);
  const containerH = Math.round(base.containerH * vScale);

  const [rejectedId, setRejectedId] = useState<string | null>(null);
  const rejectedTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const rejectCard = useCallback((id: string) => {
    clearTimeout(rejectedTimer.current);
    setRejectedId(id);
    rejectedTimer.current = setTimeout(() => setRejectedId(null), 400);
  }, []);

  const [tugId, setTugId] = useState<string | null>(null);
  const [tugOffset, setTugOffset] = useState({ x: 0, y: 0 });

  const startTug = useCallback((cardId: string, startX: number, startY: number) => {
    setTugId(cardId);
    setTugOffset({ x: 0, y: 0 });

    const onMove = (me: MouseEvent) => {
      const dx = me.clientX - startX;
      const dy = me.clientY - startY;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist > TUG_LIMIT) {
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
        setTugId(null);
        setTugOffset({ x: 0, y: 0 });
        rejectCard(cardId);
      } else {
        setTugOffset({ x: dx, y: dy });
      }
    };

    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      setTugId(null);
      setTugOffset({ x: 0, y: 0 });
      rejectCard(cardId);
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }, [rejectCard]);

  const [hoveredCardId, setHoveredCardId] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Stable callback refs — avoids recreating handlers when parent callbacks change
  const onStartDragRef = useRef(onStartDrag);
  onStartDragRef.current = onStartDrag;
  const onClickCardRef = useRef(onClickCard);
  onClickCardRef.current = onClickCard;
  const onHoverCardRef = useRef(onHoverCard);
  onHoverCardRef.current = onHoverCard;

  const handleCardMouseDown = useCallback((card: XMageCard, e: React.MouseEvent) => {
    e.preventDefault();
    if (card.isPlayable && onStartDragRef.current) {
      onStartDragRef.current(card, e);
    } else if (card.isPlayable) {
      onClickCardRef.current?.(card, e);
    } else {
      startTug(card.id, e.clientX, e.clientY);
    }
  }, [startTug]);

  const handleCardMouseEnter = useCallback((card: XMageCard, e: React.MouseEvent) => {
    clearTimeout(hideTimerRef.current);
    setHoveredCardId(card.id);
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const finalTop = rect.top - 12 - (cardH * HOVER_SCALE - cardH);

    onHoverCardRef.current?.(card, e, {
      useAnchor: true,
      placement: "top-center",
      anchorOverride: {
        left: rect.left,
        right: rect.right,
        top: finalTop,
        bottom: finalTop + cardH * HOVER_SCALE,
        width: cardW * HOVER_SCALE,
        height: cardH * HOVER_SCALE,
        x: rect.left,
        y: finalTop,
        toJSON: () => ({})
      } as DOMRect,
    });
  }, [cardW, cardH]);

  const handleCardMouseLeave = useCallback(() => {
    hideTimerRef.current = setTimeout(() => {
      setHoveredCardId(null);
      onHoverCardRef.current?.(null);
    }, 150);
  }, []);

  return (
    <div className="flex flex-col gap-1 shrink-0" ref={containerRef}>
      <span className={ZONE_LABEL}>Hand ({cards.length})</span>
      <div className="overflow-x-auto">
        <div className="flex gap-2 pt-4 pb-2 px-1 items-end" style={{ minHeight: containerH - 8 }}>
          {cards.map((card) => (
            <HandCardItem
              key={card.id}
              card={card}
              cardW={cardW}
              cardH={cardH}
              isHovered={hoveredCardId === card.id}
              isTugging={tugId === card.id}
              tugOffset={tugOffset}
              isCasting={castingCardId != null && card.id === castingCardId}
              isDragging={card.id === draggingCardId}
              isRejected={rejectedId === card.id}
              actions={hoveredCardId === card.id && getActions ? getActions(card) : EMPTY_ACTIONS}
              onFlipCard={onFlipCard}
              showBackFace={showBackFace}
              onSelectAction={onSelectAction}
              onMouseDown={handleCardMouseDown}
              onMouseEnter={handleCardMouseEnter}
              onMouseLeave={handleCardMouseLeave}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
