import { useRef, useState, useLayoutEffect, useCallback } from "react";
import { CARD_W, CARD_H, CARD_GAP as GAP } from "@/components/game/game.constants";

interface Marquee {
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  additive: boolean;
}

interface UseBattlefieldLayoutOptions {
  cardIds: string[];
  bottomReserved: number;
  leftReserved: number;
  rightReserved: number;
}

export function useBattlefieldLayout({
  cardIds,
  bottomReserved,
  leftReserved,
  rightReserved,
}: UseBattlefieldLayoutOptions) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [positions, setPositions] = useState<Record<string, { x: number; y: number }>>({});
  const [selectedCardIds, setSelectedCardIds] = useState<Set<string>>(new Set());
  const [draggingCardIds, setDraggingCardIds] = useState<Set<string>>(new Set());
  const [selectMode, setSelectMode] = useState(false);
  const [marquee, setMarquee] = useState<Marquee | null>(null);

  // Refs so event handlers always read the latest state without stale closures
  const positionsRef = useRef(positions);
  positionsRef.current = positions;
  const selectedCardIdsRef = useRef(selectedCardIds);
  selectedCardIdsRef.current = selectedCardIds;
  const selectModeRef = useRef(selectMode);
  selectModeRef.current = selectMode;
  const marqueeRef = useRef<Marquee | null>(null);

  // Mutable drag state — updated without triggering re-renders
  const dragRef = useRef<{
    cardIds: string[];
    startMouseX: number;
    startMouseY: number;
    startPositions: Record<string, { x: number; y: number }>;
    moved: boolean;
  } | null>(null);

  // Auto-position new cards in a left-to-right grid; remove departed cards
  useLayoutEffect(() => {
    if (!containerRef.current) return;
    const containerW = containerRef.current.clientWidth;
    const containerH = containerRef.current.clientHeight;
    const xMin = Math.max(0, leftReserved);
    const xMaxByRight = Math.max(xMin, containerW - CARD_W - Math.max(0, rightReserved));
    const usableW = Math.max(CARD_W, xMaxByRight - xMin + CARD_W);
    const cols = Math.max(1, Math.floor((usableW + GAP) / (CARD_W + GAP)));
    const xMax = xMaxByRight;
    const yMax = containerH > 0 ? Math.max(0, containerH - CARD_H - bottomReserved) : Infinity;

    const cardIdSet = new Set(cardIds);

    setPositions((prev) => {
      const next = { ...prev };
      for (const id of Object.keys(next)) {
        if (!cardIdSet.has(id)) delete next[id];
      }
      const alreadyPositioned = Object.keys(next).length;
      let newIdx = 0;
      for (const id of cardIds) {
        if (!next[id]) {
          const slot = alreadyPositioned + newIdx;
          next[id] = {
            x: Math.min(xMax, xMin + (slot % cols) * (CARD_W + GAP) + GAP),
            y: Math.min(Math.floor(slot / cols) * (CARD_H + GAP) + GAP, yMax),
          };
          newIdx++;
        }
      }
      return next;
    });

    setSelectedCardIds((prev) => {
      const next = new Set([...prev].filter((id) => cardIdSet.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [cardIds, bottomReserved, leftReserved, rightReserved]);

  // Card mousedown: shift+click toggles selection; otherwise start drag
  const handleCardMouseDown = useCallback((e: React.MouseEvent, cardId: string) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.shiftKey) {
      setSelectedCardIds((prev) => {
        const next = new Set(prev);
        if (next.has(cardId)) next.delete(cardId);
        else next.add(cardId);
        return next;
      });
      return;
    }

    const pos = positionsRef.current[cardId];
    if (!pos) return;

    const inSelection = selectedCardIdsRef.current.has(cardId);
    const cardsToDrag = inSelection ? [...selectedCardIdsRef.current] : [cardId];

    if (!inSelection) setSelectedCardIds(new Set());

    const startPositions: Record<string, { x: number; y: number }> = {};
    for (const id of cardsToDrag) {
      startPositions[id] = positionsRef.current[id] ?? { x: 0, y: 0 };
    }

    dragRef.current = {
      cardIds: cardsToDrag,
      startMouseX: e.clientX,
      startMouseY: e.clientY,
      startPositions,
      moved: false,
    };
    setDraggingCardIds(new Set(cardsToDrag));

    const handleMouseMove = (me: MouseEvent) => {
      if (!dragRef.current) return;
      const dx = me.clientX - dragRef.current.startMouseX;
      const dy = me.clientY - dragRef.current.startMouseY;
      if (!dragRef.current.moved && Math.sqrt(dx * dx + dy * dy) < 5) return;
      dragRef.current.moved = true;

      const el = containerRef.current;
      if (!el) return;
      const xMin = Math.max(0, leftReserved);
      const xMax = Math.max(xMin, el.clientWidth - CARD_W - Math.max(0, rightReserved));

      setPositions((prev) => {
        const next = { ...prev };
        for (const id of dragRef.current!.cardIds) {
          const start = dragRef.current!.startPositions[id];
          if (!start) continue;
          next[id] = {
            x: Math.max(xMin, Math.min(xMax, start.x + dx)),
            y: Math.max(0, Math.min(el.clientHeight - CARD_H, start.y + dy)),
          };
        }
        return next;
      });
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      setDraggingCardIds(new Set());
      dragRef.current = null;
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [leftReserved, rightReserved]);

  // Container background mousedown: marquee in select mode, clear selection otherwise
  const handleContainerMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;

    const el = containerRef.current;
    if (!el) return;

    if (selectModeRef.current) {
      e.preventDefault();
      const rect = el.getBoundingClientRect();
      const startX = e.clientX - rect.left;
      const startY = e.clientY - rect.top;
      const additive = e.shiftKey;

      const initial: Marquee = { startX, startY, currentX: startX, currentY: startY, additive };
      marqueeRef.current = initial;
      setMarquee(initial);

      const handleMouseMove = (me: MouseEvent) => {
        const currentX = Math.max(0, Math.min(el.clientWidth, me.clientX - rect.left));
        const currentY = Math.max(0, Math.min(el.clientHeight, me.clientY - rect.top));
        const updated = { ...marqueeRef.current!, currentX, currentY };
        marqueeRef.current = updated;
        setMarquee(updated);
      };

      const handleMouseUp = () => {
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);

        const m = marqueeRef.current;
        marqueeRef.current = null;
        setMarquee(null);

        if (!m) return;
        const selX = Math.min(m.startX, m.currentX);
        const selY = Math.min(m.startY, m.currentY);
        const selW = Math.abs(m.currentX - m.startX);
        const selH = Math.abs(m.currentY - m.startY);

        if (selW > 4 || selH > 4) {
          const hits = new Set<string>();
          for (const [id, pos] of Object.entries(positionsRef.current)) {
            if (
              pos.x < selX + selW &&
              pos.x + CARD_W > selX &&
              pos.y < selY + selH &&
              pos.y + CARD_H > selY
            ) {
              hits.add(id);
            }
          }
          setSelectedCardIds(
            additive ? new Set([...selectedCardIdsRef.current, ...hits]) : hits,
          );
        } else if (!additive) {
          setSelectedCardIds(new Set());
        }
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    } else {
      setSelectedCardIds(new Set());
    }
  }, []);

  // Marquee rectangle for rendering
  const marqueeRect = marquee
    ? {
        left: Math.min(marquee.startX, marquee.currentX),
        top: Math.min(marquee.startY, marquee.currentY),
        width: Math.abs(marquee.currentX - marquee.startX),
        height: Math.abs(marquee.currentY - marquee.startY),
      }
    : null;

  return {
    containerRef,
    positions,
    selectedCardIds,
    draggingCardIds,
    selectMode,
    setSelectMode,
    marqueeRect,
    handleCardMouseDown,
    handleContainerMouseDown,
  };
}
