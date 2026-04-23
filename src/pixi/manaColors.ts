/**
 * Mana-letter utilities for the Pixi canvas.
 *
 * The tint values themselves live in the theme (`mana.W` … `mana.C`); this
 * module only provides the letter-type guard and a convenience lookup that
 * reads from a `GameThemeColors` and converts inline.
 */

import type { AppTheme } from "@/hooks/useTheme";
import { hexToNum } from "./colorUtils";

export type ManaLetter = "W" | "U" | "B" | "R" | "G" | "C";

const MANA_LETTERS = new Set<ManaLetter>(["W", "U", "B", "R", "G", "C"]);

export const isManaLetter = (value: string | undefined): value is ManaLetter =>
  value != null && MANA_LETTERS.has(value as ManaLetter);

export const manaColorFor = (
  letter: string | undefined,
  theme: AppTheme,
  fallback: number,
): number => (isManaLetter(letter) ? hexToNum(theme.game.mana[letter]) : fallback);
