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
import type { StackObject, StackTarget } from "@/types/xmage";
import { useStackUIStore } from "@/stores/useStackUIStore";

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
  /** Current stack objects (last item is top of stack). */
  stack?: StackObject[];
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

function stackObjectCenter(container: HTMLElement, stackObjectId: string) {
  return getCenter(container, `[data-stack-object-id="${CSS.escape(stackObjectId)}"]`);
}

function placementGhostCenter(container: HTMLElement) {
  return getCenter(container, "[data-placement-ghost]");
}

function getAllTargets(obj: StackObject): StackTarget[] {
  const maybeObj = obj as unknown as Record<string, unknown>;
  const targets = Array.isArray(maybeObj.targets)
    ? (maybeObj.targets as StackTarget[])
    : [];
  if (targets.length === 0) {
    const legacyTargetCardId =
      typeof maybeObj.targetCardId === "string" ? maybeObj.targetCardId : null;
    if (legacyTargetCardId) {
      return [{
        kind: "card",
        id: legacyTargetCardId,
        nodeIndex: 0,
        targetIndex: 0,
      }];
    }
    return [];
  }
  return targets;
}

function entityCenter(container: HTMLElement, target: StackTarget) {
  if (target.kind === "card") return cardCenter(container, target.id);
  if (target.kind === "player") return playerCenter(container, target.id);
  if (target.kind === "stack") return stackObjectCenter(container, target.id);
  return null;
}

function stackArrowType(target: StackTarget): ArrowDef["type"] {
  return target.hostile ? "hostile-target" : "friendly-target";
}

function isPermanentSpell(obj: StackObject): boolean {
  return obj.isPermanentSpell === true;
}

function getActiveStackObject(
  stack: StackObject[] | undefined,
  activeStackObjectId?: string | null,
): StackObject | null {
  if (!stack || stack.length === 0) return null;
  return (
    (activeStackObjectId
      ? stack.find((obj) => obj.id === activeStackObjectId)
      : null) ??
    stack[stack.length - 1]
  );
}

// ─── Arrow builder ────────────────────────────────────────────────────────────

/**
 * Pure function: given a DOM container and game state snapshot,
 * produce the list of arrows to render.
 */
function buildArrows(
  container: HTMLElement,
  opts: Omit<UseGameArrowsOptions, "containerRef">,
  activeStackObjectId?: string | null,
): ArrowDef[] {
  const {
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId,
    opponentPlayerId,
    stack,
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

  // Stack arrows: target arrows OR placement ghost arrow.
  const activeObj = getActiveStackObject(stack, activeStackObjectId);
  if (activeObj) {
    const targets = getAllTargets(activeObj);
    const from = stackObjectCenter(container, activeObj.id);
    if (from && targets.length > 0) {
      // Target arrows: stack object -> each explicit target
      for (const target of targets) {
        const to = entityCenter(container, target);
        if (to) {
          arrows.push({
            fromX: from.x,
            fromY: from.y,
            toX: to.x,
            toY: to.y,
            type: stackArrowType(target),
          });
        }
      }
    } else if (from && isPermanentSpell(activeObj)) {
      // Placement arrow: permanent spell with no targets -> ghost on battlefield
      const to = placementGhostCenter(container);
      if (to) {
        arrows.push({
          fromX: from.x,
          fromY: from.y,
          toX: to.x,
          toY: to.y,
          type: "placement",
        });
      }
    } else if (from && activeObj.sourceId) {
      // Ability with no explicit targets — arrow back to source card on battlefield
      const to = cardCenter(container, activeObj.sourceId);
      if (to) {
        arrows.push({
          fromX: from.x,
          fromY: from.y,
          toX: to.x,
          toY: to.y,
          type: "friendly-target",
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
    stack,
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
      const activeStackObjectId = useStackUIStore.getState().hoveredStackObjectId;
      setArrows(
        buildArrows(container, {
          promptType,
          attackerIds,
          blockAssignments,
          combatAssignments,
          pendingAttackers,
          myPlayerId,
          opponentPlayerId,
          stack,
        }, activeStackObjectId),
      );
    }

    measure();

    // Re-measure on container resize (handles window resize / layout reflow)
    const ro = new ResizeObserver(measure);
    ro.observe(container);
    const unsubStackHover = useStackUIStore.subscribe(() => measure());
    return () => {
      ro.disconnect();
      unsubStackHover();
    };
  }, [
    containerRef,
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId,
    opponentPlayerId,
    stack,
  ]);

  return arrows;
}
