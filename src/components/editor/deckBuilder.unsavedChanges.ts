import { useEffect, useState } from "react";
import { useDeckStore } from "@/stores/useDeckStore";
import type { Card } from "@/types/openmagic";

let _hasUnsavedChanges = false;
const _listeners = new Set<() => void>();
let _lastSavedSnapshotRef: string | null = null;

export function setUnsavedState(snapshot: string, current: string) {
  const next = current !== snapshot;
  if (next !== _hasUnsavedChanges) {
    _hasUnsavedChanges = next;
    _listeners.forEach((fn) => fn());
  }
}

export function setLastSavedSnapshotRef(snapshot: string | null) {
  _lastSavedSnapshotRef = snapshot;
}

export function buildDeckSnapshot(deck: {
  format?: string;
  draft?: boolean;
  cards: Card[];
  commanders?: Card[];
  sideboard: Card[];
  maybeboard?: Card[];
  attractions?: Card[];
  contraptions?: Card[];
  schemes?: Card[];
  planes?: Card[];
  name: string;
  stackPositions?: Record<string, { x: number; y: number }>;
}): string {
  return JSON.stringify({
    format: deck.format,
    cards: deck.cards,
    commanders: deck.commanders ?? [],
    sideboard: deck.sideboard,
    maybeboard: deck.maybeboard ?? [],
    attractions: deck.attractions ?? [],
    contraptions: deck.contraptions ?? [],
    schemes: deck.schemes ?? [],
    planes: deck.planes ?? [],
    name: deck.name,
    stackPositions: deck.stackPositions,
  });
}

/** Hook to read unsaved changes state from outside DeckBuilder. */
export function useDeckUnsavedChanges(): boolean {
  const [, forceUpdate] = useState(0);
  useEffect(() => {
    const listener = () => forceUpdate((n) => n + 1);
    _listeners.add(listener);
    return () => {
      _listeners.delete(listener);
    };
  }, []);
  return _hasUnsavedChanges;
}

/** Revert currentDeck to the last saved snapshot. Called when user leaves without saving. */
export function revertDeckToLastSaved() {
  if (!_lastSavedSnapshotRef) return;
  try {
    const deck = JSON.parse(_lastSavedSnapshotRef);
    useDeckStore.getState().loadDeck(deck);
  } catch {
    /* ignore parse errors */
  }
}
