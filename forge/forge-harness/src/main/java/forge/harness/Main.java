package forge.harness;

import com.google.common.eventbus.Subscribe;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import forge.deck.Deck;
import forge.game.*;
import forge.game.event.GameEventTurnPhase;
import forge.game.phase.PhaseType;
import forge.game.player.RegisteredPlayer;
import forge.gui.GuiBase;
import forge.model.FModel;

import java.io.*;
import java.util.*;

/**
 * Headless CLI entry point for the forge-harness parity testing tool.
 *
 * <p><b>One-shot mode</b> (default):
 * <pre>
 * java -jar forge-harness-jar-with-dependencies.jar \
 *   --deck1 red_burn --deck2 green_stompy --seed 42 --max-turns 10
 * </pre>
 *
 * <p><b>Server mode</b> ({@code --server}):
 * Initializes FModel once, then reads JSONL requests from stdin and writes
 * snapshot responses to stdout. Avoids repeated JVM startup cost.
 * <pre>
 * java -jar forge-harness-jar-with-dependencies.jar --server
 * </pre>
 *
 * Outputs JSONL to stdout: one StateSnapshot per line.
 * All diagnostic messages go to stderr.
 */
public final class Main {
    private Main() {}

    /** Dedicated protocol output stream, safe from Forge's stray System.out calls. */
    private static PrintStream protocolOut;

    public static void main(String[] args) {
        // Parse CLI arguments
        String deck1Name = "red_burn";
        String deck2Name = "green_stompy";
        long seed = 42;
        int maxTurns = 10;
        boolean preferActions = false;
        String forgeHome = null;
        boolean serverMode = false;

        for (int i = 0; i < args.length; i++) {
            switch (args[i]) {
                case "--deck1":
                    if (i + 1 < args.length) deck1Name = args[++i];
                    break;
                case "--deck2":
                    if (i + 1 < args.length) deck2Name = args[++i];
                    break;
                case "--seed":
                    if (i + 1 < args.length) seed = Long.parseLong(args[++i]);
                    break;
                case "--max-turns":
                    if (i + 1 < args.length) maxTurns = Integer.parseInt(args[++i]);
                    break;
                case "--prefer-actions":
                    preferActions = true;
                    break;
                case "--forge-home":
                    if (i + 1 < args.length) forgeHome = args[++i];
                    break;
                case "--server":
                    serverMode = true;
                    break;
                case "--help":
                    printUsage();
                    return;
                default:
                    System.err.println("[harness] Unknown argument: " + args[i]);
                    break;
            }
        }

        // Resolve forge-gui assets directory
        String assetsDir = resolveAssetsDir(forgeHome);
        System.err.println("[harness] Assets dir: " + assetsDir);

        // In server mode, capture real stdout and redirect System.out BEFORE
        // FModel.initialize() to prevent Forge's stray println() calls from
        // leaking into the protocol stream during initialization.
        if (serverMode) {
            protocolOut = System.out;
            System.setOut(new PrintStream(new OutputStream() {
                @Override public void write(int b) { /* discard */ }
                @Override public void write(byte[] b, int off, int len) { /* discard */ }
            }));
        }

        // Initialize Forge headless — must set GuiBase before FModel.initialize
        System.err.println("[harness] Initializing Forge...");
        GuiBase.setInterface(new HeadlessGuiBase(assetsDir));
        try {
            FModel.initialize(null, null);
        } catch (Exception e) {
            System.err.println("[harness] Failed to initialize Forge: " + e.getMessage());
            e.printStackTrace(System.err);
            System.exit(1);
        }

        if (serverMode) {
            runServerMode();
        } else {
            runOneShot(deck1Name, deck2Name, seed, maxTurns, preferActions);
        }
    }

    /**
     * Original one-shot mode: run a single game and exit.
     */
    private static void runOneShot(
        String deck1Name,
        String deck2Name,
        long seed,
        int maxTurns,
        boolean preferActions
    ) {
        // In one-shot mode, protocol output goes to real System.out
        protocolOut = System.out;

        System.err.printf("[harness] Running: %s vs %s | seed=%d | max_turns=%d%n",
            deck1Name, deck2Name, seed, maxTurns);

        runGame(deck1Name, deck2Name, seed, maxTurns, preferActions);

        System.err.println("[harness] Done.");
        protocolOut.flush();
    }

    /**
     * Server mode: read JSONL requests from stdin, run games, write responses.
     * FModel is already initialized — we reuse it across all requests.
     */
    private static void runServerMode() {
        // protocolOut and System.out redirect already set up in main() before FModel.initialize()
        System.err.println("[harness] Server mode ready. Waiting for requests on stdin...");

        BufferedReader stdin = new BufferedReader(new InputStreamReader(System.in));
        String line;

        try {
            while ((line = stdin.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty()) continue;

                JsonObject request;
                try {
                    request = JsonParser.parseString(line).getAsJsonObject();
                } catch (Exception e) {
                    System.err.println("[harness] Failed to parse request: " + e.getMessage());
                    protocolOut.println("{\"done\":true,\"error\":\"Invalid JSON: " +
                        escapeJson(e.getMessage()) + "\"}");
                    protocolOut.flush();
                    continue;
                }

                String command = request.has("command") ? request.get("command").getAsString() : "run";

                if ("quit".equals(command)) {
                    System.err.println("[harness] Received quit command. Shutting down.");
                    break;
                }

                if (!"run".equals(command)) {
                    System.err.println("[harness] Unknown command: " + command);
                    protocolOut.println("{\"done\":true,\"error\":\"Unknown command: " +
                        escapeJson(command) + "\"}");
                    protocolOut.flush();
                    continue;
                }

                // Extract parameters
                String deck1 = request.has("deck1") ? request.get("deck1").getAsString() : "red_burn";
                String deck2 = request.has("deck2") ? request.get("deck2").getAsString() : "green_stompy";
                long gameSeed = request.has("seed") ? request.get("seed").getAsLong() : 42;
                int gameMaxTurns = request.has("max_turns") ? request.get("max_turns").getAsInt() : 10;
                boolean gamePreferActions = request.has("prefer_actions") && request.get("prefer_actions").getAsBoolean();

                System.err.printf("[harness] Request: %s vs %s | seed=%d | max_turns=%d%n",
                    deck1, deck2, gameSeed, gameMaxTurns);

                try {
                    runGame(deck1, deck2, gameSeed, gameMaxTurns, gamePreferActions);
                    protocolOut.println("{\"done\":true,\"error\":null}");
                } catch (Exception e) {
                    System.err.println("[harness] Game error: " + e.getMessage());
                    e.printStackTrace(System.err);
                    protocolOut.println("{\"done\":true,\"error\":\"" +
                        escapeJson(e.getMessage()) + "\"}");
                }
                protocolOut.flush();
                // Reclaim memory between games — clear image caches and hint GC.
                forge.ImageKeys.clearCaches();
                System.gc();
            }
        } catch (IOException e) {
            System.err.println("[harness] Stdin read error: " + e.getMessage());
        }

        System.err.println("[harness] Server exiting.");
    }

    /**
     * Run a single game with the given parameters.
     * Snapshots are written to {@link #protocolOut} (not System.out).
     */
    private static void runGame(
        String deck1Name,
        String deck2Name,
        long seed,
        int maxTurns,
        boolean preferActions
    ) {
        // Build decks
        Deck deck1 = PresetDecks.buildDeck(deck1Name);
        Deck deck2 = PresetDecks.buildDeck(deck2Name);

        if (deck1 == null) {
            throw new IllegalArgumentException("Unknown deck: " + deck1Name +
                ". Available: " + Arrays.toString(PresetDecks.availablePresets()));
        }
        if (deck2 == null) {
            throw new IllegalArgumentException("Unknown deck: " + deck2Name +
                ". Available: " + Arrays.toString(PresetDecks.availablePresets()));
        }

        System.err.printf("[harness] Deck 1: %s (%d cards)%n", deck1Name,
            deck1.getMain().countAll());
        System.err.printf("[harness] Deck 2: %s (%d cards)%n", deck2Name,
            deck2.getMain().countAll());

        // Set up game
        GameRules rules = new GameRules(GameType.Constructed);
        rules.setAppliedVariants(EnumSet.of(GameType.Constructed));
        rules.setSimTimeout(120);

        // Reset RNG for this game — fresh seed each time for reproducibility
        forge.util.MyRandom.setRandom(new Random(seed));

        // Create a shared Random for agent decisions, seeded identically to
        // the Rust side's JavaRandom(seed). Both players share this instance
        // so RNG consumption order matches the Rust agents exactly.
        CountingRandom agentRng = new CountingRandom(seed);

        List<RegisteredPlayer> players = new ArrayList<>();

        RegisteredPlayer rp1 = new RegisteredPlayer(deck1);
        rp1.setPlayer(new DeterministicLobbyPlayer("Player1", agentRng, preferActions));
        players.add(rp1);

        RegisteredPlayer rp2 = new RegisteredPlayer(deck2);
        rp2.setPlayer(new DeterministicLobbyPlayer("Player2", agentRng, preferActions));
        players.add(rp2);

        Match match = new Match(rules, players, "ParityTest");
        Game game = match.createGame();
        ParityCardMap.reset();

        // Register turn snapshot subscriber — emits JSONL snapshots at each turn boundary.
        // Uses GameEventTurnPhase(UNTAP) instead of GameEventTurnBegan because:
        //   - GameEventTurnBegan fires BEFORE setSickness(false) and incrementTurn()
        //   - GameEventTurnPhase(UNTAP) fires AFTER both, but BEFORE actual untapping
        // This matches Rust's snapshot timing (after new_turn_for_player resets).
        // Enforce max-turns limit: Rust's loop is `while turn_number <= max_turns`,
        // so it runs turns 1..=max_turns. When turn max_turns+1 begins, Rust exits
        // WITHOUT calling run_turn (no snapshot). We match by stopping the game
        // BEFORE emitting a snapshot for the turn beyond the limit.
        final int turnLimit = maxTurns;
        game.subscribeToEvents(new Object() {
            @Subscribe
            public void onTurnPhase(GameEventTurnPhase event) {
                if (event.phase() != PhaseType.UNTAP) return;
                int currentTurn = game.getPhaseHandler().getTurn();

                // Stop before emitting if we've exceeded the turn limit
                if (currentTurn > turnLimit) {
                    System.err.printf("[harness] Turn limit reached (%d > %d), ending game%n",
                        currentTurn, turnLimit);
                    game.setGameOver(GameEndReason.Draw);
                    return;
                }
                if (currentTurn == 1) {
                    ParityCardMap.initializeFromOpeningState(game);
                }

                String snap = SnapshotExtractor.snapshotJson(game);
                protocolOut.println(snap);
                protocolOut.flush();
                System.err.printf("[harness] Snapshot: turn=%d%n", currentTurn);
            }
        });

        System.err.println("[harness] Starting game...");
        DecisionLog.setSink(line -> {
            protocolOut.println(line);
            protocolOut.flush();
        });

        // Run the game synchronously
        try {
            match.startGame(game);
        } catch (Exception e) {
            System.err.println("[harness] Game error: " + e.getMessage());
            e.printStackTrace(System.err);
        } finally {
            if (!game.isGameOver()) {
                game.setGameOver(GameEndReason.Draw);
            }
        }

        // Summary to stderr
        if (game.getOutcome() != null && !game.getOutcome().isDraw()) {
            System.err.printf("[harness] Game over. Winner: %s%n",
                game.getOutcome().getWinningLobbyPlayer().getName());
        } else {
            System.err.println("[harness] Game ended in a draw.");
        }
        DecisionLog.setSink(null);
    }

    /** Escape a string for embedding in a JSON string value. */
    private static String escapeJson(String s) {
        if (s == null) return "null";
        return s.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }

    /**
     * Resolve the assets directory (forge-gui/) from --forge-home or by auto-detecting.
     * The assets dir must contain res/cardsfolder/.
     */
    private static String resolveAssetsDir(String forgeHome) {
        // If --forge-home is provided, use it
        if (forgeHome != null) {
            String dir = forgeHome.endsWith(File.separator) ? forgeHome : forgeHome + File.separator;
            return dir;
        }

        // Try to auto-detect: look for forge-gui/ relative to the JAR or CWD
        // Common layouts:
        //   forge/forge-harness/target/ -> forge/forge-gui/
        //   forge/ -> forge/forge-gui/
        //   ./ -> forge-gui/ or ../forge-gui/
        String[] candidates = {
            "forge-gui" + File.separator,
            "../forge-gui" + File.separator,
            "../../forge-gui" + File.separator,
            "../../../forge-gui" + File.separator,
        };

        for (String candidate : candidates) {
            File resDir = new File(candidate + "res" + File.separator + "cardsfolder");
            if (resDir.isDirectory()) {
                return candidate;
            }
        }

        // Fallback: use CWD (same as GuiDesktop default for non-git builds)
        System.err.println("[harness] WARNING: Could not auto-detect assets dir. Use --forge-home <path-to-forge-gui/>");
        return "";
    }

    private static void printUsage() {
        System.err.println("Usage: forge-harness [OPTIONS]");
        System.err.println();
        System.err.println("Options:");
        System.err.println("  --deck1 <name>       Preset deck for player 1 (default: red_burn)");
        System.err.println("  --deck2 <name>       Preset deck for player 2 (default: green_stompy)");
        System.err.println("  --seed <number>      RNG seed (default: 42)");
        System.err.println("  --max-turns <n>      Maximum turns (default: 10)");
        System.err.println("  --prefer-actions     Bias random choices toward acting instead of passing");
        System.err.println("  --forge-home <path>  Path to forge-gui/ assets directory");
        System.err.println("  --server             Run in server mode (stdin/stdout JSONL protocol)");
        System.err.println("  --help               Show this help");
        System.err.println();
        System.err.println("Available decks: " + Arrays.toString(PresetDecks.availablePresets()));
    }
}
