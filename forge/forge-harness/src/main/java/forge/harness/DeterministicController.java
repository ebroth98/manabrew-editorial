package forge.harness;

import com.google.common.collect.Lists;
import forge.LobbyPlayer;
import forge.ai.AiPlayDecision;
import forge.ai.ComputerUtil;
import forge.ai.ComputerUtilCombat;
import forge.ai.ComputerUtilCost;
import forge.ai.PlayerControllerAi;
import forge.card.ColorSet;
import forge.card.MagicColor.Color;
import forge.game.*;
import forge.game.card.*;
import forge.game.combat.Combat;
import forge.game.combat.CombatUtil;
import forge.game.player.*;
import forge.game.spellability.*;
import forge.game.trigger.WrappedAbility;
import forge.game.zone.ZoneType;
import forge.util.collect.FCollectionView;

import java.util.*;

/**
 * A fully deterministic PlayerController for cross-engine parity testing.
 * <p>
 * Extends {@link PlayerControllerAi} to inherit the AI infrastructure
 * (AiController, AiCardMemory, cost payment, spell execution) while
 * overriding decision-making methods with deterministic logic that
 * mirrors the Rust {@code DeterministicAgent} exactly:
 * <ul>
 *   <li>Always keep opening hand (no mulligan)</li>
 *   <li>Play first playable card alphabetically, then pass</li>
 *   <li>Attack with all eligible creatures sorted by name</li>
 *   <li>Never block</li>
 *   <li>Target opponent for player targets; first alphabetical for card targets</li>
 *   <li>Discard first N alphabetically</li>
 *   <li>Always confirm actions</li>
 * </ul>
 */
public class DeterministicController extends PlayerControllerAi {

    public DeterministicController(Game game, Player p, LobbyPlayer lp) {
        super(game, p, lp);
    }

    // ── Mulligan ──────────────────────────────────────────────────────

    @Override
    public boolean mulliganKeepHand(Player firstPlayer, int cardsToReturn) {
        return true; // always keep
    }

    @Override
    public boolean confirmMulliganScry(Player p) {
        return false;
    }

    // ── Main Phase Action ─────────────────────────────────────────────

    @Override
    public List<SpellAbility> chooseSpellAbilityToPlay() {
        // Priority: play lands first, then spells — pick first alphabetically.
        // Uses the AI infrastructure (AiController.canPlaySa, ComputerUtilCost) for
        // proper spell evaluation, targeting, and mana checking.

        CardCollectionView hand = player.getCardsIn(ZoneType.Hand);

        // 1. Land plays — use getAllPossibleAbilities for properly configured land SAs
        List<SpellAbility> landPlays = new ArrayList<>();
        for (Card c : hand) {
            if (c.isLand() && player.canPlayLand(c, false, null)) {
                List<SpellAbility> abilities = c.getAllPossibleAbilities(player, true);
                for (SpellAbility sa : abilities) {
                    if (sa.isLandAbility()) {
                        landPlays.add(sa);
                    }
                }
            }
        }
        if (!landPlays.isEmpty()) {
            landPlays.sort(Comparator.comparing(sa -> sa.getHostCard().getName()));
            return Lists.newArrayList(landPlays.get(0));
        }

        // 2. Spell plays — use AiController.canPlaySa to set up targets as a side
        //    effect, but accept any spell that isn't mechanically uncastable.
        //    This removes the AI's strategic "willingness" filter so Java plays
        //    any castable spell — matching Rust's get_playable_cards() behavior.
        //    If canPlaySa doesn't set up targets (e.g. returns early for strategic
        //    reasons), we set them deterministically.
        List<SpellAbility> spellPlays = new ArrayList<>();
        for (Card c : hand) {
            if (!c.isLand()) {
                for (SpellAbility sa : c.getBasicSpells()) {
                    sa.setActivatingPlayer(player);
                    // canPlaySa sets up targets as a side effect
                    AiPlayDecision decision = getAi().canPlaySa(sa);
                    if (decision == AiPlayDecision.CantPlaySa
                            || decision == AiPlayDecision.TargetingFailed) {
                        continue;
                    }
                    if (!ComputerUtilCost.canPayCost(sa, player, false)) {
                        continue;
                    }
                    // ALWAYS set deterministic targets for targeted spells.
                    // canPlaySa() sets targets as a side effect, but the AI may
                    // choose strategically suboptimal targets (e.g. targeting a
                    // creature instead of the opponent). We override with
                    // deterministic targeting (opponent player first) to match
                    // Rust's DeterministicAgent behavior.
                    if (sa.usesTargeting()) {
                        setupDeterministicTargets(sa);
                        if (!sa.isTargetNumberValid()) {
                            continue; // no valid targets available
                        }
                    }
                    spellPlays.add(sa);
                }
            }
        }

        if (spellPlays.isEmpty()) {
            return null; // pass
        }

        // Deterministic: pick first alphabetically by card name
        spellPlays.sort(Comparator.comparing(sa -> sa.getHostCard().getName()));
        return Lists.newArrayList(spellPlays.get(0));
    }

    @Override
    public boolean playChosenSpellAbility(SpellAbility sa) {
        // For targeted spells, provide a chooseTargets callback to ensure targets
        // are properly set after moveToStack. This is a safety net: even if targets
        // were set in chooseSpellAbilityToPlay, they can be lost when the card is
        // copied for the stack zone.
        if (sa.usesTargeting()) {
            Runnable chooseTargets = () -> {
                if (!sa.isTargetNumberValid()) {
                    setupDeterministicTargets(sa);
                }
            };
            return ComputerUtil.handlePlayingSpellAbility(player, sa, getGame(), chooseTargets);
        }
        return super.playChosenSpellAbility(sa);
    }

    // ── Combat ────────────────────────────────────────────────────────

    @Override
    public void declareAttackers(Player attacker, Combat combat) {
        // Attack with all eligible creatures, sorted by name
        CardCollection creatures = new CardCollection(attacker.getCreaturesInPlay());
        creatures.sort(Comparator.comparing(Card::getName));

        // Find default defender (opponent)
        GameEntity defender = null;
        for (Player p : getGame().getPlayers()) {
            if (!p.equals(attacker)) {
                defender = p;
                break;
            }
        }
        if (defender == null) return;

        for (Card c : creatures) {
            if (CombatUtil.canAttack(c, defender)) {
                combat.addAttacker(c, defender);
            }
        }
    }

    @Override
    public void declareBlockers(Player defender, Combat combat) {
        // Never block — do nothing
    }

    @Override
    public CardCollection orderBlockers(Card attacker, CardCollection blockers) {
        blockers.sort(Comparator.comparing(Card::getName));
        return blockers;
    }

    @Override
    public CardCollection orderBlocker(Card attacker, Card blocker, CardCollection oldBlockers) {
        CardCollection all = new CardCollection(oldBlockers);
        all.add(blocker);
        all.sort(Comparator.comparing(Card::getName));
        return all;
    }

    @Override
    public CardCollection orderAttackers(Card blocker, CardCollection attackers) {
        attackers.sort(Comparator.comparing(Card::getName));
        return attackers;
    }

    @Override
    public Map<Card, Integer> assignCombatDamage(Card attacker, CardCollectionView blockers,
            CardCollectionView remaining, int damageDealt, GameEntity defender, boolean overrideOrder) {
        Map<Card, Integer> result = new LinkedHashMap<>();
        int damageLeft = damageDealt;
        for (Card blocker : blockers) {
            int lethal = ComputerUtilCombat.getEnoughDamageToKill(blocker, damageLeft, attacker, false, false);
            int assign = Math.min(lethal, damageLeft);
            result.put(blocker, assign);
            damageLeft -= assign;
            if (damageLeft <= 0) break;
        }
        if (damageLeft > 0 && !blockers.isEmpty()) {
            Card last = blockers.get(blockers.size() - 1);
            result.put(last, result.getOrDefault(last, 0) + damageLeft);
        }
        return result;
    }

    // ── Targeting & Choices ───────────────────────────────────────────

    @Override
    public <T extends GameEntity> T chooseSingleEntityForEffect(FCollectionView<T> optionList,
            DelayedReveal delayedReveal, SpellAbility sa, String title, boolean isOptional,
            Player relatedPlayer, Map<String, Object> params) {
        if (delayedReveal != null) reveal(delayedReveal);
        if (optionList == null || optionList.isEmpty()) return null;

        // Prefer targeting an opponent Player — matches Rust's choose_target_player
        // which explicitly picks the opponent over self.
        for (T e : optionList) {
            if (e instanceof Player && !e.equals(player)) {
                return e;
            }
        }

        // Fall back to alphabetical sort for non-player targets (cards, etc.)
        List<T> sorted = new ArrayList<>();
        for (T e : optionList) sorted.add(e);
        sorted.sort(Comparator.comparing(GameEntity::getName));
        return sorted.get(0);
    }

    @Override
    public CardCollectionView chooseCardsForEffect(CardCollectionView sourceList, SpellAbility sa,
            String title, int min, int max, boolean isOptional, Map<String, Object> params) {
        if (sourceList == null || sourceList.isEmpty()) return new CardCollection();
        CardCollection sorted = new CardCollection(sourceList);
        sorted.sort(Comparator.comparing(Card::getName));
        return new CardCollection(sorted.subList(0, Math.min(max, sorted.size())));
    }

    // ── Discard ───────────────────────────────────────────────────────

    @Override
    public CardCollection chooseCardsToDiscardFrom(Player playerDiscard, SpellAbility sa,
            CardCollection validCards, int min, int max) {
        return chooseFirstN(validCards, min);
    }

    @Override
    public CardCollection chooseCardsToDiscardToMaximumHandSize(int numDiscard) {
        CardCollection hand = new CardCollection(player.getCardsIn(ZoneType.Hand));
        hand.sort(Comparator.comparing(Card::getName));
        return new CardCollection(hand.subList(0, Math.min(numDiscard, hand.size())));
    }

    // ── Confirmations ─────────────────────────────────────────────────

    @Override
    public boolean confirmAction(SpellAbility sa, PlayerActionConfirmMode mode, String message,
            List<String> options, Card cardToShow, Map<String, Object> params) {
        return true;
    }

    @Override
    public boolean confirmTrigger(WrappedAbility sa) {
        return true;
    }

    // ── Numbers & Colors ──────────────────────────────────────────────

    @Override
    public byte chooseColor(String message, SpellAbility sa, ColorSet colors) {
        for (Color color : colors) return color.getColorMask();
        return Color.WHITE.getColorMask();
    }

    @Override
    public byte chooseColorAllowColorless(String message, Card card, ColorSet colors) {
        for (Color color : colors) return color.getColorMask();
        return Color.COLORLESS.getColorMask();
    }

    // ── Misc ──────────────────────────────────────────────────────────

    @Override
    public Player chooseStartingPlayer(boolean isFirstGame) {
        return player;
    }

    // ── Reveal (headless no-ops) ──────────────────────────────────────

    @Override
    public void reveal(CardCollectionView cards, ZoneType zone, Player owner,
            String messagePrefix, boolean addMsgSuffix) {
        // headless — no-op
    }

    @Override
    public void reveal(List<CardView> cards, ZoneType zone, PlayerView owner,
            String messagePrefix, boolean addMsgSuffix) {
        // headless — no-op
    }

    @Override
    public void notifyOfValue(SpellAbility saSource, GameObject relatedTarget, String value) {
        // headless — no-op
    }

    // ── Utility ───────────────────────────────────────────────────────

    private CardCollection chooseFirstN(CardCollectionView items, int amount) {
        if (items == null || items.isEmpty()) return new CardCollection();
        CardCollection sorted = new CardCollection(items);
        sorted.sort(Comparator.comparing(Card::getName));
        return new CardCollection(sorted.subList(0, Math.min(amount, sorted.size())));
    }

    /**
     * Set up deterministic targets for a spell ability.
     * Prefers targeting an opponent player (for burn spells etc.),
     * falls back to self, then to alphabetical card targets.
     */
    private void setupDeterministicTargets(SpellAbility sa) {
        sa.resetTargets();
        // Prefer opponent player
        for (Player p : getGame().getPlayers()) {
            if (!p.equals(player) && sa.canTarget(p)) {
                sa.getTargets().add(p);
                if (sa.isTargetNumberValid()) return;
            }
        }
        // Fall back to self
        if (!sa.isTargetNumberValid() && sa.canTarget(player)) {
            sa.getTargets().add(player);
            if (sa.isTargetNumberValid()) return;
        }
        // Fall back to cards on the battlefield (sorted alphabetically)
        if (!sa.isTargetNumberValid()) {
            CardCollection allCards = new CardCollection();
            for (Player p : getGame().getPlayers()) {
                allCards.addAll(p.getCardsIn(ZoneType.Battlefield));
            }
            allCards.sort(Comparator.comparing(Card::getName));
            for (Card c : allCards) {
                if (sa.canTarget(c)) {
                    sa.getTargets().add(c);
                    if (sa.isTargetNumberValid()) return;
                }
            }
        }
    }
}
