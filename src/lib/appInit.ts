import { fetchSets } from "@/api/scryfall";
import { useScryfallStore, prefetchCards } from "@/stores/useScryfallStore";
import { useDeckStore } from "@/stores/useDeckStore";
import { getDefaultGameRuntime } from "@/game";
import { resolveCoverCard } from "@/components/deck/deckCover.utils";
import { prefetchPresetDecks } from "@/stores/usePresetDecksStore";
import type { Deck } from "@/types/manabrew";

let initPromise: Promise<void> | null = null;

async function initScryfallSets(): Promise<void> {
  if (useScryfallStore.getState().sets?.length) return;
  const sets = await fetchSets();
  useScryfallStore.setState({ sets });
}

type CardLookup = { name: string; setCode?: string; cardNumber?: string };

function lookupOfCard(
  card: { name: string; setCode?: string; cardNumber?: string } | null | undefined,
): CardLookup | null {
  if (!card?.name) return null;
  return {
    name: card.name,
    setCode: card.setCode || undefined,
    cardNumber: card.cardNumber || undefined,
  };
}

function allCardsOfDeck(deck: Deck): CardLookup[] {
  const lookups: CardLookup[] = [];
  const sections = [
    deck.cards,
    deck.sideboard,
    deck.maybeboard ?? [],
    deck.attractions ?? [],
    deck.contraptions ?? [],
    deck.schemes ?? [],
    deck.planes ?? [],
    deck.commanders ?? [],
  ];
  for (const section of sections) {
    for (const card of section) {
      const lookup = lookupOfCard(card);
      if (lookup) lookups.push(lookup);
    }
  }
  return lookups;
}

async function prefetchAllDeckCards(): Promise<void> {
  const presetDecks = await getDefaultGameRuntime()
    .api.getPresetDecks()
    .catch((e) => {
      console.error("[appInit] failed to load preset decks:", e);
      return [] as Deck[];
    });
  const { savedDecks = [], currentDeck } = useDeckStore.getState();
  const seen = new Set<string>();
  const lookups: CardLookup[] = [];
  const push = (c: CardLookup | null) => {
    if (!c) return;
    const k = `${c.name.toLowerCase()}::${(c.setCode ?? "").toLowerCase()}::${(c.cardNumber ?? "").toLowerCase()}`;
    if (seen.has(k)) return;
    seen.add(k);
    lookups.push(c);
  };
  // Cover first so deck-list thumbnails paint before per-card images.
  for (const d of presetDecks) push(lookupOfCard(resolveCoverCard(d)));
  for (const sd of savedDecks) push(lookupOfCard(resolveCoverCard(sd.deck)));
  if (currentDeck) push(lookupOfCard(resolveCoverCard(currentDeck)));
  for (const d of presetDecks) for (const l of allCardsOfDeck(d)) push(l);
  for (const sd of savedDecks) for (const l of allCardsOfDeck(sd.deck)) push(l);
  if (currentDeck) for (const l of allCardsOfDeck(currentDeck)) push(l);
  console.log(
    `[appInit] prefetching ${lookups.length} deck cards (preset=${presetDecks.length}, saved=${savedDecks.length}, current=${currentDeck ? 1 : 0})`,
  );
  await prefetchCards(lookups);
}

export function initApp(): Promise<void> {
  console.log("[appInit] initializing...");
  if (initPromise) return initPromise;
  initPromise = (async () => {
    // Preset metadata enrichment runs first so it lands in the Scryfall
    // cache before per-card prefetch issues duplicate /cards/named lookups.
    await Promise.all([
      initScryfallSets().catch((e) => console.error("[appInit] sets failed:", e)),
      prefetchPresetDecks().catch((e) => console.error("[appInit] preset enrichment failed:", e)),
    ]);
    await prefetchAllDeckCards().catch((e) =>
      console.error("[appInit] deck card prefetch failed:", e),
    );
  })();
  return initPromise;
}
