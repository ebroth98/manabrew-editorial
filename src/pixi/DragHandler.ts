import { CARD_W, CARD_H } from "@/components/game/game.constants";
import type { ScreenPos } from "./types";

const MOVE_THRESHOLD = 5;
const JUST_DRAGGED_CLEAR_MS = 300;

interface DragState {
  cardIds: string[];
  startPositions: Map<string, ScreenPos>;
  startMouseX: number;
  startMouseY: number;
  hasMoved: boolean;
}

interface HandExclusion {
  xStart: number;
  xEnd: number;
  topY: number;
}

interface ExtraBlockerRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export class DragHandler {
  private drag: DragState | null = null;
  private containerWidth = 0;
  private containerHeight = 0;
  private leftReserved = 0;
  private rightReserved = 0;
  private bottomReserved = 0;
  /**
   * Horizontal band occupied by the hand fan. When set, dragging is only
   * clamped vertically where the card's x overlaps this band — outside it,
   * cards can travel all the way to the bottom of the canvas.
   */
  private handExclusion: HandExclusion | null = null;
  /** Additional keep-out rects (UI overlays) that the drag must avoid. */
  private extraBlockers: ExtraBlockerRect[] = [];
  justDraggedCardIds = new Set<string>();
  private justDraggedTimer: ReturnType<typeof setTimeout> | null = null;

  setContainerSize(w: number, h: number): void {
    this.containerWidth = w;
    this.containerHeight = h;
  }

  setReserved(left: number, right: number, bottom: number): void {
    this.leftReserved = left;
    this.rightReserved = right;
    this.bottomReserved = bottom;
  }

  setHandExclusion(rect: { x: number; y: number; width: number; height: number } | null): void {
    this.handExclusion = rect
      ? { xStart: rect.x, xEnd: rect.x + rect.width, topY: rect.y }
      : null;
  }

  setExtraBlockers(rects: ExtraBlockerRect[]): void {
    this.extraBlockers = rects;
  }

  get isDragging(): boolean {
    return this.drag !== null && this.drag.hasMoved;
  }

  get draggingCardIds(): Set<string> {
    if (!this.drag) return new Set();
    return new Set(this.drag.cardIds);
  }

  start(
    cardId: string,
    mouseX: number,
    mouseY: number,
    selectedCardIds: Set<string>,
    currentPositions: Map<string, ScreenPos>,
    shift: boolean,
  ): Set<string> {
    let selection = new Set(selectedCardIds);

    if (shift) {
      if (selection.has(cardId)) {
        selection.delete(cardId);
      } else {
        selection.add(cardId);
      }
    } else if (!selection.has(cardId)) {
      selection = new Set([cardId]);
    }

    const dragCards = [...selection];
    const startPositions = new Map<string, ScreenPos>();
    for (const id of dragCards) {
      const pos = currentPositions.get(id);
      if (pos) startPositions.set(id, { ...pos });
    }

    this.drag = {
      cardIds: dragCards,
      startPositions,
      startMouseX: mouseX,
      startMouseY: mouseY,
      hasMoved: false,
    };

    return selection;
  }

  move(mouseX: number, mouseY: number): Map<string, ScreenPos> | null {
    if (!this.drag) return null;

    const dx = mouseX - this.drag.startMouseX;
    const dy = mouseY - this.drag.startMouseY;

    if (!this.drag.hasMoved) {
      if (Math.abs(dx) < MOVE_THRESHOLD && Math.abs(dy) < MOVE_THRESHOLD) {
        return null;
      }
      this.drag.hasMoved = true;
    }

    const xMin = Math.max(0, this.leftReserved) + CARD_W / 2;
    const xMax = Math.max(xMin, this.containerWidth - CARD_W / 2 - this.rightReserved);
    const yMin = CARD_H / 2;
    const yMaxFloor = this.handExclusion
      ? this.containerHeight - CARD_H / 2
      : Math.max(yMin, this.containerHeight - CARD_H / 2 - this.bottomReserved);

    const positions = new Map<string, ScreenPos>();
    for (const [id, start] of this.drag.startPositions) {
      const x = Math.max(xMin, Math.min(xMax, start.x + dx));
      let yMax = yMaxFloor;

      if (this.handExclusion) {
        const cardLeft = x - CARD_W / 2;
        const cardRight = x + CARD_W / 2;
        const overlapsHand =
          cardLeft < this.handExclusion.xEnd && cardRight > this.handExclusion.xStart;
        if (overlapsHand) {
          yMax = Math.max(yMin, this.handExclusion.topY - CARD_H / 2);
        }
      }

      // Extra keep-out rects (e.g. the PASS button cluster at bottom-right).
      // If the card's horizontal span overlaps a blocker, clamp yMax so the
      // card stays above it.
      for (const rect of this.extraBlockers) {
        const cardLeft = x - CARD_W / 2;
        const cardRight = x + CARD_W / 2;
        const overlapsX = cardLeft < rect.x + rect.width && cardRight > rect.x;
        if (overlapsX) {
          yMax = Math.min(yMax, Math.max(yMin, rect.y - CARD_H / 2));
        }
      }

      const y = Math.max(yMin, Math.min(yMax, start.y + dy));
      positions.set(id, { x, y });
    }

    return positions;
  }

  end(): { positions: Map<string, ScreenPos>; wasDrag: boolean } | null {
    if (!this.drag) return null;

    const result = {
      positions: new Map<string, ScreenPos>(),
      wasDrag: this.drag.hasMoved,
    };

    if (this.drag.hasMoved) {
      if (this.justDraggedTimer) clearTimeout(this.justDraggedTimer);
      this.justDraggedCardIds = new Set(this.drag.cardIds);
      this.justDraggedTimer = setTimeout(() => {
        this.justDraggedCardIds.clear();
      }, JUST_DRAGGED_CLEAR_MS);
    }

    this.drag = null;
    return result;
  }

  cancel(): void {
    this.drag = null;
  }

  destroy(): void {
    if (this.justDraggedTimer) clearTimeout(this.justDraggedTimer);
    this.justDraggedCardIds.clear();
  }
}
