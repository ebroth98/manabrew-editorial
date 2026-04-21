package forge.harness;

import forge.LobbyPlayer;
import forge.game.Game;
import forge.game.player.IGameEntitiesFactory;
import forge.game.player.Player;
import forge.game.player.PlayerController;

public final class OpenMagicInteractiveLobbyPlayer extends LobbyPlayer implements IGameEntitiesFactory {
    private final OpenMagicInteractiveSession session;

    public OpenMagicInteractiveLobbyPlayer(final String name, final OpenMagicInteractiveSession session) {
        super(name);
        this.session = session;
    }

    @Override
    public Player createIngamePlayer(final Game game, final int id) {
        Player player = new Player(getName(), game, id);
        player.setFirstController(new OpenMagicInteractiveController(game, player, this, session));
        return player;
    }

    @Override
    public PlayerController createMindSlaveController(final Player master, final Player slave) {
        return new OpenMagicInteractiveController(slave.getGame(), slave, this, session);
    }

    @Override
    public void hear(final LobbyPlayer player, final String message) {
        // Headless adapter: chat is ignored.
    }
}
