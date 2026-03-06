package forge.harness;

import forge.game.card.CardCollection;
import forge.game.card.CardCollectionView;
import forge.util.collect.FCollectionView;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.Collections;
import java.util.List;
import java.util.Random;

/**
 * Harness-side choice sampling helpers.
 *
 * These utilities never invent legality: they only sample from option lists
 * already provided by Java engine callbacks (PlayerController methods).
 */
public final class ChoiceSpace {
    public static final class ChoiceResult<T> {
        public final List<T> sorted;
        public final int index;
        public final T chosen;

        private ChoiceResult(final List<T> sorted, final int index, final T chosen) {
            this.sorted = sorted;
            this.index = index;
            this.chosen = chosen;
        }

        public boolean isPass() {
            return index >= sorted.size();
        }
    }

    private ChoiceSpace() {}

    public static <T> T pickOne(final FCollectionView<T> options, final Random rng) {
        if (options == null || options.isEmpty()) {
            return null;
        }
        return options.get(rng.nextInt(options.size()));
    }

    public static <T> T pickOne(final List<T> options, final Random rng) {
        if (options == null || options.isEmpty()) {
            return null;
        }
        return options.get(rng.nextInt(options.size()));
    }

    public static int pickCount(final int min, final int max, final int available, final Random rng) {
        final int hi = Math.min(Math.max(max, 0), Math.max(available, 0));
        final int lo = Math.min(Math.max(min, 0), hi);
        return lo + (hi > lo ? rng.nextInt(hi - lo + 1) : 0);
    }

    public static int pickIntInRange(final int min, final int max, final Random rng) {
        if (max <= min) {
            return min;
        }
        final long span = (long) max - (long) min + 1L;
        if (span <= Integer.MAX_VALUE) {
            return min + rng.nextInt((int) span);
        }
        // Extremely wide ranges are rare; keep deterministic behavior without overflow.
        final long candidate = (long) min + rng.nextInt(Integer.MAX_VALUE);
        return (int) Math.min(candidate, (long) max);
    }

    public static CardCollection pickManyCards(
            final CardCollectionView options,
            final int min,
            final int max,
            final Random rng) {
        final CardCollection pool = new CardCollection(options == null ? new CardCollection() : options);
        final int count = pickCount(min, max, pool.size(), rng);
        final CardCollection out = new CardCollection();
        for (int i = 0; i < count && !pool.isEmpty(); i++) {
            out.add(pool.remove(rng.nextInt(pool.size())));
        }
        return out;
    }

    public static <T> List<T> shuffleCopy(final List<T> options, final Random rng) {
        final List<T> out = new ArrayList<>(options);
        Collections.shuffle(out, rng);
        return out;
    }

    public static boolean pickBool(final Random rng) {
        return rng.nextInt(2) == 1;
    }

    /** Pick an index in [0, size). Consumes RNG even when size == 1. */
    public static int pickIndex(final int size, final Random rng) {
        if (size <= 0) {
            return -1;
        }
        return rng.nextInt(size);
    }

    /** Pick an index in [0, size] where size means PASS. */
    public static int pickIndexWithPass(final int size, final Random rng) {
        if (size < 0) {
            return 0;
        }
        return rng.nextInt(size + 1);
    }

    /** Canonical parity pipeline: native list -> parity sort -> choose (optionally PASS). */
    public static <T> List<T> sortNative(
            final List<T> nativeOptions,
            final Comparator<T> parityComparator
    ) {
        final List<T> sorted = new ArrayList<>(nativeOptions);
        sorted.sort(parityComparator);
        return sorted;
    }

    /** Canonical parity pipeline: native list -> parity sort -> choose (optionally PASS). */
    public static <T> ChoiceResult<T> chooseFromNative(
            final List<T> nativeOptions,
            final Comparator<T> parityComparator,
            final boolean allowPass,
            final Random rng
    ) {
        final List<T> sorted = sortNative(nativeOptions, parityComparator);
        if (sorted.isEmpty()) {
            return new ChoiceResult<>(sorted, allowPass ? 0 : -1, null);
        }
        final int idx = allowPass ? pickIndexWithPass(sorted.size(), rng) : pickIndex(sorted.size(), rng);
        final T chosen = idx >= sorted.size() ? null : sorted.get(idx);
        return new ChoiceResult<>(sorted, idx, chosen);
    }

    /** Weighted choose where each action has `actionWeight` and PASS has weight 1. */
    public static int pickWeightedIndexWithPass(final int size, final int actionWeight, final Random rng) {
        final int safeSize = Math.max(0, size);
        final int safeWeight = Math.max(1, actionWeight);
        final int totalWeight = safeSize * safeWeight + 1;
        final int roll = rng.nextInt(totalWeight);
        if (roll >= safeSize * safeWeight) {
            return safeSize; // PASS
        }
        return roll / safeWeight;
    }
}
