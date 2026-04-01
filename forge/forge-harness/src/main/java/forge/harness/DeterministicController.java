package forge.harness;

import com.google.common.collect.Lists;
import com.google.common.collect.ListMultimap;
import com.google.common.collect.Multimap;
import forge.StaticData;
import forge.LobbyPlayer;
import forge.ai.ComputerUtilCombat;
import forge.ai.ComputerUtilCost;
import forge.game.ability.ApiType;
import forge.game.ability.effects.RollDiceEffect;
import forge.game.cost.Cost;
import forge.game.cost.CostEnlist;
import forge.game.cost.CostPart;
import forge.game.player.PlayerController;
import forge.game.cost.CostPartMana;
import forge.card.mana.ManaCost;
import forge.card.mana.ManaCostShard;
import forge.card.ColorSet;
import forge.card.MagicColor.Color;
import forge.deck.Deck;
import forge.deck.DeckSection;
import forge.game.*;
import forge.game.card.*;
import forge.game.combat.Combat;
import forge.game.combat.CombatUtil;
import forge.game.mana.Mana;
import forge.game.mana.ManaCostBeingPaid;
import forge.game.mana.ManaConversionMatrix;
import forge.game.player.*;
import forge.game.card.CounterType;
import forge.game.replacement.ReplacementEffect;
import forge.game.spellability.*;
import forge.card.ICardFace;
import forge.game.keyword.KeywordInterface;
import forge.game.staticability.StaticAbility;
import forge.game.trigger.WrappedAbility;
import forge.game.zone.PlayerZone;
import forge.game.zone.ZoneType;
import forge.item.PaperCard;
import forge.util.ITriggerEvent;
import forge.util.collect.FCollectionView;

import org.apache.commons.lang3.tuple.ImmutablePair;
import org.apache.commons.lang3.tuple.Pair;

import java.util.*;
import java.util.function.Predicate;
import java.util.stream.Collectors;

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
public class DeterministicController extends PlayerController {
    private static final boolean DEBUG_ACTIONS = Boolean.getBoolean("forge.parity.rng.trace");

    private static final int PREFER_ACTION_WEIGHT = 3;
    private final CountingRandom rng;
    private final boolean preferActions;
    private final DeterministicCostPlumbing costPlumbing;
    private final AutoPay autoPay;
    private final DeterministicPlayPlumbing playPlumbing;

    public DeterministicController(Game game, Player p, LobbyPlayer lp, CountingRandom rng, boolean preferActions) {
        super(game, p, lp);
        this.rng = rng;
        this.preferActions = preferActions;
        this.costPlumbing = new DeterministicCostPlumbing(this, this.player);
        this.autoPay = new AutoPay(this.player, this.costPlumbing);
        this.playPlumbing = new DeterministicPlayPlumbing(this, this.player, this.costPlumbing);
    }

    private boolean chooseDeterministicBooleanDecision(final String decisionType, final String falseLabel, final String trueLabel) {
        final boolean accepted = ChoiceSpace.pickBool(rng);
        DecisionLog.logChoice(
                player,
                decisionType,
                Arrays.asList(falseLabel, trueLabel),
                accepted ? trueLabel : falseLabel);
        return accepted;
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
        // Canonical legality source is the engine action space (Card#getAllPossibleAbilities).
        // Preserve engine-provided order; agent only samples from this list.
        final List<SpellAbility> all = ChoiceSpace.sortNative(
                new ArrayList<>(ActionSpace.getPossibleActions(player)),
                ParityOrder.actionComparator());

        if (all.isEmpty()) {
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] pass empty%n", player.getId(),
                        getGame().getPhaseHandler().getTurn());
            }
            return null; // pass — no RNG consumed
        }

        final List<String> opts = ActionSpace.buildMainActionLabels(all);

        final int idx;
        if (preferActions) {
            idx = ChoiceSpace.pickWeightedIndexWithPass(all.size(), PREFER_ACTION_WEIGHT, rng);
        } else {
            idx = ChoiceSpace.pickIndexWithPass(all.size(), rng);
        }
        final String choice = idx >= all.size() ? "PASS" : opts.get(idx);
        DecisionLog.logMainAction(player, opts, choice);
        if (DEBUG_ACTIONS) {
            System.err.printf("[det-java p%d t%d] options=%s idx=%d/%d rng#%d%n", player.getId(),
                    getGame().getPhaseHandler().getTurn(), opts, idx, all.size(), rng.getCallCount());
        }
        if (idx >= all.size()) {
            return null; // pass
        }

        return Lists.newArrayList(all.get(idx));
    }

    @Override
    public boolean playChosenSpellAbility(SpellAbility sa) {
        // Force X to max available mana — matches Rust's choose_x_value default.
        Cost payCosts = sa.getPayCosts();
        if (payCosts != null) {
            ManaCost mana = payCosts.getTotalMana();
            if (mana != null && mana.countX() > 0) {
                int maxX = ComputerUtilCost.getMaxXValue(sa, player, sa.isTrigger());
                if (maxX > 0) {
                    sa.setXManaCostPaid(maxX);
                }
            }
        }

        return playPlumbing.handlePlayingSpellAbilityDeterministic(player, sa, getGame());
    }

    @Override
    public void playSpellAbilityNoStack(SpellAbility effectSA, boolean canSetupTargets) {
        if (canSetupTargets && !effectSA.setupTargets()) {
            return;
        }
        playPlumbing.playNoStackDeterministic(player, effectSA, getGame(), true);
    }

    boolean chooseDeterministicBoolean(
            final String decisionType,
            final String falseLabel,
            final String trueLabel
    ) {
        return chooseDeterministicBooleanDecision(decisionType, falseLabel, trueLabel);
    }

    @Override
    public boolean chooseTargetsFor(final SpellAbility currentAbility) {
        if (Boolean.getBoolean("forge.parity.rng.trace")) {
            String name = currentAbility != null && currentAbility.getHostCard() != null
                ? currentAbility.getHostCard().getName() : "null";
            System.err.printf("[java-target] chooseTargetsFor: %s api=%s rng#%d%n", name,
                currentAbility != null ? currentAbility.getApi() : "null",
                rng.getCallCount());
        }
        if (currentAbility == null || !currentAbility.usesTargeting()) {
            return true;
        }

        final TargetRestrictions tr = currentAbility.getTargetRestrictions();
        if (tr == null) {
            return true;
        }

        while (!currentAbility.isTargetNumberValid()) {
            final List<GameEntity> candidates = tr.getAllCandidates(currentAbility, true);
            final List<GameEntity> valid = new ArrayList<>();
            for (final GameEntity candidate : candidates) {
                if (currentAbility.canTarget(candidate)) {
                    valid.add(candidate);
                }
            }

            if (valid.isEmpty()) {
                return currentAbility.isTargetNumberValid();
            }

            // Sort valid targets canonically (players first by index, then cards by name+parityId)
            // to ensure deterministic cross-engine parity regardless of internal iteration order.
            valid.sort(Comparator.comparing(ParityOrder::targetSortKey));

            final GameEntity chosen = ChoiceSpace.pickOne(valid, rng);
            if (chosen == null) {
                return currentAbility.isTargetNumberValid();
            }
            // getAllCandidates returns Cards from the Stack zone, but CounterEffect.resolve()
            // calls getTargetSpells() which filters for SpellAbility instances. Convert the
            // Card to its corresponding SpellAbility so the counter actually resolves.
            GameObject toAdd = chosen;
            if (chosen instanceof Card c && c.isInZone(ZoneType.Stack)) {
                for (final SpellAbilityStackInstance si : c.getGame().getStack()) {
                    if (si.getSourceCard() == c) {
                        toAdd = si.getSpellAbility();
                        break;
                    }
                }
            }
            currentAbility.getTargets().add(toAdd);

            if (!currentAbility.canAddMoreTarget()) {
                break;
            }

            // For abilities that permit extra targets above minimum, stop or continue
            // via RNG so we stay deterministic without AI heuristics.
            if (currentAbility.isMinTargetChosen() && !ChoiceSpace.pickBool(rng)) {
                break;
            }
        }

        return currentAbility.isTargetNumberValid();
    }

    // ── Combat ────────────────────────────────────────────────────────

    @Override
    public void declareAttackers(Player attacker, Combat combat) {
        // PhaseHandler may re-prompt attack declaration after invalid selections
        // or unpaid attack costs; always rebuild from an empty declaration.
        combat.clearAttackers();
        final List<Card> legalAttackers = new ArrayList<>();
        for (final Card c : attacker.getCreaturesInPlay()) {
            for (final GameEntity defender : combat.getDefenders()) {
                if (CombatUtil.canAttack(c, defender)) {
                    legalAttackers.add(c);
                    break;
                }
            }
        }
        final List<Card> candidates = ChoiceSpace.sortNative(legalAttackers, ParityOrder.cardComparator());
        final List<String> attackerLabels = ParityCardMap.disambiguateCards(candidates, Card::getName);
        for (int cIdx = 0; cIdx < candidates.size(); cIdx++) {
            final Card c = candidates.get(cIdx);
            final String attackerLabel = attackerLabels.get(cIdx);
            List<GameEntity> defenders = new ArrayList<>();
            for (final GameEntity defender : combat.getDefenders()) {
                if (CombatUtil.canAttack(c, defender)) {
                    defenders.add(defender);
                }
            }
            defenders = ParityOrder.sortDefenders(defenders);
            final List<String> options = new ArrayList<>();
            options.add("PASS");
            for (int i = 0; i < defenders.size(); i++) {
                options.add("ATTACK:" + attackerLabel + "->D" + i);
            }

            final int roll = ChoiceSpace.pickIndex(2, rng);
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] atk roll %s -> %d rng#%d%n",
                    player.getId(), getGame().getPhaseHandler().getTurn(), c.getName(), roll, rng.getCallCount());
            }
            String choice = "PASS";
            if (roll == 1) {
                final GameEntity defender = ChoiceSpace.pickOne(defenders, rng);
                if (defender != null) {
                    combat.addAttacker(c, defender);
                    final int idx = defenders.indexOf(defender);
                    if (idx >= 0) {
                        choice = "ATTACK:" + attackerLabel + "->D" + idx;
                    }
                }
            }
            DecisionLog.logChoice(player, "combat_attacker_choice", options, choice);
        }

        // Intentionally do not "fix up" invalid declarations here.
        // PhaseHandler is the canonical owner of attacker-validation/re-prompt flow.
    }

    @Override
    public void declareBlockers(Player defender, Combat combat) {
        List<Card> attackers = new ArrayList<>(combat.getAttackers());
        if (attackers.isEmpty()) return;

        final List<Card> legalBlockers = new ArrayList<>();
        for (final Card blocker : defender.getCreaturesInPlay()) {
            if (CombatUtil.canBlock(blocker, combat)) {
                legalBlockers.add(blocker);
            }
        }
        final List<Card> blockers = ChoiceSpace.sortNative(legalBlockers, ParityOrder.cardComparator());
        final List<String> blockerLabels = ParityCardMap.disambiguateCards(blockers, Card::getName);
        for (int bIdx = 0; bIdx < blockers.size(); bIdx++) {
            final Card blocker = blockers.get(bIdx);
            final String blockerLabel = blockerLabels.get(bIdx);
            final List<Card> legalForBlocker = new ArrayList<>();
            for (final Card attacker : attackers) {
                if (CombatUtil.canBlock(attacker, blocker, combat)) {
                    legalForBlocker.add(attacker);
                }
            }
            final List<Card> options = ChoiceSpace.sortNative(legalForBlocker, ParityOrder.cardComparator());
            final List<String> optionLabels = ParityCardMap.disambiguateCards(options, Card::getName);
            final List<String> loggedOptions = new ArrayList<>();
            loggedOptions.add("PASS");
            for (final String attackerLabel : optionLabels) {
                loggedOptions.add("BLOCK:" + blockerLabel + "->" + attackerLabel);
            }
            final int choice = ChoiceSpace.pickIndexWithPass(options.size(), rng);
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] blk roll %s -> %d/%d%n",
                    player.getId(), getGame().getPhaseHandler().getTurn(),
                    blocker.getName(), choice, options.size());
            }
            String loggedChoice = "PASS";
            if (choice > 0 && choice <= options.size()) {
                final Card chosen = options.get(choice - 1);
                combat.addBlocker(chosen, blocker);
                loggedChoice = "BLOCK:" + blockerLabel + "->" + optionLabels.get(choice - 1);
            }
            DecisionLog.logChoice(player, "combat_blocker_choice", loggedOptions, loggedChoice);
        }
    }

    @Override
    public CardCollection orderBlockers(Card attacker, CardCollection blockers) {
        blockers.sort(Comparator.comparing((Card c) -> c.getName()).thenComparingInt(ParityCardMap::parityId));
        return blockers;
    }

    @Override
    public CardCollection orderBlocker(Card attacker, Card blocker, CardCollection oldBlockers) {
        CardCollection all = new CardCollection(oldBlockers);
        all.add(blocker);
        all.sort(Comparator.comparing((Card c) -> c.getName()).thenComparingInt(ParityCardMap::parityId));
        return all;
    }

    @Override
    public CardCollection orderAttackers(Card blocker, CardCollection attackers) {
        attackers.sort(Comparator.comparing((Card c) -> c.getName()).thenComparingInt(ParityCardMap::parityId));
        return attackers;
    }

    @Override
    public Map<Card, Integer> assignCombatDamage(Card attacker, CardCollectionView blockers,
            CardCollectionView remaining, int damageDealt, GameEntity defender, boolean overrideOrder) {
        Map<Card, Integer> result = new LinkedHashMap<>();
        int damageLeft = damageDealt;
        final boolean canTrampleToDefender = defender != null && attacker.hasKeyword("Trample");
        for (Card blocker : blockers) {
            int lethal = ComputerUtilCombat.getEnoughDamageToKill(blocker, damageLeft, attacker, false, false);
            int assign = Math.min(lethal, damageLeft);
            result.put(blocker, assign);
            damageLeft -= assign;
            if (damageLeft <= 0) break;
        }
        if (damageLeft > 0) {
            if (canTrampleToDefender) {
                // Java controller contract: null key means defending entity.
                result.put(null, damageLeft);
            } else if (!blockers.isEmpty()) {
                Card last = blockers.get(blockers.size() - 1);
                result.put(last, result.getOrDefault(last, 0) + damageLeft);
            }
        }
        return result;
    }

    // ── Targeting & Choices ───────────────────────────────────────────

    @Override
    public <T extends GameEntity> T chooseSingleEntityForEffect(FCollectionView<T> optionList,
            DelayedReveal delayedReveal, SpellAbility sa, String title, boolean isOptional,
            Player relatedPlayer, Map<String, Object> params) {
        if (delayedReveal != null) reveal(delayedReveal);
        // Sort by (name, parityId) for deterministic cross-engine parity.
        // Avoids HashMap/collection ordering differences between Java and Rust.
        java.util.List<T> sorted = new java.util.ArrayList<>(optionList);
        sorted.sort(java.util.Comparator.comparing((T e) -> e.getName())
                .thenComparingInt(e -> (e instanceof Card) ? ParityCardMap.parityId((Card) e) : 0));
        return ChoiceSpace.pickOne(sorted, rng);
    }

    @Override
    public CardCollectionView chooseCardsForEffect(CardCollectionView sourceList, SpellAbility sa,
            String title, int min, int max, boolean isOptional, Map<String, Object> params) {
        // Sort cards canonically for deterministic cross-engine parity
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(sourceList));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), min, max, rng);
    }

    @Override
    public CardCollection chooseCardsForEffectMultiple(
            Map<String, CardCollection> validMap,
            SpellAbility sa,
            String title,
            boolean isOptional
    ) {
        final CardCollection chosen = new CardCollection();
        if (validMap == null || validMap.isEmpty()) {
            return chosen;
        }
        for (final CardCollection pool : validMap.values()) {
            if (pool == null || pool.isEmpty()) {
                continue;
            }
            final CardCollection remaining = new CardCollection(pool);
            remaining.removeAll(chosen);
            final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(remaining));
            final Card pick = ChoiceSpace.pickOne(sorted, rng);
            if (pick != null) {
                chosen.add(pick);
            }
        }
        return chosen;
    }

    @Override
    public <T extends GameEntity> List<T> chooseEntitiesForEffect(
            FCollectionView<T> optionList, int min, int max, DelayedReveal delayedReveal, SpellAbility sa,
            String title, Player relatedPlayer, Map<String, Object> params) {
        if (delayedReveal != null) {
            reveal(delayedReveal);
        }
        final List<T> pool = new ArrayList<>(optionList);
        // Sort pool canonically for deterministic cross-engine parity (matches Rust sort).
        pool.sort(Comparator.comparing(ParityOrder::targetSortKey));
        final int count = ChoiceSpace.pickCount(min, max, pool.size(), rng);
        final List<T> selected = new ArrayList<>(count);
        for (int i = 0; i < count && !pool.isEmpty(); i++) {
            selected.add(pool.remove(ChoiceSpace.pickIndex(pool.size(), rng)));
        }

        if (sa != null && sa.getApi() == ApiType.Dig) {
            final List<String> options = new ArrayList<>(pool.size() + selected.size() + 1);
            final List<Card> cards = new ArrayList<>();
            for (final T entity : optionList) {
                if (entity instanceof Card) {
                    cards.add((Card) entity);
                }
            }
            if (cards.size() == optionList.size()) {
                options.addAll(ParityCardMap.disambiguateCards(cards, Card::getName));
            } else {
                for (final T entity : optionList) {
                    options.add(entity == null ? "?" : entity.toString());
                }
            }

            final String choice;
            if (selected.isEmpty()) {
                choice = "PASS";
            } else {
                final List<String> chosenLabels = new ArrayList<>();
                for (final T entity : selected) {
                    if (entity instanceof Card) {
                        final Card card = (Card) entity;
                        chosenLabels.add(card.getName() + "@" + ParityCardMap.parityId(card));
                    } else {
                        chosenLabels.add(entity == null ? "?" : entity.toString());
                    }
                }
                choice = String.join(",", chosenLabels);
            }
            DecisionLog.logChoice(player, "choose_dig", options, choice);
        }

        return selected;
    }

    // ── Sacrifice / Destroy ────────────────────────────────────────────
    // Rust's choose_sacrifice sorts alphabetically by name, picks first.

    @Override
    public CardCollectionView choosePermanentsToSacrifice(SpellAbility sa, int min, int max,
            CardCollectionView validTargets, String message) {
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(validTargets));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), min, max, rng);
    }

    @Override
    public CardCollectionView choosePermanentsToDestroy(SpellAbility sa, int min, int max,
            CardCollectionView validTargets, String message) {
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(validTargets));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), min, max, rng);
    }

    // ── Zone Change (Search/Tutor) ──────────────────────────────────

    @Override
    public Card chooseSingleCardForZoneChange(ZoneType destination, List<ZoneType> origin,
            SpellAbility sa, CardCollection fetchList, DelayedReveal delayedReveal,
            String selectPrompt, boolean isOptional, Player decider) {
        if (delayedReveal != null) reveal(delayedReveal);
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId((List<Card>) fetchList);
        return ChoiceSpace.pickOne(sorted, rng);
    }

    @Override
    public CardCollection chooseCardsToDiscardFrom(Player playerDiscard, SpellAbility sa,
            CardCollection validCards, int min, int max) {
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(validCards));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), min, max, rng);
    }

    @Override
    public CardCollection chooseCardsToDiscardToMaximumHandSize(int numDiscard) {
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(player.getCardsIn(ZoneType.Hand)));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), numDiscard, numDiscard, rng);
    }

    // ── Scry / Surveil / Library Manipulation ───────────────────────

    @Override
    public ImmutablePair<CardCollection, CardCollection> arrangeForScry(CardCollection topN) {
        CardCollection top = new CardCollection(topN);
        CardCollection bottom = new CardCollection();
        for (Card c : new ArrayList<>(topN)) {
            if (ChoiceSpace.pickBool(rng)) {
                top.remove(c);
                bottom.add(c);
            }
        }
        return ImmutablePair.of(top, bottom);
    }

    @Override
    public ImmutablePair<CardCollection, CardCollection> arrangeForSurveil(CardCollection topN) {
        CardCollection top = new CardCollection(topN);
        CardCollection gy = new CardCollection();
        for (Card c : new ArrayList<>(topN)) {
            if (ChoiceSpace.pickBool(rng)) {
                top.remove(c);
                gy.add(c);
            }
        }
        return ImmutablePair.of(top, gy);
    }

    @Override
    public CardCollectionView orderMoveToZoneList(CardCollectionView cards, ZoneType destinationZone,
            SpellAbility source) {
        return new CardCollection(cards);
    }

    // ── Charm / Modal ────────────────────────────────────────────────

    @Override
    public List<AbilitySub> chooseModeForAbility(SpellAbility sa, List<AbilitySub> possible, int min, int num, boolean allowRepeat) {
        if (possible == null || possible.isEmpty()) return new ArrayList<>();
        int count = ChoiceSpace.pickCount(min, num, possible.size(), rng);
        List<AbilitySub> pool = new ArrayList<>(possible);
        List<AbilitySub> out = new ArrayList<>();
        for (int i = 0; i < count && !pool.isEmpty(); i++) {
            out.add(pool.remove(ChoiceSpace.pickIndex(pool.size(), rng)));
        }
        return out;
    }

    @Override
    public boolean confirmTrigger(WrappedAbility sa) {
        return chooseDeterministicBooleanDecision("optional_trigger", "DECLINE", "ACCEPT");
    }

    @Override
    public boolean confirmAction(SpellAbility sa, PlayerActionConfirmMode mode, String message,
            List<String> options, Card cardToShow, Map<String, Object> params) {
        return chooseDeterministicBooleanDecision("confirm_action", "DECLINE", "ACCEPT");
    }

    @Override
    public boolean confirmPayment(final forge.game.cost.CostPart costPart, final String prompt, final SpellAbility sa) {
        if (costPart == null || costPart instanceof CostPartMana) {
            return true;
        }
        if (DeterministicCostPlumbing.isSpellPaymentContext(sa)) {
            return true;
        }
        return chooseDeterministicBooleanDecision("confirm_payment", "DECLINE", "ACCEPT");
    }

    @Override
    public boolean chooseBinary(final SpellAbility sa, final String question, final BinaryChoiceType kindOfChoice, final Boolean defaultVal) {
        final String left = kindOfChoice.name() + ":LEFT";
        final String right = kindOfChoice.name() + ":RIGHT";
        return chooseDeterministicBooleanDecision("choose_binary", right, left);
    }

    @Override
    public boolean chooseBinary(final SpellAbility sa, final String question, final BinaryChoiceType kindOfChoice, final Map<String, Object> params) {
        return chooseBinary(sa, question, kindOfChoice, (Boolean) null);
    }

    // ── Additional Costs (Kicker, Buyback, Multikicker, Replicate) ────
    // Rust defaults: choose_kicker→false, choose_buyback→false,
    // choose_multikicker→0, choose_replicate→0.
    // We must match by never paying optional costs.

    @Override
    public List<OptionalCostValue> chooseOptionalCosts(SpellAbility chosen,
            List<OptionalCostValue> optionalCostValues) {
        return Collections.emptyList();
    }

    @Override
    public int chooseNumberForKeywordCost(SpellAbility sa, Cost cost,
            KeywordInterface keyword, String prompt, int max) {
        return 0;
    }

    // ── X-Cost ────────────────────────────────────────────────────────
    // Rust default choose_x_value returns max_x (spend all available mana).
    // NOTE: This mirrors engine-side bounds logic from PlayerControllerHuman.
    // We do not use ComputerUtilCost here because announceRequirements is about
    // choosing a legal announced value (X / AnnounceMax / target-limited ranges),
    // not about paying or validating an entire cost payment plan.

    @Override
    public Integer announceRequirements(SpellAbility ability, String announce) {
        return GuiRepro.announceRequirements(player, ability, announce, rng);
    }

    // ── Numbers & Colors ──────────────────────────────────────────────

    @Override
    public byte chooseColor(String message, SpellAbility sa, ColorSet colors) {
        List<Byte> colorList = new ArrayList<>();
        for (Color color : colors) colorList.add(color.getColorMask());
        if (colorList.isEmpty()) return Color.WHITE.getColorMask();
        return colorList.get(ChoiceSpace.pickIndex(colorList.size(), rng));
    }

    @Override
    public byte chooseColorAllowColorless(String message, Card card, ColorSet colors) {
        List<Byte> colorList = new ArrayList<>();
        for (Color color : colors) colorList.add(color.getColorMask());
        if (colorList.isEmpty()) return Color.COLORLESS.getColorMask();
        return colorList.get(ChoiceSpace.pickIndex(colorList.size(), rng));
    }

    // ── Type / Card Name / Number Selection ────────────────────────────
    // Rust defaults: first valid type, first valid name, min value.

    @Override
    public String chooseSomeType(String kindOfType, SpellAbility sa, Collection<String> validTypes, boolean isOptional) {
        if (validTypes == null || validTypes.isEmpty()) return "";
        List<String> values = new ArrayList<>(validTypes);
        Collections.sort(values);
        return values.get(ChoiceSpace.pickIndex(values.size(), rng));
    }

    @Override
    public String chooseCardName(SpellAbility sa, Predicate<ICardFace> cpp, String valid, String message) {
        final Card source = sa.getHostCard();
        final Predicate<ICardFace> faceFilter = cpp == null ? x -> true : cpp;
        final List<ICardFace> faces = StaticData.instance().getCommonCards().streamAllFaces()
                .filter(faceFilter)
                .filter(face -> {
                    if (valid == null || valid.isEmpty()) {
                        return true;
                    }
                    final PaperCard cp = StaticData.instance().getCommonCards().getCard(face.getName());
                    if (cp == null) {
                        return false;
                    }
                    final Card instanceForPlayer = Card.fromPaperCard(cp, player);
                    final Player sourceController = source == null ? player : source.getController();
                    return instanceForPlayer.isValid(valid, sourceController, source, sa);
                })
                .sorted()
                .collect(Collectors.toList());
        return chooseCardName(sa, faces, message);
    }

    @Override
    public String chooseCardName(SpellAbility sa, List<ICardFace> faces, String message) {
        if (faces == null || faces.isEmpty()) return "";
        return faces.get(ChoiceSpace.pickIndex(faces.size(), rng)).getName();
    }

    @Override
    public int chooseNumber(SpellAbility sa, String title, int min, int max) {
        return ChoiceSpace.pickIntInRange(min, max, rng);
    }

    @Override
    public int chooseNumber(SpellAbility sa, String title, List<Integer> values, Player relatedPlayer) {
        if (values == null || values.isEmpty()) return 0;
        return values.get(ChoiceSpace.pickIndex(values.size(), rng));
    }

    @Override
    public int chooseNumberForCostReduction(final SpellAbility sa, final int min, final int max) {
        return ChoiceSpace.pickIntInRange(min, max, rng);
    }

    // ── Coin Flip ─────────────────────────────────────────────────────
    // Rust default flip_coin_call returns true (always call heads).

    @Override
    public boolean chooseFlipResult(SpellAbility sa, Player flipper, boolean[] results, boolean call) {
        return ChoiceSpace.pickBool(rng);
    }

    // ── Mulligan Bottom Selection ────────────────────────────────────
    // Rust default choose_cards_to_bottom returns first N cards.

    @Override
    public CardCollectionView tuckCardsViaMulligan(Player mulliganingPlayer, int cardsToReturn) {
        CardCollectionView hand = mulliganingPlayer.getCardsIn(ZoneType.Hand);
        CardCollection pool = new CardCollection(hand);
        CardCollection out = new CardCollection();
        int count = Math.min(cardsToReturn, pool.size());
        for (int i = 0; i < count && !pool.isEmpty(); i++) {
            out.add(pool.remove(ChoiceSpace.pickIndex(pool.size(), rng)));
        }
        return out;
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
        return costPlumbing.payWithDeterministicDecision(cost, sa, true);
    }

    @Override
    public boolean payCombatCost(Card c, Cost cost, SpellAbility sa, String prompt) {
        return playPlumbing.playNoStackDeterministic(c.getController(), sa, getGame(), true);
    }

    @Override
    public void orderAndPlaySimultaneousSa(List<SpellAbility> activePlayerSAs) {
        playPlumbing.orderAndPlaySimultaneousSa(activePlayerSAs, getGame());
    }

    @Override
    public boolean playTrigger(Card host, WrappedAbility wrapperAbility, boolean isMandatory) {
        if (playPlumbing.prepareSingleSaDeterministic(host, wrapperAbility, isMandatory)) {
            return playPlumbing.playNoStackDeterministic(
                    wrapperAbility.getActivatingPlayer(), wrapperAbility, getGame(), true);
        }
        return false;
    }

    @Override
    public boolean playSaFromPlayEffect(SpellAbility tgtSA) {
        return playPlumbing.playSaFromPlayEffect(tgtSA, getGame());
    }

    @Override
    public SpellAbility getAbilityToPlay(Card hostCard, List<SpellAbility> abilities, ITriggerEvent triggerEvent) {
        return ChoiceSpace.pickOne(abilities, rng);
    }

    @Override
    public List<PaperCard> sideboard(Deck deck, GameType gameType, String message) {
        return null;
    }

    @Override
    public TargetChoices chooseNewTargetsFor(SpellAbility ability, Predicate<GameObject> filter, boolean optional) {
        return null;
    }

    @Override
    public Pair<SpellAbilityStackInstance, GameObject> chooseTarget(
            SpellAbility sa,
            List<Pair<SpellAbilityStackInstance, GameObject>> allTargets
    ) {
        return ChoiceSpace.pickOne(allTargets, rng);
    }

    @Override
    public boolean helpPayForAssistSpell(ManaCostBeingPaid cost, SpellAbility sa, int max, int requested) {
        return ChoiceSpace.pickBool(rng);
    }

    @Override
    public Player choosePlayerToAssistPayment(FCollectionView<Player> optionList, SpellAbility sa, String title, int max) {
        return ChoiceSpace.pickOne(optionList, rng);
    }

    @Override
    public List<PaperCard> chooseCardsYouWonToAddToDeck(List<PaperCard> losses) {
        return losses == null ? null : new ArrayList<>(losses);
    }

    @Override
    public Map<GameEntity, Integer> divideShield(Card effectSource, Map<GameEntity, Integer> affected, int shieldAmount) {
        final Map<GameEntity, Integer> out = new LinkedHashMap<>();
        if (affected == null || shieldAmount <= 0) {
            return out;
        }
        final List<GameEntity> pool = new ArrayList<>();
        for (final Map.Entry<GameEntity, Integer> e : affected.entrySet()) {
            if (e.getKey() != null && e.getValue() != null && e.getValue() > 0) {
                pool.add(e.getKey());
                out.put(e.getKey(), 0);
            }
        }
        int remaining = shieldAmount;
        while (remaining > 0 && !pool.isEmpty()) {
            final GameEntity chosen = pool.get(ChoiceSpace.pickIndex(pool.size(), rng));
            final int current = out.getOrDefault(chosen, 0);
            final int cap = affected.getOrDefault(chosen, 0);
            if (current >= cap) {
                pool.remove(chosen);
                continue;
            }
            out.put(chosen, current + 1);
            remaining--;
        }
        return out;
    }

    @Override
    public Map<Byte, Integer> specifyManaCombo(SpellAbility sa, ColorSet colorSet, int manaAmount, boolean different) {
        final Map<Byte, Integer> result = new LinkedHashMap<>();
        ColorSet mutable = colorSet;
        for (int i = 0; i < manaAmount; i++) {
            final byte chosen = chooseColor("", sa, mutable);
            result.put(chosen, result.getOrDefault(chosen, 0) + 1);
            if (different) {
                mutable = ColorSet.fromMask(mutable.getColor() & ~chosen);
            }
        }
        return result;
    }

    @Override
    public List<SpellAbility> chooseSpellAbilitiesForEffect(
            List<SpellAbility> spells,
            SpellAbility sa,
            String title,
            int num,
            Map<String, Object> params
    ) {
        if (spells == null || spells.isEmpty() || num <= 0) {
            return new ArrayList<>();
        }
        final List<SpellAbility> pool = new ArrayList<>(spells);
        final int count = Math.min(num, pool.size());
        final List<SpellAbility> out = new ArrayList<>(count);
        for (int i = 0; i < count && !pool.isEmpty(); i++) {
            out.add(pool.remove(ChoiceSpace.pickIndex(pool.size(), rng)));
        }
        return out;
    }

    @Override
    public SpellAbility chooseSingleSpellForEffect(List<SpellAbility> spells, SpellAbility sa, String title, Map<String, Object> params) {
        return ChoiceSpace.pickOne(spells, rng);
    }

    @Override
    public boolean confirmBidAction(SpellAbility sa, PlayerActionConfirmMode bidlife, String string, int bid, Player winner) {
        return ChoiceSpace.pickBool(rng);
    }

    @Override
    public boolean confirmReplacementEffect(ReplacementEffect replacementEffect, SpellAbility effectSA, GameEntity affected, String question) {
        return ChoiceSpace.pickBool(rng);
    }

    @Override
    public boolean confirmStaticApplication(Card hostCard, PlayerActionConfirmMode mode, String message, String logic) {
        return ChoiceSpace.pickBool(rng);
    }

    @Override
    public List<Card> exertAttackers(List<Card> attackers) {
        if (attackers == null) {
            return new ArrayList<>();
        }
        final List<Card> out = new ArrayList<>();
        for (final Card attacker : attackers) {
            if (ChoiceSpace.pickBool(rng)) {
                out.add(attacker);
            }
        }
        return out;
    }

    @Override
    public List<Card> enlistAttackers(List<Card> attackers) {
        if (attackers == null || attackers.isEmpty()) {
            return new ArrayList<>();
        }
        // Use engine legality only: if Enlist cannot currently be paid at all,
        // do not choose any attacker to pay that optional cost.
        if (CostEnlist.getCardsForEnlisting(player).isEmpty()) {
            return new ArrayList<>();
        }
        return Lists.newArrayList(attackers.get(ChoiceSpace.pickIndex(attackers.size(), rng)));
    }

    @Override
    public boolean willPutCardOnTop(Card c) {
        return ChoiceSpace.pickBool(rng);
    }

    @Override
    public CardCollectionView chooseCardsToDiscardUnlessType(int min, CardCollectionView hand, String param, SpellAbility sa) {
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(hand));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), min, min, rng);
    }

    @Override
    public CardCollectionView chooseCardsToDelve(int genericAmount, CardCollection grave) {
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(grave));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), 0, Math.min(genericAmount, grave.size()), rng);
    }

    @Override
    public Map<Card, ManaCostShard> chooseCardsForConvokeOrImprovise(
            SpellAbility sa,
            ManaCost manaCost,
            CardCollectionView untappedCards,
            boolean artifacts,
            boolean creatures,
            Integer maxReduction
    ) {
        final Map<Card, ManaCostShard> out = new LinkedHashMap<>();
        if (untappedCards == null || untappedCards.isEmpty()) {
            return out;
        }
        final int cap = maxReduction == null ? untappedCards.size() : Math.max(0, Math.min(maxReduction, untappedCards.size()));
        final int count = ChoiceSpace.pickCount(0, cap, untappedCards.size(), rng);
        final List<Card> sortedUntapped = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(untappedCards));
        final CardCollection pool = new CardCollection(sortedUntapped);
        for (int i = 0; i < count && !pool.isEmpty(); i++) {
            final Card chosen = pool.remove(ChoiceSpace.pickIndex(pool.size(), rng));
            out.put(chosen, ManaCostShard.GENERIC);
        }
        return out;
    }

    @Override
    public List<Card> chooseCardsForSplice(SpellAbility sa, List<Card> cards) {
        if (cards == null || cards.isEmpty()) {
            return new ArrayList<>();
        }
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(cards);
        final List<Card> out = new ArrayList<>();
        for (final Card card : sorted) {
            if (ChoiceSpace.pickBool(rng)) {
                out.add(card);
            }
        }
        return out;
    }

    @Override
    public CardCollectionView chooseCardsToRevealFromHand(int min, int max, CardCollectionView valid) {
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(valid));
        return ChoiceSpace.pickManyCards(new CardCollection(sorted), min, max, rng);
    }

    @Override
    public List<SpellAbility> chooseSaToActivateFromOpeningHand(List<SpellAbility> usableFromOpeningHand) {
        if (usableFromOpeningHand == null || usableFromOpeningHand.isEmpty()) {
            return new ArrayList<>();
        }
        final List<SpellAbility> out = new ArrayList<>();
        for (final SpellAbility sa : usableFromOpeningHand) {
            if (ChoiceSpace.pickBool(rng)) {
                out.add(sa);
            }
        }
        return out;
    }

    @Override
    public PlayerZone chooseStartingHand(List<PlayerZone> zones) {
        return ChoiceSpace.pickOne(zones, rng);
    }

    @Override
    public Mana chooseManaFromPool(List<Mana> manaChoices) {
        return ChoiceSpace.pickOne(manaChoices, rng);
    }

    @Override
    public String chooseSector(Card assignee, String ai, List<String> sectors) {
        return ChoiceSpace.pickOne(sectors, rng);
    }

    @Override
    public List<Card> chooseContraptionsToCrank(List<Card> contraptions) {
        if (contraptions == null || contraptions.isEmpty()) {
            return new ArrayList<>();
        }
        final List<Card> out = new ArrayList<>();
        for (final Card contraption : contraptions) {
            if (ChoiceSpace.pickBool(rng)) {
                out.add(contraption);
            }
        }
        return out;
    }

    @Override
    public int chooseSprocket(Card assignee, boolean forceDifferent) {
        return ChoiceSpace.pickIntInRange(1, 3, rng);
    }

    @Override
    public PlanarDice choosePDRollToIgnore(List<PlanarDice> rolls) {
        return ChoiceSpace.pickOne(rolls, rng);
    }

    @Override
    public Integer chooseRollToIgnore(List<Integer> rolls) {
        return ChoiceSpace.pickOne(rolls, rng);
    }

    @Override
    public List<Integer> chooseDiceToReroll(List<Integer> rolls) {
        if (rolls == null || rolls.isEmpty()) {
            return new ArrayList<>();
        }
        final List<Integer> out = new ArrayList<>();
        for (final Integer roll : rolls) {
            if (ChoiceSpace.pickBool(rng)) {
                out.add(roll);
            }
        }
        return out;
    }

    @Override
    public Integer chooseRollToModify(List<Integer> rolls) {
        return ChoiceSpace.pickOne(rolls, rng);
    }

    @Override
    public RollDiceEffect.DieRollResult chooseRollToSwap(List<RollDiceEffect.DieRollResult> rolls) {
        return ChoiceSpace.pickOne(rolls, rng);
    }

    @Override
    public String chooseRollSwapValue(List<String> swapChoices, Integer currentResult, int power, int toughness) {
        return ChoiceSpace.pickOne(swapChoices, rng);
    }

    @Override
    public Object vote(
            SpellAbility sa,
            String prompt,
            List<Object> options,
            ListMultimap<Object, Player> votes,
            Player forPlayer,
            boolean optional
    ) {
        if (optional && ChoiceSpace.pickBool(rng)) {
            return null;
        }
        return ChoiceSpace.pickOne(options, rng);
    }

    @Override
    public ColorSet chooseColors(String message, SpellAbility sa, int min, int max, ColorSet options) {
        if (options == null || options.isColorless()) {
            return ColorSet.fromMask(0);
        }
        final List<Byte> colors = new ArrayList<>();
        for (final Color color : options) {
            colors.add(color.getColorMask());
        }
        final int count = ChoiceSpace.pickCount(min, max, colors.size(), rng);
        int mask = 0;
        for (int i = 0; i < count && !colors.isEmpty(); i++) {
            final byte chosen = colors.remove(ChoiceSpace.pickIndex(colors.size(), rng));
            mask |= chosen;
        }
        return ColorSet.fromMask(mask);
    }

    @Override
    public ICardFace chooseSingleCardFace(SpellAbility sa, String message, Predicate<ICardFace> cpp, String name) {
        final Predicate<ICardFace> filter = cpp == null ? x -> true : cpp;
        final List<ICardFace> faces = StaticData.instance().getCommonCards().streamAllFaces()
                .filter(filter)
                .collect(Collectors.toList());
        return chooseSingleCardFace(sa, faces, message);
    }

    @Override
    public ICardFace chooseSingleCardFace(SpellAbility sa, List<ICardFace> faces, String message) {
        return ChoiceSpace.pickOne(faces, rng);
    }

    @Override
    public CardState chooseSingleCardState(SpellAbility sa, List<CardState> states, String message, Map<String, Object> params) {
        return ChoiceSpace.pickOne(states, rng);
    }

    @Override
    public boolean chooseCardsPile(SpellAbility sa, CardCollectionView pile1, CardCollectionView pile2, String faceUp) {
        return ChoiceSpace.pickBool(rng);
    }

    @Override
    public CounterType chooseCounterType(List<CounterType> options, SpellAbility sa, String prompt, Map<String, Object> params) {
        return ChoiceSpace.pickOne(options, rng);
    }

    @Override
    public String chooseKeywordForPump(List<String> options, SpellAbility sa, String prompt, Card tgtCard) {
        return ChoiceSpace.pickOne(options, rng);
    }

    @Override
    public ReplacementEffect chooseSingleReplacementEffect(List<ReplacementEffect> possibleReplacers) {
        return ChoiceSpace.pickOne(ParityOrder.sortReplacementEffects(possibleReplacers), rng);
    }

    @Override
    public StaticAbility chooseSingleStaticAbility(String prompt, List<StaticAbility> possibleReplacers) {
        // Do NOT consume RNG here. This method is called during action-space evaluation
        // (canPlay() checks), not just at resolution time. Consuming RNG here causes
        // desync with Rust, which selects static abilities algorithmically without
        // calling any agent callback.
        if (possibleReplacers == null || possibleReplacers.isEmpty()) {
            return null;
        }
        return possibleReplacers.get(0);
    }

    @Override
    public String chooseProtectionType(String string, SpellAbility sa, List<String> choices) {
        return ChoiceSpace.pickOne(choices, rng);
    }

    @Override
    public void revealAnte(String message, Multimap<Player, PaperCard> removedAnteCards) {
        // headless — no-op
    }

    @Override
    public void revealAISkipCards(String message, Map<Player, Map<DeckSection, List<? extends PaperCard>>> deckCards) {
        // headless — no-op
    }

    @Override
    public void revealUnsupported(Map<Player, List<PaperCard>> unsupported) {
        // headless — no-op
    }

    @Override
    public void resetAtEndOfTurn() {
        // headless — no-op
    }

    @Override
    public List<CostPart> orderCosts(List<CostPart> costs) {
        if (costs == null || costs.size() < 2) {
            return costs;
        }
        final List<CostPart> out = new ArrayList<>(costs);
        Collections.shuffle(out, rng);
        return out;
    }

    @Override
    public boolean payCostDuringRoll(Cost cost, SpellAbility sa, FCollectionView<Player> allPayers) {
        if (!ComputerUtilCost.canPayCost(cost, sa, player, true)) {
            return false;
        }
        return costPlumbing.payWithDeterministicDecision(cost, sa, true);
    }

    @Override
    public boolean payManaCost(
            ManaCost toPay,
            CostPartMana costPartMana,
            SpellAbility sa,
            String prompt,
            ManaConversionMatrix matrix,
            boolean effect
    ) {
        return autoPay.payManaCost(toPay, sa, effect);
    }

    @Override
    public List<Card> chooseCardsForZoneChange(
            ZoneType destination,
            List<ZoneType> origin,
            SpellAbility sa,
            CardCollection fetchList,
            int min,
            int max,
            DelayedReveal delayedReveal,
            String selectPrompt,
            Player decider
    ) {
        if (delayedReveal != null) {
            reveal(delayedReveal);
        }
        final List<Card> sorted = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(fetchList));
        final CardCollection chosen = ChoiceSpace.pickManyCards(new CardCollection(sorted), min, max, rng);
        return new ArrayList<>(chosen);
    }

    @Override
    public void autoPassCancel() {
        // headless — no-op
    }

    @Override
    public void awaitNextInput() {
        // headless — no-op
    }

    @Override
    public void cancelAwaitNextInput() {
        // headless — no-op
    }

}
