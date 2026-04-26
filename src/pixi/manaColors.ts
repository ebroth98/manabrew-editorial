/**
 * Mana-letter utilities for the Pixi canvas.
 *
 * The tint values themselves live in the theme (`mana.W` … `mana.C`); this
 * module only provides the letter-type guard and a convenience lookup that
 * reads from a `GameThemeColors` and converts inline.
 */

import type { Theme } from "@/hooks/useTheme";
import { hexToNum } from "./colorUtils";
import { MANA_LETTERS, type ManaLetter } from "@/themes/gameTheme";

export type { ManaLetter } from "@/themes/gameTheme";

const MANA_SET = new Set<ManaLetter>(MANA_LETTERS);

export const isManaLetter = (value: string | undefined): value is ManaLetter =>
  value != null && MANA_SET.has(value as ManaLetter);

export const manaColorFor = (letter: string | undefined, theme: Theme, fallback: number): number =>
  isManaLetter(letter) ? hexToNum(theme.gameTheme.mana[letter]) : fallback;
