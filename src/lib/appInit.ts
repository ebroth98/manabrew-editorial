import { fetchSets } from "@/api/scryfall";
import { useScryfallStore, prefetchCards } from "@/stores/useScryfallStore";
import { useDeckStore } from "@/stores/useDeckStore";
import { getDefaultGameRuntime } from "@/game";
import { resolveCoverCard } from "@/components/deck/deckCover.utils";
import type { Deck } from "@/types/openmagic";

// One-shot app initialization. Idempotent — multiple callers (e.g. React
// StrictMode double-mount, or a future re-init flow) share the same promise.
let initPromise: Promise<void> | null = null;

async function initScryfallSets(): Promise<void> {
  if (useScryfallStore.getState().sets?.length) return;
  const sets = await fetchSets();
  useScryfallStore.setState({ sets });
}

type CoverLookup = { name: string; setCode?: string; cardNumber?: string };

function coverCardOf(deck: Deck): CoverLookup | null {
  const card = resolveCoverCard(deck);
  if (!card?.name) return null;
  return {
    name: card.name,
    setCode: card.setCode || undefined,
    cardNumber: card.cardNumber || undefined,
  };
}

async function prefetchDeckCovers(): Promise<void> {
  const presetDecks = await getDefaultGameRuntime()
    .api.getPresetDecks()
    .catch((e) => {
      console.error("[appInit] failed to load preset decks:", e);
      return [] as Deck[];
    });
  const { savedDecks = [], currentDeck } = useDeckStore.getState();
  const seen = new Set<string>();
  const covers: CoverLookup[] = [];
  const push = (c: CoverLookup | null) => {
    if (!c) return;
    const k = `${c.name.toLowerCase()}::${(c.setCode ?? "").toLowerCase()}::${(c.cardNumber ?? "").toLowerCase()}`;
    if (seen.has(k)) return;
    seen.add(k);
    covers.push(c);
  };
  // Every preset deck, every saved deck (including drafts — `savedDecks` is
  // unfiltered), and the currently-edited deck. No format filtering: the
  // store doesn't filter by format and `list_preset_decks` returns the full
  // registry, so this covers every deck the user can see.
  for (const d of presetDecks) push(coverCardOf(d));
  for (const sd of savedDecks) push(coverCardOf(sd.deck));
  if (currentDeck) push(coverCardOf(currentDeck));
  console.log(
    `[appInit] prefetching ${covers.length} deck covers (preset=${presetDecks.length}, saved=${savedDecks.length}, current=${currentDeck ? 1 : 0})`,
  );
  await prefetchCards(covers);
}

export function initApp(): Promise<void> {
  console.log("[appInit] initializing...");
  if (initPromise) return initPromise;
  initPromise = Promise.all([
    initScryfallSets().catch((e) => console.error("[appInit] sets failed:", e)),
    prefetchDeckCovers().catch((e) => console.error("[appInit] cover prefetch failed:", e)),
  ]).then(() => undefined);
  return initPromise;
}
