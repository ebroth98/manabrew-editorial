/**
 * useGameArrows — hook that maps game state to arrow definitions.
 *
 * Reads DOM element positions via data-card-id / data-player-id attributes
 * and produces ArrowDef[] for the ArrowOverlay component.
 *
 * Arrow scenarios handled:
 *   chooseBlockers  — orange attack arrows (attacker → my player)
 *                     red block arrows (blocker → attacker, live as user assigns)
 *   chooseAttackers — orange preview arrows (pending attacker → opponent player)
 */

import { type RefObject, useEffect, useState } from "react";
import type { ArrowDef } from "./ArrowOverlay";
import type { PromptType } from "@/types/promptType";
import { PromptType as PT } from "@/types/promptType";

// ─── Types ───────────────────────────────────────────────────────────────────

export interface BlockAssignment {
  blockerId: string;
  attackerId: string;
}

export interface UseGameArrowsOptions {
  /** Ref to the outermost game container (position: relative). */
  containerRef: RefObject<HTMLElement | null>;
  /** Current prompt type from the backend, e.g. "chooseBlockers". */
  promptType: PromptType | undefined;
  /** Opponent's attacking creature IDs (from ChooseBlockers prompt). */
  attackerIds: string[];
  /** Block assignments the human player has declared so far. */
  blockAssignments: BlockAssignment[];
  /** Authoritative blocker assignments from the game snapshot. */
  combatAssignments: BlockAssignment[];
  /** Creature IDs the human player has selected as attackers (chooseAttackers). */
  pendingAttackers: string[];
  /** The human player's ID (for "attacker → me" arrows). */
  myPlayerId: string;
  /** The opponent player's ID (for "my attacker → them" arrows). */
  opponentPlayerId: string;
}

// ─── DOM helpers ─────────────────────────────────────────────────────────────

/**
 * Return the center of the element matching `selector`, expressed in
 * coordinates relative to `container`'s top-left corner.
 * Returns null if the element is not found or not visible.
 */
function getCenter(
  container: HTMLElement,
  selector: string,
): { x: number; y: number } | null {
  const el = container.querySelector(selector);
  if (!el) return null;
  const cr = container.getBoundingClientRect();
  const er = el.getBoundingClientRect();
  // Skip elements that have no size (hidden / not rendered)
  if (er.width === 0 && er.height === 0) return null;
  return {
    x: er.left + er.width / 2 - cr.left,
    y: er.top + er.height / 2 - cr.top,
  };
}

function cardCenter(container: HTMLElement, cardId: string) {
  return getCenter(container, `[data-card-id="${CSS.escape(cardId)}"]`);
}

function playerCenter(container: HTMLElement, playerId: string) {
  return getCenter(container, `[data-player-id="${CSS.escape(playerId)}"]`);
}

// ─── Arrow builder ────────────────────────────────────────────────────────────

/**
 * Pure function: given a DOM container and game state snapshot,
 * produce the list of arrows to render.
 */
function buildArrows(
  container: HTMLElement,
  opts: Omit<UseGameArrowsOptions, "containerRef">,
): ArrowDef[] {
  const {
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId,
    opponentPlayerId,
  } = opts;

  const arrows: ArrowDef[] = [];

  if (promptType === PT.ChooseBlockers) {
    // 1. Orange attack arrows: each opponent attacker → my player panel
    const myPos = playerCenter(container, myPlayerId);
    for (const id of attackerIds) {
      const from = cardCenter(container, id);
      if (from && myPos) {
        arrows.push({
          fromX: from.x,
          fromY: from.y,
          toX: myPos.x,
          toY: myPos.y,
          type: "attack",
        });
      }
    }

    // 2. Red block arrows: my blocker → the attacker it's assigned to
    for (const { blockerId, attackerId } of blockAssignments) {
      const from = cardCenter(container, blockerId);
      const to = cardCenter(container, attackerId);
      if (from && to) {
        arrows.push({
          fromX: from.x,
          fromY: from.y,
          toX: to.x,
          toY: to.y,
          type: "block",
        });
      }
    }
  }

  // Persist block arrows after declaration using authoritative combat snapshot data.
  for (const { blockerId, attackerId } of combatAssignments) {
    const from = cardCenter(container, blockerId);
    const to = cardCenter(container, attackerId);
    if (from && to) {
      arrows.push({
        fromX: from.x,
        fromY: from.y,
        toX: to.x,
        toY: to.y,
        type: "block",
      });
    }
  }

  if (promptType === PT.ChooseAttackers) {
    // Orange preview arrows: selected attacker → opponent player panel
    const oppPos = playerCenter(container, opponentPlayerId);
    for (const id of pendingAttackers) {
      const from = cardCenter(container, id);
      if (from && oppPos) {
        arrows.push({
          fromX: from.x,
          fromY: from.y,
          toX: oppPos.x,
          toY: oppPos.y,
          type: "attack",
        });
      }
    }
  }

  return arrows;
}

// ─── Hook ────────────────────────────────────────────────────────────────────

/**
 * Measures DOM positions and returns arrows to pass to <ArrowOverlay>.
 *
 * Re-measures whenever any relevant piece of state changes.
 * Also installs a ResizeObserver on the container so arrows stay correct
 * when the window is resized.
 */
export function useGameArrows(opts: UseGameArrowsOptions): ArrowDef[] {
  const [arrows, setArrows] = useState<ArrowDef[]>([]);

  const {
    containerRef,
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId,
    opponentPlayerId,
  } = opts;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      setArrows([]);
      return;
    }

    // Measure immediately after React commits the DOM
    function measure() {
      if (!container) return;
      setArrows(
        buildArrows(container, {
          promptType,
          attackerIds,
          blockAssignments,
          combatAssignments,
          pendingAttackers,
          myPlayerId,
          opponentPlayerId,
        }),
      );
    }

    measure();

    // Re-measure on container resize (handles window resize / layout reflow)
    const ro = new ResizeObserver(measure);
    ro.observe(container);
    return () => ro.disconnect();
  }, [
    containerRef,
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId,
    opponentPlayerId,
  ]);

  return arrows;
}
