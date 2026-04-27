/**
 * Shared types for the dice-roll UI subsystem.
 *
 * Renderer-agnostic — `DiceRollAnimationProps` is the single contract any
 * animation implementation must satisfy. The default impl is CSS-based
 * (`CssDiceAnimator`); a future pixi/three.js impl just needs to match
 * the same props shape.
 */

export interface DiceRollSpec {
  /** Number of sides on each die (e.g. 6, 20). */
  sides: number;
  /** Pre-modifier values, one per kept die. */
  naturalResults: number[];
  /** Post-modifier values, one per kept die. */
  finalResults: number[];
  /** Rolls dropped before modification (ignore-lowest, choose-to-ignore). */
  ignoredRolls?: number[];
}

/** A single die in a static (non-animated) face render. */
export interface DieFace {
  sides: number;
  value: number;
}

/** Contract for any concrete dice-roll animation component. */
export interface DiceRollAnimationProps {
  spec: DiceRollSpec;
  /** Fired once the animation (including its dwell phase) completes. */
  onComplete?: () => void;
  /**
   * Optional theme-token color (CSS color string). When supplied, every
   * die in this animation is tinted with the color so the roll's
   * source player is visually clear.
   */
  accentColor?: string;
  className?: string;
}
