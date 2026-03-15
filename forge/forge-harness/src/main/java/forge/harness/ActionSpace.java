package forge.harness;

import forge.ai.ComputerUtilMana;
import forge.game.Game;
import forge.game.card.Card;
import forge.game.card.CardCollectionView;
import forge.game.player.Player;
import forge.game.spellability.SpellAbility;
import forge.game.spellability.TargetRestrictions;
import forge.game.zone.ZoneType;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Harness-side action-space enumeration.
 *
 * Uses engine legality directly via {@code Card#getAllPossibleAbilities(player, true)}.
 */
public final class ActionSpace {
    private ActionSpace() {}

    private static String actionBaseLabel(final SpellAbility sa) {
        final String kind = sa.isLandAbility() ? "LAND"
                : (sa.isSpell() ? "SPELL" : (sa.isManaAbility() ? "MANA" : "AB"));
        final String fbTag = sa.isFlashback() ? "[FB]" : "";
        return kind + ":" + sa.getHostCard().getName() + fbTag;
    }

    /**
     * Build deterministic logged labels for main-action options.
     * <p>
     * If a single host card presents multiple spell abilities with the same base
     * label (e.g. normal and alternative cost casts), suffix them as `$1`, `$2`, ...
     * For single-variant cards, omit the suffix.
     */
    public static List<String> buildMainActionLabels(final List<SpellAbility> actions) {
        final List<String> baseLabels = new ArrayList<>(actions.size());
        for (final SpellAbility sa : actions) {
            baseLabels.add(actionBaseLabel(sa));
        }

        final Map<String, Integer> totals = new HashMap<>();
        for (int i = 0; i < actions.size(); i++) {
            final SpellAbility sa = actions.get(i);
            final String key = baseLabels.get(i) + "|" + ParityCardMap.parityId(sa.getHostCard());
            totals.put(key, totals.getOrDefault(key, 0) + 1);
        }

        final Map<String, Integer> seen = new HashMap<>();
        final List<String> out = new ArrayList<>(actions.size());
        for (int i = 0; i < actions.size(); i++) {
            final SpellAbility sa = actions.get(i);
            final String base = baseLabels.get(i);
            final String key = base + "|" + ParityCardMap.parityId(sa.getHostCard());
            final int total = totals.getOrDefault(key, 1);
            final String withCostSuffix;
            if (total <= 1) {
                withCostSuffix = base;
            } else {
                final int idx = seen.getOrDefault(key, 0) + 1;
                seen.put(key, idx);
                withCostSuffix = base + "$" + idx;
            }
            out.add(withCostSuffix + "@" + ParityCardMap.parityId(sa.getHostCard()));
        }
        return out;
    }

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
            if (p != player) {
                p.getZone(ZoneType.Graveyard)
                        .getCardsPlayerCanActivate(player)
                        .forEach(candidates::add);
            }
        }

        final List<SpellAbility> actions = new ArrayList<>();
        for (final Card c : candidates) {
            for (final SpellAbility sa : c.getAllPossibleAbilities(player, true)) {
                sa.setActivatingPlayer(player);
                final boolean canPlay = sa.canPlay(true);
                final boolean hasManaCost = sa.getPayCosts() != null && sa.getPayCosts().hasManaCost();
                final boolean canPayMana = !hasManaCost || ComputerUtilMana.canPayManaCost(sa, player, 0, false);
                final boolean validTargets = hasValidTargets(sa);
                if (!canPlay) {
                    continue;
                }
                // SpellAbility.canPlay() uses Cost.canPay(), and CostPartMana.canPay()
                // is permissive in engine core. Add an explicit mana-feasibility check.
                if (!canPayMana) {
                    continue;
                }
                // Keep action space focused on actionable plays/abilities.
                // Mana abilities are excluded to avoid redundant/noisy parity choices.
                if (sa.isManaAbility()) {
                    continue;
                }
                if (!validTargets) {
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
                // Avoid stale target selections mutating candidate counts across repeated action-space scans.
                current.resetTargets();
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
