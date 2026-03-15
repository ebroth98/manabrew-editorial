package forge.harness;

import java.util.Random;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * A {@link Random} subclass that counts every {@code nextInt()} call.
 *
 * Used for debugging RNG desync between the Java harness and the Rust engine:
 * both engines are seeded identically and must consume RNG in the same order,
 * so call counts should match at every decision boundary.
 */
public final class CountingRandom extends Random {
    private final AtomicInteger callCount = new AtomicInteger(0);

    public CountingRandom(long seed) {
        super(seed);
    }

    @Override
    public int nextInt(int bound) {
        int n = callCount.incrementAndGet();
        int result = super.nextInt(bound);
        if (Boolean.getBoolean("forge.parity.rng.trace")) {
            System.err.printf("[rng-java #%d] nextInt(%d) = %d%n", n, bound, result);
        }
        return result;
    }

    @Override
    public int nextInt() {
        int n = callCount.incrementAndGet();
        int result = super.nextInt();
        if (Boolean.getBoolean("forge.parity.rng.trace")) {
            System.err.printf("[rng-java #%d] nextInt() = %d%n", n, result);
        }
        return result;
    }

    /** Returns the total number of {@code nextInt} calls made so far. */
    public int getCallCount() {
        return callCount.get();
    }
}
