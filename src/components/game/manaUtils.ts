import type { ActivatableAbilityInfo } from "@/types/manabrew";

/** Extract all mana letters from an ability description like "Add {G}." or "Add {W} or {U}." */
export function extractManaLetters(desc: string | undefined): string[] {
  if (!desc) return [];
  const matches = desc.matchAll(/\{([WUBRGC])\}/g);
  return Array.from(matches, (m) => m[1]);
}

export const ANY_COLOR_LETTERS = ["W", "U", "B", "R", "G"];

/**
 * Expand mana abilities by detecting color options in descriptions.
 */
export const getExpandedManaAbilities = (
  cardId: string,
  options: ActivatableAbilityInfo[],
): ActivatableAbilityInfo[] => {
  const cardAbs = options.filter((a) => a.cardId === cardId);
  if (cardAbs.length === 0) return [];

  const expanded: ActivatableAbilityInfo[] = [];

  for (const ab of cardAbs) {
    const letters = extractManaLetters(ab.description);
    const desc = ab.description.toLowerCase();
    const isAnyColor =
      desc.includes("any color") ||
      desc.includes("any one color") ||
      desc.includes("mana of any color");

    if (letters.length > 1) {
      // e.g. "Add {W} or {U}"
      letters.forEach((letter) => {
        expanded.push({
          ...ab,
          description: `Add {${letter}}`,
        });
      });
    } else if (letters.length === 1) {
      expanded.push(ab);
    } else if (isAnyColor) {
      ANY_COLOR_LETTERS.forEach((letter) => {
        expanded.push({
          ...ab,
          description: `Add {${letter}}`,
        });
      });
    } else {
      expanded.push(ab);
    }
  }

  return expanded;
};
