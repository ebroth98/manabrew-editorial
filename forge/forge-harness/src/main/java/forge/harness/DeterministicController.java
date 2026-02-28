package forge.harness;

import com.google.common.collect.Lists;
import forge.LobbyPlayer;
import forge.ai.AiCostDecision;
import forge.ai.AiPlayDecision;
import forge.ai.ComputerUtil;
import forge.ai.ComputerUtilCombat;
import forge.ai.ComputerUtilCost;
import forge.ai.PlayerControllerAi;
import forge.game.cost.Cost;
import forge.game.cost.CostPayment;
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

import org.apache.commons.lang3.tuple.ImmutablePair;

import java.util.*;

/**
 * A hybrid deterministic PlayerController for cross-engine parity testing.
 * <p>
 * Uses RNG for 4 core decisions (play choice, attackers, blockers, targeting)
 * and fixed values for everything else. This avoids RNG desync caused by
 * Java and Rust calling non-core callbacks at different times.
 * <p>
 * Both sides share a {@code java.util.Random} / {@code JavaRandom} seeded
 * identically. Core decisions sort options alphabetically then use
 * {@code rng.nextInt()} to pick, consuming the RNG in the same order.
 */
public class DeterministicController extends PlayerControllerAi {

    /** Shared RNG for all decisions — same instance used by both players. */
    private final Random rng;

    public DeterministicController(Game game, Player p, LobbyPlayer lp, Random rng) {
        super(game, p, lp);
        this.rng = rng;
    }

    // ── Mulligan ──────────────────────────────────────────────────────

    @Override
    public boolean mulliganKeepHand(Player firstPlayer, int cardsToReturn) {
        return true; // always keep — no RNG consumed
    }

    @Override
    public boolean confirmMulliganScry(Player p) {
        return false;
    }

    // ── Main Phase Action ─────────────────────────────────────────────

    @Override
    public List<SpellAbility> chooseSpellAbilityToPlay() {
        // Only consume RNG during sorcery-speed windows (our main phase, stack empty).
        // Matches Rust's choose_action which skips non-sorcery-speed priority.
        // Java calls this method for ALL priority windows, including instant-speed
        // during combat/opponent's turn, but those must not consume our shared RNG.
        forge.game.phase.PhaseHandler ph = getGame().getPhaseHandler();
        boolean isSorcerySpeed = ph.isPlayerTurn(player)
            && (ph.is(forge.game.phase.PhaseType.MAIN1) || ph.is(forge.game.phase.PhaseType.MAIN2))
            && getGame().getStack().isEmpty();
        if (!isSorcerySpeed) {
            return null; // pass priority — no RNG consumed
        }

        // Build combined list: lands first (sorted), then spells (sorted).
        // Pick randomly: idx 0..count plays a card, idx count = pass.
        // RNG consumption: exactly 1 call to rng.nextInt(count + 1) if any
        // playable cards exist, 0 calls if nothing playable.

        // Only play during main phases — matches Rust's DeterministicAgent which
        // only plays during Main1/Main2. Without this check, the Java engine would
        // allow sorcery-speed spells during any priority window (upkeep, combat, etc.)
        // because it trusts the controller to enforce timing.
        if (!player.canCastSorcery()) {
            return null; // pass during non-main phases
        }

        // Only play during main phases — matches Rust's DeterministicAgent which
        // only plays during Main1/Main2. Without this check, the Java engine would
        // allow sorcery-speed spells during any priority window (upkeep, combat, etc.)
        // because it trusts the controller to enforce timing.
        if (!player.canCastSorcery()) {
            return null; // pass during non-main phases
        }

        CardCollectionView hand = player.getCardsIn(ZoneType.Hand);

        // 1. Land plays
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
        landPlays.sort(Comparator.comparing(sa -> sa.getHostCard().getName()));

        // 2. Spell plays
        List<SpellAbility> spellPlays = new ArrayList<>();
        for (Card c : hand) {
            if (!c.isLand()) {
                for (SpellAbility sa : c.getBasicSpells()) {
                    sa.setActivatingPlayer(player);
                    // Only check cost + targeting — no AI evaluation.
                    // Matches Rust's get_playable_cards which checks
                    // can_pay + has_candidates_in_chain, not AI heuristics.
                    if (!ComputerUtilCost.canPayCost(sa, player, false)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !hasDeterministicTargets(sa)) {
                        continue;
                    }
                    spellPlays.add(sa);
                }
            }
        }
        spellPlays.sort(Comparator.comparing(sa -> sa.getHostCard().getName()));

        // Combined: lands first, then spells
        List<SpellAbility> all = new ArrayList<>();
        all.addAll(landPlays);
        all.addAll(spellPlays);

        if (all.isEmpty()) {
            return null; // pass — no RNG consumed
        }

        // Random pick: 0..count plays, count = pass
        int idx = rng.nextInt(all.size() + 1);
        if (idx >= all.size()) {
            return null; // pass
        }

        return Lists.newArrayList(all.get(idx));
    }

    @Override
    public boolean playChosenSpellAbility(SpellAbility sa) {
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
        // Sort eligible creatures alphabetically, per-creature coin flip.
        // RNG consumption: exactly 1 call per eligible creature.
        CardCollection creatures = new CardCollection(attacker.getCreaturesInPlay());
        creatures.sort(Comparator.comparing(Card::getName));

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
                if (rng.nextInt(2) == 1) {
                    combat.addAttacker(c, defender);
                }
            }
        }
    }

    @Override
    public void declareBlockers(Player defender, Combat combat) {
        // Sort eligible blockers alphabetically.
        // For each: rng.nextInt(attacker_count + 1). 0 = don't block, k = block attacker[k-1].
        CardCollection blockers = new CardCollection(defender.getCreaturesInPlay());
        blockers.sort(Comparator.comparing(Card::getName));

        // Get sorted attackers
        List<Card> attackers = new ArrayList<>(combat.getAttackers());
        attackers.sort(Comparator.comparing(Card::getName));

        if (attackers.isEmpty()) return;

        for (Card blocker : blockers) {
            if (!CombatUtil.canBlock(blocker, combat)) continue;
            int choice = rng.nextInt(attackers.size() + 1);
            if (choice > 0 && choice <= attackers.size()) {
                Card attackerToBlock = attackers.get(choice - 1);
                if (CombatUtil.canBlock(attackerToBlock, blocker, combat)) {
                    combat.addBlocker(attackerToBlock, blocker);
                }
            }
        }
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

        // Sort: players first (by name), then non-players (by name)
        List<T> sorted = new ArrayList<>();
        for (T e : optionList) sorted.add(e);
        sorted.sort((a, b) -> {
            boolean aPlayer = a instanceof Player;
            boolean bPlayer = b instanceof Player;
            if (aPlayer != bPlayer) return aPlayer ? -1 : 1;
            return a.getName().compareTo(b.getName());
        });

        return sorted.get(0); // fixed: always pick first (no RNG consumed)
    }

    @Override
    public CardCollectionView chooseCardsForEffect(CardCollectionView sourceList, SpellAbility sa,
            String title, int min, int max, boolean isOptional, Map<String, Object> params) {
        if (sourceList == null || sourceList.isEmpty()) return new CardCollection();
        CardCollection sorted = new CardCollection(sourceList);
        sorted.sort(Comparator.comparing(Card::getName));

        int count = Math.min(max, sorted.size());
        // Fixed: return first `count` cards sorted alphabetically (no RNG consumed)
        return new CardCollection(sorted.subList(0, count));
    }

    // ── Discard ───────────────────────────────────────────────────────

    @Override
    public CardCollection chooseCardsToDiscardFrom(Player playerDiscard, SpellAbility sa,
            CardCollection validCards, int min, int max) {
        // Fixed: return first `min` cards sorted alphabetically (no RNG consumed)
        List<Card> sorted = new ArrayList<>(validCards);
        sorted.sort(Comparator.comparing(Card::getName));
        int count = Math.min(min, sorted.size());
        return new CardCollection(sorted.subList(0, count));
    }

    @Override
    public CardCollection chooseCardsToDiscardToMaximumHandSize(int numDiscard) {
        // Fixed: return first `numDiscard` cards sorted alphabetically (no RNG consumed)
        List<Card> hand = new ArrayList<>(player.getCardsIn(ZoneType.Hand));
        hand.sort(Comparator.comparing(Card::getName));
        int count = Math.min(numDiscard, hand.size());
        return new CardCollection(hand.subList(0, count));
    }

    // ── Scry / Library Manipulation ──────────────────────────────────

    @Override
    public ImmutablePair<CardCollection, CardCollection> arrangeForScry(CardCollection topN) {
        // Fixed: keep all on top, nothing to bottom (no RNG consumed)
        return ImmutablePair.of(topN, new CardCollection());
    }

    @Override
    public CardCollectionView orderMoveToZoneList(CardCollectionView cards, ZoneType destinationZone,
            SpellAbility source) {
        // Fixed: keep original order (no RNG consumed)
        return cards;
    }

    // ── Confirmations ─────────────────────────────────────────────────

    @Override
    public boolean confirmAction(SpellAbility sa, PlayerActionConfirmMode mode, String message,
            List<String> options, Card cardToShow, Map<String, Object> params) {
        // Fixed: always confirm (no RNG consumed)
        return true;
    }

    @Override
    public boolean confirmTrigger(WrappedAbility sa) {
        // Fixed: always accept triggers (no RNG consumed)
        return true;
    }

    // ── Numbers & Colors ──────────────────────────────────────────────

    @Override
    public byte chooseColor(String message, SpellAbility sa, ColorSet colors) {
        // Fixed: return first color (no RNG consumed)
        List<Byte> colorList = new ArrayList<>();
        for (Color color : colors) colorList.add(color.getColorMask());
        if (colorList.isEmpty()) return Color.WHITE.getColorMask();
        return colorList.get(0);
    }

    @Override
    public byte chooseColorAllowColorless(String message, Card card, ColorSet colors) {
        // Fixed: return first color (no RNG consumed)
        List<Byte> colorList = new ArrayList<>();
        for (Color color : colors) colorList.add(color.getColorMask());
        if (colorList.isEmpty()) return Color.COLORLESS.getColorMask();
        return colorList.get(0);
    }

    // ── Misc ──────────────────────────────────────────────────────────

    @Override
    public Player chooseStartingPlayer(boolean isFirstGame) {
        return getGame().getPlayers().get(0);
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

    // ── Unless Costs (shock lands etc.) ───────────────────────────────

    @Override
    public boolean payCostToPreventEffect(Cost cost, SpellAbility sa, boolean alreadyPaid,
            FCollectionView<Player> allPayers) {
        if (!ComputerUtilCost.canPayCost(cost, sa, player, true)) {
            return false;
        }
        final CostPayment pay = new CostPayment(cost, sa);
        return pay.payComputerCosts(new AiCostDecision(player, sa, true));
    }

    /**
     * Check if a targeted spell has at least one valid target.
     * Does NOT consume RNG — used during candidate evaluation to match
     * Rust's has_candidates_in_chain filtering.
     */
    private boolean hasDeterministicTargets(SpellAbility sa) {
        for (Player p : getGame().getPlayers()) {
            if (sa.canTarget(p)) return true;
        }
        for (Player p : getGame().getPlayers()) {
            for (Card c : p.getCardsIn(ZoneType.Battlefield)) {
                if (sa.canTarget(c)) return true;
            }
        }
        return false;
    }

    /**
     * Set up deterministic targets for a spell ability.
     * Sorts all candidate targets alphabetically, then picks randomly.
     */
    private void setupDeterministicTargets(SpellAbility sa) {
        sa.resetTargets();

        // Build unified candidate list: players first (by name), then cards (by name)
        List<GameEntity> candidates = new ArrayList<>();
        for (Player p : getGame().getPlayers()) {
            if (sa.canTarget(p)) {
                candidates.add(p);
            }
        }
        // Sort players by name
        candidates.sort(Comparator.comparing(GameEntity::getName));

        List<Card> cardCandidates = new ArrayList<>();
        for (Player p : getGame().getPlayers()) {
            for (Card c : p.getCardsIn(ZoneType.Battlefield)) {
                if (sa.canTarget(c)) {
                    cardCandidates.add(c);
                }
            }
        }
        cardCandidates.sort(Comparator.comparing(Card::getName));

        // Players first, then cards — matching Rust's choose_target_any ordering
        List<GameEntity> allCandidates = new ArrayList<>(candidates);
        allCandidates.addAll(cardCandidates);

        if (allCandidates.isEmpty()) return;

        // Pick random target
        int idx = rng.nextInt(allCandidates.size());
        sa.getTargets().add(allCandidates.get(idx));
    }
}
