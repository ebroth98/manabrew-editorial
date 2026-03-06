package forge.harness;

import forge.ai.ComputerUtilCost;
import forge.game.Game;
import forge.game.card.Card;
import forge.game.card.CardCollectionView;
import forge.game.player.Player;
import forge.game.spellability.SpellAbility;
import forge.game.spellability.TargetRestrictions;
import forge.game.zone.ZoneType;

import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

/**
 * Harness-side action-space enumeration.
 *
 * Uses engine legality directly via {@code Card#getAllPossibleAbilities(player, true)}.
 */
public final class ActionSpace {
    private ActionSpace() {}

    public static List<SpellAbility> getPossibleActions(final Player player) {
        final Game game = player.getGame();
        final Set<Card> candidates = new LinkedHashSet<>();

        candidates.addAll(player.getCardsIn(ZoneType.Hand));
        candidates.addAll(player.getCardsIn(ZoneType.Battlefield));
        candidates.addAll(player.getCardsIn(ZoneType.Graveyard));
        candidates.addAll(player.getCardsIn(ZoneType.Exile));
        candidates.addAll(player.getCardsIn(ZoneType.Command));

        candidates.addAll(game.getCardsIn(ZoneType.Battlefield));
        candidates.addAll(game.getCardsIn(ZoneType.Exile));
        candidates.addAll(game.getCardsIn(ZoneType.Command));

        for (final Player p : game.getPlayers()) {
            final CardCollectionView lib = p.getCardsIn(ZoneType.Library);
            if (!lib.isEmpty()) {
                candidates.add(lib.get(0));
            }
            candidates.addAll(p.getCardsIn(ZoneType.Graveyard));
        }

        final List<SpellAbility> actions = new ArrayList<>();
        for (final Card c : candidates) {
            for (final SpellAbility sa : c.getAllPossibleAbilities(player, true)) {
                sa.setActivatingPlayer(player);
                if (!sa.canPlay(true)) {
                    continue;
                }
                // Keep action space focused on actionable plays/abilities.
                // Mana abilities are excluded to avoid redundant/noisy parity choices.
                if (sa.isManaAbility()) {
                    continue;
                }
                if (!ComputerUtilCost.canPayCost(sa, player, false)) {
                    continue;
                }
                if (!hasValidTargets(sa)) {
                    continue;
                }
                actions.add(sa);
            }
        }
        return actions;
    }

    private static boolean hasValidTargets(final SpellAbility sa) {
        SpellAbility current = sa;
        while (current != null) {
            if (current.usesTargeting()) {
                final TargetRestrictions tr = current.getTargetRestrictions();
                if (tr == null) {
                    return false;
                }
                final int minTargets = current.getMinTargets();
                if (minTargets > 0 && tr.getNumCandidates(current, true) < minTargets) {
                    return false;
                }
            }
            current = current.getSubAbility();
        }
        return true;
    }
}
