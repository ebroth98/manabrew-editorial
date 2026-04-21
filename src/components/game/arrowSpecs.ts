/**
 * Pure helpers for producing `ArrowSpec[]` from the current game state.
 *
 * Arrows are reserved for combat declarations (attack/block) and the
 * placement preview when casting a permanent spell. Every other targeting
 * relation is emitted by `buildPointerSpecs` as a `PointerSpec` instead.
 *
 * The Pixi renderer resolves these specs to canvas-local coordinates
 * every tick so arrows follow animating sprites.
 */

import type { ArrowSpec } from "@/pixi/types";
import type { StackObject } from "@/types/openmagic";
import { PromptType as PT, type PromptType } from "@/types/promptType";

export interface BuildArrowSpecsOptions {
  promptType?: PromptType;
  attackerIds: string[];
  blockAssignments: { blockerId: string; attackerId: string }[];
  combatAssignments: { blockerId: string; attackerId: string }[];
  pendingAttackers: string[];
  myPlayerId: string;
  opponentPlayerId: string;
  stack?: StackObject[];
  /** If set, treat this stack object as the "active" source for the
   *  placement arrow (usually the hovered one) instead of the top-of-stack. */
  activeStackObjectId?: string | null;
}

function getActiveStackObject(
  stack: StackObject[] | undefined,
  activeStackObjectId?: string | null,
): StackObject | null {
  if (!stack || stack.length === 0) return null;
  if (activeStackObjectId) {
    const hit = stack.find((obj) => obj.id === activeStackObjectId);
    if (hit) return hit;
  }
  return stack[stack.length - 1] ?? null;
}

export function buildArrowSpecs(opts: BuildArrowSpecsOptions): ArrowSpec[] {
  const {
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId,
    opponentPlayerId,
    stack,
    activeStackObjectId,
  } = opts;

  const specs: ArrowSpec[] = [];

  if (promptType === PT.ChooseBlockers) {
    for (const id of attackerIds) {
      specs.push({
        from: { kind: "card", id },
        to: { kind: "player", id: myPlayerId },
        type: "attack",
      });
    }
    for (const { blockerId, attackerId } of blockAssignments) {
      specs.push({
        from: { kind: "card", id: blockerId },
        to: { kind: "card", id: attackerId },
        type: "block",
      });
    }
  }

  for (const { blockerId, attackerId } of combatAssignments) {
    specs.push({
      from: { kind: "card", id: blockerId },
      to: { kind: "card", id: attackerId },
      type: "block",
    });
  }

  if (promptType === PT.ChooseAttackers) {
    for (const id of pendingAttackers) {
      specs.push({
        from: { kind: "card", id },
        to: { kind: "player", id: opponentPlayerId },
        type: "attack",
      });
    }
  }

  // Placement ghost preview stays as an arrow (dashed marching-ants) — it
  // signals "drop here" rather than a targeting relationship.
  const activeObj = getActiveStackObject(stack, activeStackObjectId);
  if (activeObj && activeObj.isPermanentSpell === true) {
    const hasTargets = Array.isArray((activeObj as unknown as Record<string, unknown>).targets)
      && ((activeObj as unknown as { targets: unknown[] }).targets.length > 0);
    if (!hasTargets) {
      specs.push({
        from: { kind: "stack", id: activeObj.id },
        to: { kind: "placement-ghost" },
        type: "placement",
      });
    }
  }

  return specs;
}
