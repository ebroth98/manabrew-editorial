import type { Deck, DeckCard, GameCard } from "@/types/manabrew";
import { peekArchivedToken } from "@/stores/useScryfallStore";

/** Engine emits token names with a "Token" suffix ("Map Token", "Blood
 *  Token"); Scryfall (and so the archive + user-pinned `deck.tokens`)
 *  stores them as bare names ("Map", "Blood"). Compare on the bare form. */
function normalizeTokenName(name: string): string {
  return name.toLowerCase().replace(/\s+token$/i, "");
}

export function asDeckCard(deck: Deck, gameCard: GameCard): DeckCard {
  const pool = getDeckCardPool(deck);
  const exact = pool.find(
    (c) =>
      c.name === gameCard.name &&
      c.setCode === gameCard.setCode &&
      c.cardNumber === gameCard.cardNumber,
  );
  if (exact) return exact;
  if (gameCard.isToken) {
    const target = normalizeTokenName(gameCard.name);
    const byName = pool.find(
      (c) => c.name === gameCard.name || normalizeTokenName(c.name) === target,
    );
    if (byName) return byName;
    const token = peekArchivedToken({
      name: gameCard.name,
      setCode: gameCard.setCode,
      cardNumber: gameCard.cardNumber,
    });
    if (token) return token;
    throw new Error(
      `Token archive has no entry for ${gameCard.name} (${gameCard.setCode}#${gameCard.cardNumber})`,
    );
  }
  throw new Error(
    `No DeckCard in "${deck.name}" for ${gameCard.name} (${gameCard.setCode}#${gameCard.cardNumber})`,
  );
}

export function getDeckCardPool(deck: Deck): DeckCard[] {
  return [
    ...deck.cards,
    ...deck.sideboard,
    ...(deck.attractions ?? []),
    ...(deck.contraptions ?? []),
    ...(deck.schemes ?? []),
    ...(deck.planes ?? []),
    ...(deck.commanders ?? []),
    ...(deck.tokens ?? []),
  ];
}

export function getDeckCardNames(deck: Deck): string[] {
  return [...deck.cards, ...(deck.commanders ?? [])].map((c) => c.name);
}

export function getDeckFingerprint(deck: Deck): string {
  const tag = (section: string, list: DeckCard[]) =>
    list.map((c) => `${section}:${c.name}:${c.setCode}`);
  const serialized = [
    ...tag("main", deck.cards),
    ...tag("sideboard", deck.sideboard),
    ...tag("attractions", deck.attractions ?? []),
    ...tag("contraptions", deck.contraptions ?? []),
    ...tag("schemes", deck.schemes ?? []),
    ...tag("planes", deck.planes ?? []),
    ...tag("commander", deck.commanders ?? []),
  ].sort();
  return JSON.stringify({
    name: deck.name,
    format: deck.format ?? "standard",
    commander: deck.commanders?.[0]?.name ?? null,
    cards: serialized,
  });
}
