package forge.harness;

import forge.LobbyPlayer;
import forge.game.Game;
import forge.game.player.IGameEntitiesFactory;
import forge.game.player.Player;
import forge.game.player.PlayerController;

/**
 * A LobbyPlayer that creates {@link DeterministicController} instances
 * for cross-engine parity testing.
 */
public class DeterministicLobbyPlayer extends LobbyPlayer implements IGameEntitiesFactory {

    public DeterministicLobbyPlayer(String name) {
        super(name);
    }

    @Override
    public Player createIngamePlayer(Game game, int id) {
        Player p = new Player(getName(), game, id);
        p.setFirstController(new DeterministicController(game, p, this));
        return p;
    }

    @Override
    public PlayerController createMindSlaveController(Player master, Player slave) {
        return new DeterministicController(slave.getGame(), slave, this);
    }

    @Override
    public void hear(LobbyPlayer player, String message) {
        // Headless — ignore all messages.
    }
}
