package forge.harness;

import forge.game.GameEntity;
import forge.game.card.Card;
import forge.game.spellability.SpellAbility;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

/** Centralized canonical ordering for parity choice spaces. */
public final class ParityOrder {
    private ParityOrder() {}

    public static List<Card> sortCardsByNameThenId(final List<Card> cards) {
        final List<Card> out = new ArrayList<>(cards);
        out.sort(cardComparator());
        return out;
    }

    public static Comparator<Card> cardComparator() {
        return Comparator.comparing((Card c) -> c.getName())
                .thenComparingInt(ParityCardMap::parityId);
    }

    public static List<SpellAbility> sortActions(final List<SpellAbility> actions) {
        final List<SpellAbility> out = new ArrayList<>(actions);
        out.sort(actionComparator());
        return out;
    }

    public static Comparator<SpellAbility> actionComparator() {
        return Comparator.comparing(ParityOrder::actionSortKey);
    }

    private static String actionSortKey(final SpellAbility sa) {
        final String kind = sa.isLandAbility() ? "LAND"
                : (sa.isSpell() ? "SPELL"
                : (sa.isManaAbility() ? "MANA" : "AB"));
        final String host = sa.getHostCard() == null ? "" : sa.getHostCard().getName();
        final String text = sa.toUnsuppressedString() == null ? "" : sa.toUnsuppressedString();
        final int hostParity = ParityCardMap.parityId(sa.getHostCard());
        return kind + "|" + host + "|" + text + "|" + hostParity;
    }

    public static List<GameEntity> sortDefenders(final List<GameEntity> defenders) {
        final List<GameEntity> out = new ArrayList<>(defenders);
        out.sort(Comparator.comparing(ParityOrder::defenderKey));
        return out;
    }

    private static String defenderKey(final GameEntity e) {
        return e.getName() + "|" + e.getId();
    }
}
