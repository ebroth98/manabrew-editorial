import { CssDiceAnimator } from "./animator/CssDiceAnimator";
import type { DiceRollAnimationProps } from "./types";

/**
 * Public entry-point for the dice roll animation. Currently delegates to
 * the CSS implementation; swapping in pixi/three.js later is a one-line
 * change here — every consumer depends on this component, not the impl.
 */
export function DiceRollAnimation(props: DiceRollAnimationProps) {
  return <CssDiceAnimator {...props} />;
}
