/**
 * Pure helpers for producing `ArrowSpec[]` from the current game state.
 *
 * The Pixi renderer resolves these specs to canvas-local coordinates
 * every tick so arrows follow animating sprites. The React SVG overlay
 * still uses `useGameArrows` which resolves to DOM positions directly.
 * Both code paths start from the same set of logical arrows so behaviour
 * stays identical.
 */

import type { ArrowSpec, ArrowEndpoint } from "@/pixi/types";
import type { StackObject, StackTarget } from "@/types/openmagic";
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
  /** If set, treat this stack object as the "active" source for the stack
   *  target arrows (usually the hovered one) instead of the top-of-stack. */
  activeStackObjectId?: string | null;
}

function getAllTargets(obj: StackObject): StackTarget[] {
  const maybeObj = obj as unknown as Record<string, unknown>;
  const targets = Array.isArray(maybeObj.targets)
    ? (maybeObj.targets as StackTarget[])
    : [];
  if (targets.length > 0) return targets;
  const legacy = typeof maybeObj.targetCardId === "string" ? maybeObj.targetCardId : null;
  if (!legacy) return [];
  return [{ kind: "card", id: legacy, nodeIndex: 0, targetIndex: 0, hostile: true }];
}

function targetEndpoint(target: StackTarget): ArrowEndpoint | null {
  switch (target.kind) {
    case "card":   return { kind: "card", id: target.id };
    case "player": return { kind: "player", id: target.id };
    case "stack":  return { kind: "stack", id: target.id };
    default:       return null;
  }
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
  return stack[stack.length - 1];
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

  const activeObj = getActiveStackObject(stack, activeStackObjectId);
  if (activeObj) {
    const targets = getAllTargets(activeObj);
    if (targets.length > 0) {
      for (const t of targets) {
        const ep = targetEndpoint(t);
        if (!ep) continue;
        specs.push({
          from: { kind: "stack", id: activeObj.id },
          to: ep,
          type: t.hostile ? "hostile-target" : "friendly-target",
        });
      }
    } else if (activeObj.isPermanentSpell === true) {
      specs.push({
        from: { kind: "stack", id: activeObj.id },
        to: { kind: "placement-ghost" },
        type: "placement",
      });
    } else if (activeObj.sourceId) {
      specs.push({
        from: { kind: "stack", id: activeObj.id },
        to: { kind: "card", id: activeObj.sourceId },
        type: "friendly-target",
      });
    }
  }

  return specs;
}
