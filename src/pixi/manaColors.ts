/**
 * Mana-symbol background colors (hex) for Pixi overlays.
 *
 * Mirrors the RGBA values in `ManaAbilityTapButton.tsx` so the canvas and
 * React renderers show the same button tint for each color. Alpha is
 * applied at paint time via `MANA_BUTTON_ALPHA` / `MANA_BUTTON_HOVER_ALPHA`
 * in constants.ts.
 */

export type ManaLetter = "W" | "U" | "B" | "R" | "G" | "C";

export const MANA_COLORS_HEX: Record<ManaLetter, number> = {
  W: 0xf8f6d8,
  U: 0xc1d7e9,
  B: 0xbab1ab,
  R: 0xeb9f82,
  G: 0xc4d3ca,
  C: 0xcccac7,
};

const MANA_LETTERS = new Set(Object.keys(MANA_COLORS_HEX) as ManaLetter[]);

export const isManaLetter = (value: string | undefined): value is ManaLetter =>
  value != null && MANA_LETTERS.has(value as ManaLetter);

export const manaColorFor = (letter: string | undefined, fallback = 0x000000): number =>
  isManaLetter(letter) ? MANA_COLORS_HEX[letter] : fallback;
