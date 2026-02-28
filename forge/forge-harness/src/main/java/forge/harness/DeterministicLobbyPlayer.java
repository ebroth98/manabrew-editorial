package forge.harness;

import forge.LobbyPlayer;
import forge.game.Game;
import forge.game.player.IGameEntitiesFactory;
import forge.game.player.Player;
import forge.game.player.PlayerController;

import java.util.Random;

/**
 * A LobbyPlayer that creates {@link DeterministicController} instances
 * for cross-engine parity testing. Both controllers share the same
 * {@link Random} instance so that decision RNG consumption is identical
 * to the Rust side.
 */
public class DeterministicLobbyPlayer extends LobbyPlayer implements IGameEntitiesFactory {

    /** Shared RNG for agent decisions — same instance across both players. */
    private final Random rng;

    public DeterministicLobbyPlayer(String name, Random rng) {
        super(name);
        this.rng = rng;
    }

    @Override
    public Player createIngamePlayer(Game game, int id) {
        Player p = new Player(getName(), game, id);
        p.setFirstController(new DeterministicController(game, p, this, rng));
        return p;
    }

    @Override
    public PlayerController createMindSlaveController(Player master, Player slave) {
        return new DeterministicController(slave.getGame(), slave, this, rng);
    }

    @Override
    public void hear(LobbyPlayer player, String message) {
        // Headless — ignore all messages.
    }
}
