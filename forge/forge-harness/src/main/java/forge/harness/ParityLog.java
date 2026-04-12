package forge.harness;

import java.util.ArrayList;
import java.util.List;
import java.util.Random;

public final class ParityLog {
    private static final ThreadLocal<List<String>> SINK = ThreadLocal.withInitial(() -> null);
    private static final ThreadLocal<Random> RNG_REF = ThreadLocal.withInitial(() -> null);

    private ParityLog() {}

    public static void enable(final Random rng) {
        SINK.set(new ArrayList<>());
        RNG_REF.set(rng);
    }

    public static void enable() {
        enable(null);
    }

    public static void disable() {
        SINK.remove();
        RNG_REF.remove();
    }

    public static int rngCallCount() {
        final Random rng = RNG_REF.get();
        if (rng instanceof CountingRandom cr) {
            return cr.getCallCount();
        }
        return -1;
    }

    public static void log(final String message) {
        final List<String> sink = SINK.get();
        if (sink != null) {
            sink.add(message);
        }
    }

    public static List<String> drain() {
        final List<String> sink = SINK.get();
        if (sink == null || sink.isEmpty()) {
            return List.of();
        }
        final List<String> result = new ArrayList<>(sink);
        sink.clear();
        return result;
    }
}
