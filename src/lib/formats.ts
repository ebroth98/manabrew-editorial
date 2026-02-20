// Mirrors Forge's DeckFormat (structural rules) + GameFormat (card legality).
// For our limited card pool, we combine both into a single GameFormat interface.

export interface GameFormat {
  id: string;
  name: string;
  /** Short label used in badges, e.g. "Constr." / "EDH" */
  shortName: string;
  description: string;
  /** Tailwind color variant key for FormatBadge */
  badgeColor: string;
  deckRules: {
    minDeckSize: number;
    maxDeckSize: number | null; // null = unlimited
    maxCopies: number; // 4 for Constructed, 1 for Commander
    sideboardMax: number;
    startingLife: number;
    requiresCommander: boolean;
  };
  bannedCards: string[];
}

/** Basic land names are exempt from the max-copies rule. */
export const BASIC_LAND_NAMES = new Set([
  "Plains",
  "Island",
  "Swamp",
  "Mountain",
  "Forest",
]);

export const GAME_FORMATS: GameFormat[] = [
  {
    id: "constructed",
    name: "Constructed",
    shortName: "Constr.",
    description: "60+ cards, max 4 copies, 20 life",
    badgeColor: "blue",
    deckRules: {
      minDeckSize: 60,
      maxDeckSize: null,
      maxCopies: 4,
      sideboardMax: 15,
      startingLife: 20,
      requiresCommander: false,
    },
    bannedCards: [],
  },
  {
    id: "commander",
    name: "Commander",
    shortName: "EDH",
    description: "100 cards, singleton, 40 life",
    badgeColor: "purple",
    deckRules: {
      minDeckSize: 100,
      maxDeckSize: 100,
      maxCopies: 1,
      sideboardMax: 10,
      startingLife: 40,
      requiresCommander: true,
    },
    bannedCards: [],
  },
];

export interface DeckValidation {
  legal: boolean;
  errors: string[];
}

export function getFormat(id: string): GameFormat | undefined {
  return GAME_FORMATS.find((f) => f.id === id);
}

/**
 * Validate a deck (as an array of card names, one per copy) against a format.
 * Basic lands are exempt from the per-card copy limit.
 */
export function validateDeck(
  cardNames: string[],
  format: GameFormat
): DeckValidation {
  const errors: string[] = [];
  const { minDeckSize, maxDeckSize, maxCopies } = format.deckRules;

  if (cardNames.length < minDeckSize) {
    errors.push(
      `Deck must have at least ${minDeckSize} cards (has ${cardNames.length})`
    );
  }
  if (maxDeckSize !== null && cardNames.length > maxDeckSize) {
    errors.push(
      `Deck must have at most ${maxDeckSize} cards (has ${cardNames.length})`
    );
  }

  // Count copies and check against limit
  const counts = new Map<string, number>();
  for (const name of cardNames) {
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  for (const [name, count] of counts) {
    if (!BASIC_LAND_NAMES.has(name) && count > maxCopies) {
      errors.push(
        `Too many copies of "${name}": ${count} (max ${maxCopies})`
      );
    }
  }

  // Check banned list
  const seenBanned = new Set<string>();
  for (const name of cardNames) {
    if (format.bannedCards.includes(name) && !seenBanned.has(name)) {
      errors.push(`"${name}" is banned in ${format.name}`);
      seenBanned.add(name);
    }
  }

  return { legal: errors.length === 0, errors };
}

/**
 * Returns all formats the deck is legal in.
 * Mirrors Forge's GameFormat.Collection.getAllFormatsOfDeck().
 */
export function inferFormats(cardNames: string[]): GameFormat[] {
  return GAME_FORMATS.filter((f) => validateDeck(cardNames, f).legal);
}
