package forge.harness;

import forge.game.Game;
import forge.game.card.Card;
import forge.game.player.Player;
import forge.game.spellability.SpellAbility;
import forge.game.zone.ZoneType;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Function;

/** Native-engine card-id to cross-engine parity-id mapping. */
public final class ParityCardMap {
    private static final Map<Integer, Integer> CARD_TO_PARITY = new HashMap<>();
    private static boolean initialized = false;

    private ParityCardMap() {}

    public static synchronized void reset() {
        CARD_TO_PARITY.clear();
        initialized = false;
    }

    public static synchronized void initializeFromOpeningState(final Game game) {
        if (initialized || game == null) {
            return;
        }
        int next = 1;
        final List<Player> players = new ArrayList<>(game.getRegisteredPlayers());
        players.sort(Comparator.comparingInt(Player::getId));

        for (final Player p : players) {
            for (final Card c : p.getCardsIn(ZoneType.Hand)) {
                if (!CARD_TO_PARITY.containsKey(c.getId())) {
                    CARD_TO_PARITY.put(c.getId(), next++);
                }
            }
            for (final Card c : p.getCardsIn(ZoneType.Library)) {
                if (!CARD_TO_PARITY.containsKey(c.getId())) {
                    CARD_TO_PARITY.put(c.getId(), next++);
                }
            }
        }
        initialized = true;
    }

    public static int parityId(final Card c) {
        if (c == null) {
            return Integer.MAX_VALUE;
        }
        return CARD_TO_PARITY.getOrDefault(c.getId(), Integer.MAX_VALUE - c.getId());
    }

    public static List<String> disambiguateCards(
            final List<Card> cards,
            final Function<Card, String> baseLabel
    ) {
        return appendKey(cards, baseLabel, c -> String.valueOf(parityId(c)));
    }

    public static List<String> disambiguateAbilities(
            final List<SpellAbility> abilities,
            final Function<SpellAbility, String> baseLabel
    ) {
        return appendKey(abilities, baseLabel, sa -> String.valueOf(parityId(sa.getHostCard())));
    }

    private static <T> List<String> appendKey(
            final List<T> items,
            final Function<T, String> baseLabel,
            final Function<T, String> key
    ) {
        final List<String> out = new ArrayList<>(items.size());
        for (int i = 0; i < items.size(); i++) {
            final String base = baseLabel.apply(items.get(i));
            out.add(base + "@" + key.apply(items.get(i)));
        }
        return out;
    }
}
