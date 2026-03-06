package forge.harness;

import com.google.common.collect.Lists;
import forge.StaticData;
import forge.LobbyPlayer;
import forge.ai.AiCostDecision;
import forge.ai.ComputerUtil;
import forge.ai.ComputerUtilCombat;
import forge.ai.ComputerUtilCost;
import forge.ai.PlayerControllerAi;
import forge.game.cost.Cost;
import forge.game.cost.CostPayment;
import forge.card.mana.ManaCost;
import forge.card.ColorSet;
import forge.card.MagicColor.Color;
import forge.game.*;
import forge.game.card.*;
import forge.game.combat.Combat;
import forge.game.combat.CombatUtil;
import forge.game.player.*;
import forge.game.spellability.*;
import forge.card.ICardFace;
import forge.game.keyword.KeywordInterface;
import forge.game.trigger.WrappedAbility;
import forge.game.zone.ZoneType;
import forge.item.PaperCard;
import forge.util.collect.FCollectionView;

import org.apache.commons.lang3.tuple.ImmutablePair;

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
public class DeterministicController extends PlayerControllerAi {
    private static final boolean DEBUG_ACTIONS = false;

    private static final int PREFER_ACTION_WEIGHT = 3;
    private final Random rng;
    private final boolean preferActions;
    public DeterministicController(Game game, Player p, LobbyPlayer lp, Random rng, boolean preferActions) {
        super(game, p, lp);
        this.rng = rng;
        this.preferActions = preferActions;
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

        List<String> opts = new ArrayList<>();
        for (SpellAbility sa : all) {
            String kind = sa.isLandAbility() ? "LAND" : (sa.isSpell() ? "SPELL" : (sa.isManaAbility() ? "MANA" : "AB"));
            String fbTag = sa.isFlashback() ? "[FB]" : "";
            opts.add(kind + ":" + sa.getHostCard().getName() + fbTag);
        }
        opts = ParityCardMap.disambiguateAbilities(all, sa -> {
            String kind = sa.isLandAbility() ? "LAND" : (sa.isSpell() ? "SPELL" : (sa.isManaAbility() ? "MANA" : "AB"));
            String fbTag = sa.isFlashback() ? "[FB]" : "";
            return kind + ":" + sa.getHostCard().getName() + fbTag;
        });

        final int idx;
        if (preferActions) {
            idx = ChoiceSpace.pickWeightedIndexWithPass(all.size(), PREFER_ACTION_WEIGHT, rng);
        } else {
            idx = ChoiceSpace.pickIndexWithPass(all.size(), rng);
        }
        final String choice = idx >= all.size() ? "PASS" : opts.get(idx);
        DecisionLog.logMainAction(player, opts, choice);
        if (DEBUG_ACTIONS) {
            System.err.printf("[det-java p%d t%d] options=%s idx=%d/%d%n", player.getId(),
                    getGame().getPhaseHandler().getTurn(), opts, idx, all.size());
        }
        if (idx >= all.size()) {
            return null; // pass
        }

        return Lists.newArrayList(all.get(idx));
    }

    @Override
    public boolean playChosenSpellAbility(SpellAbility sa) {
        // Use engine targeting flow before cast/activation.
        // Mirrors UI path where SpellAbility.setupTargets() is authoritative.
        if (sa.usesTargeting() && !sa.setupTargets()) {
            return false;
        }

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

        return ComputerUtil.handlePlayingSpellAbility(player, sa, getGame());
    }

    @Override
    public boolean chooseTargetsFor(final SpellAbility currentAbility) {
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

            final GameEntity chosen = ChoiceSpace.pickOne(valid, rng);
            if (chosen == null) {
                return currentAbility.isTargetNumberValid();
            }
            currentAbility.getTargets().add(chosen);

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
        final List<Card> candidates = ChoiceSpace.sortNative(
                CombatChoiceSpace.legalAttackers(attacker, combat),
                ParityOrder.cardComparator());
        final List<String> attackerLabels = ParityCardMap.disambiguateCards(candidates, Card::getName);
        for (int cIdx = 0; cIdx < candidates.size(); cIdx++) {
            final Card c = candidates.get(cIdx);
            final String attackerLabel = attackerLabels.get(cIdx);
            final List<GameEntity> defenders = ParityOrder.sortDefenders(
                    CombatChoiceSpace.legalDefendersForAttacker(c, combat));
            final List<String> options = new ArrayList<>();
            options.add("PASS");
            for (int i = 0; i < defenders.size(); i++) {
                options.add("ATTACK:" + attackerLabel + "->D" + i);
            }

            final int roll = ChoiceSpace.pickIndex(2, rng);
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] atk roll %s -> %d%n",
                    player.getId(), getGame().getPhaseHandler().getTurn(), c.getName(), roll);
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

        // Match Java AI fallback: if our random declaration is illegal, replace it
        // with the engine-provided legal attacker map to guarantee progress.
        if (!CombatUtil.validateAttackers(combat)) {
            combat.clearAttackers();
            final Map<Card, GameEntity> legal = combat.getAttackConstraints().getLegalAttackers().getLeft();
            for (final Map.Entry<Card, GameEntity> e : legal.entrySet()) {
                combat.addAttacker(e.getKey(), e.getValue());
            }
        }
    }

    @Override
    public void declareBlockers(Player defender, Combat combat) {
        List<Card> attackers = new ArrayList<>(combat.getAttackers());
        if (attackers.isEmpty()) return;

        final List<Card> blockers = ChoiceSpace.sortNative(
                CombatChoiceSpace.legalBlockers(defender, combat),
                ParityOrder.cardComparator());
        final List<String> blockerLabels = ParityCardMap.disambiguateCards(blockers, Card::getName);
        for (int bIdx = 0; bIdx < blockers.size(); bIdx++) {
            final Card blocker = blockers.get(bIdx);
            final String blockerLabel = blockerLabels.get(bIdx);
            final List<Card> options = ChoiceSpace.sortNative(
                    CombatChoiceSpace.legalAttackersForBlocker(blocker, attackers, combat),
                    ParityOrder.cardComparator());
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
        return ChoiceSpace.pickOne(optionList, rng);
    }

    @Override
    public CardCollectionView chooseCardsForEffect(CardCollectionView sourceList, SpellAbility sa,
            String title, int min, int max, boolean isOptional, Map<String, Object> params) {
        return ChoiceSpace.pickManyCards(sourceList, min, max, rng);
    }

    // ── Sacrifice / Destroy ────────────────────────────────────────────
    // Rust's choose_sacrifice sorts alphabetically by name, picks first.

    @Override
    public CardCollectionView choosePermanentsToSacrifice(SpellAbility sa, int min, int max,
            CardCollectionView validTargets, String message) {
        return ChoiceSpace.pickManyCards(validTargets, min, max, rng);
    }

    @Override
    public CardCollectionView choosePermanentsToDestroy(SpellAbility sa, int min, int max,
            CardCollectionView validTargets, String message) {
        return ChoiceSpace.pickManyCards(validTargets, min, max, rng);
    }

    // ── Zone Change (Search/Tutor) ──────────────────────────────────

    @Override
    public Card chooseSingleCardForZoneChange(ZoneType destination, List<ZoneType> origin,
            SpellAbility sa, CardCollection fetchList, DelayedReveal delayedReveal,
            String selectPrompt, boolean isOptional, Player decider) {
        if (delayedReveal != null) reveal(delayedReveal);
        return ChoiceSpace.pickOne((List<Card>) fetchList, rng);
    }

    @Override
    public CardCollection chooseCardsToDiscardFrom(Player playerDiscard, SpellAbility sa,
            CardCollection validCards, int min, int max) {
        return ChoiceSpace.pickManyCards(validCards, min, max, rng);
    }

    @Override
    public CardCollection chooseCardsToDiscardToMaximumHandSize(int numDiscard) {
        return ChoiceSpace.pickManyCards(player.getCardsIn(ZoneType.Hand), numDiscard, numDiscard, rng);
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
            out.add(pool.remove(rng.nextInt(pool.size())));
        }
        return out;
    }

    @Override
    public boolean confirmTrigger(WrappedAbility sa) {
        final boolean accept = ChoiceSpace.pickBool(rng);
        DecisionLog.logChoice(
                player,
                "optional_trigger",
                Arrays.asList("DECLINE", "ACCEPT"),
                accept ? "ACCEPT" : "DECLINE");
        return accept;
    }

    @Override
    public boolean confirmAction(SpellAbility sa, PlayerActionConfirmMode mode, String message,
            List<String> options, Card cardToShow, Map<String, Object> params) {
        final boolean accept = ChoiceSpace.pickBool(rng);
        DecisionLog.logChoice(
                player,
                "confirm_action",
                Arrays.asList("DECLINE", "ACCEPT"),
                accept ? "ACCEPT" : "DECLINE");
        return accept;
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
        return colorList.get(rng.nextInt(colorList.size()));
    }

    @Override
    public byte chooseColorAllowColorless(String message, Card card, ColorSet colors) {
        List<Byte> colorList = new ArrayList<>();
        for (Color color : colors) colorList.add(color.getColorMask());
        if (colorList.isEmpty()) return Color.COLORLESS.getColorMask();
        return colorList.get(rng.nextInt(colorList.size()));
    }

    // ── Type / Card Name / Number Selection ────────────────────────────
    // Rust defaults: first valid type, first valid name, min value.

    @Override
    public String chooseSomeType(String kindOfType, SpellAbility sa, Collection<String> validTypes, boolean isOptional) {
        if (validTypes == null || validTypes.isEmpty()) return "";
        List<String> values = new ArrayList<>(validTypes);
        return values.get(rng.nextInt(values.size()));
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
        return faces.get(rng.nextInt(faces.size())).getName();
    }

    @Override
    public int chooseNumber(SpellAbility sa, String title, int min, int max) {
        return ChoiceSpace.pickIntInRange(min, max, rng);
    }

    @Override
    public int chooseNumber(SpellAbility sa, String title, List<Integer> values, Player relatedPlayer) {
        if (values == null || values.isEmpty()) return 0;
        return values.get(rng.nextInt(values.size()));
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
            out.add(pool.remove(rng.nextInt(pool.size())));
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
        final CostPayment pay = new CostPayment(cost, sa);
        return pay.payComputerCosts(new AiCostDecision(player, sa, true));
    }

}
