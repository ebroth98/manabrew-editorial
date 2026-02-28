package forge.harness;

import com.google.common.eventbus.Subscribe;
import forge.deck.Deck;
import forge.game.*;
import forge.game.event.GameEventTurnPhase;
import forge.game.phase.PhaseType;
import forge.game.player.RegisteredPlayer;
import forge.gui.GuiBase;
import forge.model.FModel;

import java.io.File;
import java.util.*;

/**
 * Headless CLI entry point for the forge-harness parity testing tool.
 *
 * <pre>
 * java -jar forge-harness-jar-with-dependencies.jar \
 *   --deck1 red_burn --deck2 green_stompy --seed 42 --max-turns 10
 * </pre>
 *
 * Outputs JSONL to stdout: one StateSnapshot per line.
 * All diagnostic messages go to stderr.
 */
public final class Main {
    private Main() {}

    public static void main(String[] args) {
        // Parse CLI arguments
        String deck1Name = "red_burn";
        String deck2Name = "green_stompy";
        long seed = 42;
        int maxTurns = 10;
        String forgeHome = null;

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
                case "--forge-home":
                    if (i + 1 < args.length) forgeHome = args[++i];
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

        System.err.printf("[harness] Running: %s vs %s | seed=%d | max_turns=%d%n",
            deck1Name, deck2Name, seed, maxTurns);

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

        // Build decks
        Deck deck1 = PresetDecks.buildDeck(deck1Name);
        Deck deck2 = PresetDecks.buildDeck(deck2Name);

        if (deck1 == null) {
            System.err.println("[harness] Unknown deck: " + deck1Name +
                ". Available: " + Arrays.toString(PresetDecks.availablePresets()));
            System.exit(1);
        }
        if (deck2 == null) {
            System.err.println("[harness] Unknown deck: " + deck2Name +
                ". Available: " + Arrays.toString(PresetDecks.availablePresets()));
            System.exit(1);
        }

        System.err.printf("[harness] Deck 1: %s (%d cards)%n", deck1Name,
            deck1.getMain().countAll());
        System.err.printf("[harness] Deck 2: %s (%d cards)%n", deck2Name,
            deck2.getMain().countAll());

        // Set up game
        GameRules rules = new GameRules(GameType.Constructed);
        rules.setAppliedVariants(EnumSet.of(GameType.Constructed));

        // Set a timeout to avoid infinite loops
        rules.setSimTimeout(120);

        // Create a shared Random for agent decisions, seeded identically to
        // the Rust side's JavaRandom(seed). Both players share this instance
        // so RNG consumption order matches the Rust agents exactly.
        Random agentRng = new Random(seed);

        List<RegisteredPlayer> players = new ArrayList<>();

        RegisteredPlayer rp1 = new RegisteredPlayer(deck1);
        rp1.setPlayer(new DeterministicLobbyPlayer("Player1", agentRng));
        players.add(rp1);

        RegisteredPlayer rp2 = new RegisteredPlayer(deck2);
        rp2.setPlayer(new DeterministicLobbyPlayer("Player2", agentRng));
        players.add(rp2);

        Match match = new Match(rules, players, "ParityTest");
        Game game = match.createGame();

        // Set the RNG seed for reproducibility
        forge.util.MyRandom.setRandom(new Random(seed));

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

                String snap = SnapshotExtractor.snapshotJson(game);
                System.out.println(snap);
                System.out.flush();
                System.err.printf("[harness] Snapshot: turn=%d%n", currentTurn);
            }
        });

        System.err.println("[harness] Starting game...");

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

        // No final snapshot emitted — the game-over state is captured at
        // different timing between Rust (Cleanup) and Java (Untap), making
        // it non-comparable. Only turn-start snapshots from the subscriber
        // are used for parity comparison.

        // Summary to stderr
        if (game.getOutcome() != null && !game.getOutcome().isDraw()) {
            System.err.printf("[harness] Game over. Winner: %s%n",
                game.getOutcome().getWinningLobbyPlayer().getName());
        } else {
            System.err.println("[harness] Game ended in a draw.");
        }

        System.err.println("[harness] Done.");
        System.out.flush();
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
        System.err.println("  --forge-home <path>  Path to forge-gui/ assets directory");
        System.err.println("  --help               Show this help");
        System.err.println();
        System.err.println("Available decks: " + Arrays.toString(PresetDecks.availablePresets()));
    }
}
