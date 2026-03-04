package forge.harness;

import com.google.common.collect.Lists;
import forge.LobbyPlayer;
import forge.ai.AiCostDecision;
import forge.ai.ComputerUtil;
import forge.ai.ComputerUtilAbility;
import forge.ai.ComputerUtilCombat;
import forge.ai.ComputerUtilCost;
import forge.ai.PlayerControllerAi;
import forge.game.cost.Cost;
import forge.game.cost.CostPayment;
import forge.card.mana.ManaCost;
import forge.card.ColorSet;
import forge.card.MagicColor;
import forge.card.MagicColor.Color;
import forge.game.*;
import forge.game.card.*;
import forge.game.combat.Combat;
import forge.game.combat.CombatUtil;
import forge.game.player.*;
import forge.game.spellability.*;
import forge.game.ability.ApiType;
import forge.card.ICardFace;
import forge.game.keyword.KeywordInterface;
import forge.game.trigger.WrappedAbility;
import forge.game.zone.ZoneType;
import forge.util.collect.FCollectionView;

import org.apache.commons.lang3.tuple.ImmutablePair;

import java.util.*;
import java.util.function.Predicate;

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
    private static final boolean DEBUG_ACTIONS = true;

    /** Weight multiplier for actions vs pass when preferActions is enabled. Must match Rust's PREFER_ACTION_WEIGHT. */
    private static final int PREFER_ACTION_WEIGHT = 3;

    /** Shared RNG for all decisions — same instance used by both players. */
    private final Random rng;
    /** If true, bias random main-phase choices toward taking an action over pass. */
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
        // Only consume RNG during sorcery-speed windows (our main phase, stack empty).
        // Matches Rust's choose_action which skips non-sorcery-speed priority.
        // Java calls this method for ALL priority windows, including instant-speed
        // during combat/opponent's turn, but those must not consume our shared RNG.
        forge.game.phase.PhaseHandler ph = getGame().getPhaseHandler();
        boolean isSorcerySpeed = ph.isPlayerTurn(player)
            && (ph.is(forge.game.phase.PhaseType.MAIN1) || ph.is(forge.game.phase.PhaseType.MAIN2))
            && getGame().getStack().isEmpty();
        if (!isSorcerySpeed) {
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] pass non-sorcery%n", player.getId(),
                        getGame().getPhaseHandler().getTurn());
            }
            return null; // pass priority — no RNG consumed
        }

        // Build combined list: lands first (sorted), then spells (sorted),
        // then activatable non-mana abilities (sorted).
        // Use Java's native legality filtering via getAllPossibleAbilities(..., true).
        // RNG consumption: exactly 1 call per decision if any actions exist.

        // Only play during main phases — matches Rust's DeterministicAgent which
        // only plays during Main1/Main2. Without this check, the Java engine would
        // allow sorcery-speed spells during any priority window (upkeep, combat, etc.)
        // because it trusts the controller to enforce timing.
        if (!player.canCastSorcery()) {
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] pass !canCastSorcery%n", player.getId(),
                        getGame().getPhaseHandler().getTurn());
            }
            return null; // pass during non-main phases
        }

        CardCollectionView hand = player.getCardsIn(ZoneType.Hand);

        // 1. Land plays
        List<SpellAbility> landPlays = new ArrayList<>();
        List<SpellAbility> spellPlays = new ArrayList<>();
        List<SpellAbility> activatable = new ArrayList<>();

        for (Card c : hand) {
            List<SpellAbility> abilities = c.getAllPossibleAbilities(player, true);
            for (SpellAbility sa : abilities) {
                sa.setActivatingPlayer(player);
                // Skip alternative costs — Rust always uses normal cost
                if (sa.getAlternativeCost() != null) {
                    continue;
                }
                if (sa.isLandAbility()) {
                    landPlays.add(sa);
                } else if (sa.isSpell()) {
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    spellPlays.add(sa);
                } else if (sa.isAbility() && !sa.isManaAbility()) {
                    // Rust parity engine does not yet materialize K:Equip keyword
                    // activations in card generation, so exclude them here to keep
                    // deterministic action spaces mirrored.
                    if (sa.isEquip()) {
                        continue;
                    }
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    activatable.add(sa);
                }
            }
        }

        landPlays.sort(Comparator.comparing(sa -> sa.getHostCard().getName()));
        spellPlays.sort(Comparator.comparing(sa -> sa.getHostCard().getName()));

        // 2. Activatable non-mana abilities from battlefield.
        List<SpellAbility> all = new ArrayList<>();
        all.addAll(landPlays);
        all.addAll(spellPlays);
        CardCollectionView battlefield = player.getCardsIn(ZoneType.Battlefield);
        for (Card c : battlefield) {
            for (SpellAbility sa : c.getAllPossibleAbilities(player, true)) {
                sa.setActivatingPlayer(player);
                if (sa.isAbility() && !sa.isManaAbility()) {
                    if (sa.isEquip()) {
                        continue;
                    }
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    activatable.add(sa);
                }
            }
        }

        // 3. Graveyard: Flashback, Escape, and graveyard-activated abilities.
        CardCollectionView graveyard = player.getCardsIn(ZoneType.Graveyard);
        for (Card c : graveyard) {
            for (SpellAbility sa : c.getAllPossibleAbilities(player, true)) {
                sa.setActivatingPlayer(player);
                if (sa.isManaAbility()) {
                    continue;
                }
                if (sa.isSpell()) {
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    spellPlays.add(sa);
                } else if (sa.isAbility()) {
                    if (sa.isEquip()) {
                        continue;
                    }
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    activatable.add(sa);
                }
            }
        }

        // 4. Exile: Foretell, Madness, and exile-activated abilities.
        CardCollectionView exile = player.getCardsIn(ZoneType.Exile);
        for (Card c : exile) {
            for (SpellAbility sa : c.getAllPossibleAbilities(player, true)) {
                sa.setActivatingPlayer(player);
                if (sa.isManaAbility()) {
                    continue;
                }
                if (sa.isSpell()) {
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    spellPlays.add(sa);
                } else if (sa.isAbility()) {
                    if (sa.isEquip()) {
                        continue;
                    }
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    activatable.add(sa);
                }
            }
        }

        // 5. Command zone: commanders.
        CardCollectionView command = player.getCardsIn(ZoneType.Command);
        for (Card c : command) {
            for (SpellAbility sa : c.getAllPossibleAbilities(player, true)) {
                sa.setActivatingPlayer(player);
                if (sa.isManaAbility()) {
                    continue;
                }
                if (sa.isSpell()) {
                    if (!hasDeterministicMana(sa)) {
                        continue;
                    }
                    if (sa.usesTargeting() && !ComputerUtilAbility.isFullyTargetable(sa)) {
                        continue;
                    }
                    spellPlays.add(sa);
                }
            }
        }

        // Re-sort since graveyard/exile/command spells were added after initial sort
        spellPlays.sort(Comparator.comparing(sa -> sa.getHostCard().getName()));

        // Rebuild combined list with updated spellPlays
        all.clear();
        all.addAll(landPlays);
        all.addAll(spellPlays);
        activatable.sort(Comparator
            .comparing((SpellAbility sa) -> sa.getHostCard().getName())
            .thenComparing(sa -> Objects.toString(sa.getApi(), ""))
            .thenComparing(sa -> Objects.toString(sa.getOriginalDescription(), "")));
        all.addAll(activatable);

        if (all.isEmpty()) {
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] pass empty%n", player.getId(),
                        getGame().getPhaseHandler().getTurn());
            }
            return null; // pass — no RNG consumed
        }

        final int idx;
        if (preferActions) {
            // Weighted random: each action has weight PREFER_ACTION_WEIGHT, pass has weight 1.
            // Matches Rust's DeterministicAgent::choose_action() prefer_actions branch.
            int totalWeight = all.size() * PREFER_ACTION_WEIGHT + 1;
            int roll = rng.nextInt(totalWeight);
            if (roll >= all.size() * PREFER_ACTION_WEIGHT) {
                idx = all.size(); // pass
            } else {
                idx = roll / PREFER_ACTION_WEIGHT;
            }
        } else {
            idx = rng.nextInt(all.size() + 1);
        }
        if (DEBUG_ACTIONS) {
            List<String> opts = new ArrayList<>();
            for (SpellAbility sa : all) {
                String kind = sa.isLandAbility() ? "LAND" : (sa.isSpell() ? "SPELL" : "AB");
                String fbTag = sa.isFlashback() ? "[FB]" : "";
                opts.add(kind + ":" + sa.getHostCard().getName() + fbTag);
            }
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

        // Always provide a chooseTargets callback that walks the entire ability
        // chain (including Charm sub-abilities chained by CharmEffect.makeChoices).
        // Previously, Charm spells fell through to super.playChosenSpellAbility()
        // because sa.usesTargeting() is false for the top-level CharmEffect SA.
        // This left sub-ability targets unset, causing the spell to get stuck in
        // the Stack zone (invisible to snapshots).
        Runnable chooseTargets = () -> {
            SpellAbility current = sa;
            while (current != null) {
                if (current.usesTargeting() && !current.isTargetNumberValid()) {
                    setupDeterministicTargets(current);
                }
                current = current.getSubAbility();
            }
        };
        return ComputerUtil.handlePlayingSpellAbility(player, sa, getGame(), chooseTargets);
    }

    // ── Combat ────────────────────────────────────────────────────────

    @Override
    public void declareAttackers(Player attacker, Combat combat) {
        // PhaseHandler may re-prompt attack declaration after invalid selections
        // or unpaid attack costs; always rebuild from an empty declaration.
        combat.clearAttackers();

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
            if (!CombatUtil.canAttack(c, defender)) continue;
            int roll = rng.nextInt(2);
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] atk roll %s -> %d%n",
                    player.getId(), getGame().getPhaseHandler().getTurn(), c.getName(), roll);
            }
            if (roll == 1) {
                combat.addAttacker(c, defender);
            }
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
        // Sort eligible blockers alphabetically.
        // For each: rng.nextInt(attacker_count + 1). 0 = don't block, k = block attacker[k-1].
        CardCollection blockers = new CardCollection(defender.getCreaturesInPlay());
        blockers.sort(Comparator.comparing(Card::getName));

        // Get sorted attackers
        List<Card> attackers = new ArrayList<>(combat.getAttackers());
        attackers.sort(Comparator.comparing(Card::getName));

        if (attackers.isEmpty()) return;

        // Parity with Rust deterministic agent:
        // consume blocker RNG only for blockers that are legal *at this moment*
        // and can block at least one current attacker. This must be evaluated
        // incrementally because combat state (e.g. block caps) can change after
        // each accepted blocker assignment.
        for (Card blocker : blockers) {
            if (!CombatUtil.canBlock(blocker, combat)) {
                continue;
            }
            boolean canBlockAny = false;
            for (Card attacker : attackers) {
                if (CombatUtil.canBlock(attacker, blocker, combat)) {
                    canBlockAny = true;
                    break;
                }
            }
            if (!canBlockAny) {
                continue;
            }

            int choice = rng.nextInt(attackers.size() + 1);
            if (DEBUG_ACTIONS) {
                System.err.printf("[det-java p%d t%d] blk roll %s -> %d/%d%n",
                    player.getId(), getGame().getPhaseHandler().getTurn(),
                    blocker.getName(), choice, attackers.size());
            }
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

    // ── Sacrifice / Destroy ────────────────────────────────────────────
    // Rust's choose_sacrifice sorts alphabetically by name, picks first.

    @Override
    public CardCollectionView choosePermanentsToSacrifice(SpellAbility sa, int min, int max,
            CardCollectionView validTargets, String message) {
        CardCollection sorted = new CardCollection(validTargets);
        sorted.sort(Comparator.comparing(Card::getName));
        int count = Math.min(Math.max(min, 1), sorted.size());
        return new CardCollection(sorted.subList(0, count));
    }

    @Override
    public CardCollectionView choosePermanentsToDestroy(SpellAbility sa, int min, int max,
            CardCollectionView validTargets, String message) {
        CardCollection sorted = new CardCollection(validTargets);
        sorted.sort(Comparator.comparing(Card::getName));
        int count = Math.min(Math.max(min, 1), sorted.size());
        return new CardCollection(sorted.subList(0, count));
    }

    // ── Zone Change (Search/Tutor) ──────────────────────────────────

    @Override
    public Card chooseSingleCardForZoneChange(ZoneType destination, List<ZoneType> origin,
            SpellAbility sa, CardCollection fetchList, DelayedReveal delayedReveal,
            String selectPrompt, boolean isOptional, Player decider) {
        if (delayedReveal != null) reveal(delayedReveal);
        if (fetchList == null || fetchList.isEmpty()) return null;
        // Deterministic: sort alphabetically and pick first. No RNG consumed.
        // Bypasses AI logic (basicManaFixing, CardLists.shuffle, etc.) to match Rust.
        List<Card> sorted = new ArrayList<>(fetchList);
        sorted.sort(Comparator.comparing(Card::getName));
        return sorted.get(0);
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

    // ── Scry / Surveil / Library Manipulation ───────────────────────

    @Override
    public ImmutablePair<CardCollection, CardCollection> arrangeForScry(CardCollection topN) {
        // Fixed: keep all on top, nothing to bottom (no RNG consumed)
        return ImmutablePair.of(topN, new CardCollection());
    }

    @Override
    public ImmutablePair<CardCollection, CardCollection> arrangeForSurveil(CardCollection topN) {
        // Fixed: keep all on top, nothing to graveyard — matches Rust's choose_surveil default.
        return ImmutablePair.of(topN, new CardCollection());
    }

    @Override
    public CardCollectionView orderMoveToZoneList(CardCollectionView cards, ZoneType destinationZone,
            SpellAbility source) {
        // Fixed: keep original order (no RNG consumed)
        return cards;
    }

    // ── Charm / Modal ────────────────────────────────────────────────

    @Override
    public List<AbilitySub> chooseModeForAbility(SpellAbility sa, List<AbilitySub> possible, int min, int num, boolean allowRepeat) {
        // Fixed: always pick first `min` modes in declaration order (no RNG consumed).
        // Matches Rust's default choose_mode which returns (0..min).
        if (possible == null || possible.isEmpty()) return new ArrayList<>();
        int count = Math.min(min, possible.size());
        return new ArrayList<>(possible.subList(0, count));
    }

    // ── Confirmations ─────────────────────────────────────────────────
    // NOTE: confirmAction is intentionally NOT fully overridden.
    // The parent PlayerControllerAi delegates to the AI brain (getAi()) which
    // has smart logic to decline optional actions that could cause
    // infinite loops. Blindly returning true for all confirmations caused the
    // Java engine to get stuck in infinite trigger/priority loops (e.g. when
    // Standard Bearer's Flagbearer ability interacted with certain board states).
    //
    // confirmAction exception: RearrangeTopOfLibrary (Ponder) must decline shuffle.
    // confirmTrigger: matches Rust's choose_optional_trigger (decline Pump/PumpAll).

    @Override
    public boolean confirmTrigger(WrappedAbility sa) {
        // Match Rust's choose_optional_trigger: decline Pump/PumpAll to avoid
        // infinite loops from repeated +0/+0 triggers, accept everything else.
        ApiType api = sa.getApi();
        if (api == ApiType.Pump || api == ApiType.PumpAll) {
            return false;
        }
        return true;
    }

    @Override
    public boolean confirmAction(SpellAbility sa, PlayerActionConfirmMode mode, String message,
            List<String> options, Card cardToShow, Map<String, Object> params) {
        // Decline shuffle for RearrangeTopOfLibrary (Ponder-like effects).
        // The shuffle uses game-level RNG (MyRandom) which is NOT synchronized
        // between Java and Rust engines, causing library order divergence.
        // Rust's choose_may_shuffle() also returns false (never shuffle).
        if (sa != null && sa.getApi() == ApiType.RearrangeTopOfLibrary) {
            return false;
        }
        // Delegate to parent AI brain for all other confirmations.
        // This avoids infinite loops from blindly accepting optional effects.
        return super.confirmAction(sa, mode, message, options, cardToShow, params);
    }

    // ── Additional Costs (Kicker, Buyback, Multikicker, Replicate) ────
    // Rust defaults: choose_kicker→false, choose_buyback→false,
    // choose_multikicker→0, choose_replicate→0.
    // We must match by never paying optional costs.

    @Override
    public List<OptionalCostValue> chooseOptionalCosts(SpellAbility chosen,
            List<OptionalCostValue> optionalCostValues) {
        // Never pay kicker, buyback, or any other optional cost.
        return Collections.emptyList();
    }

    @Override
    public int chooseNumberForKeywordCost(SpellAbility sa, Cost cost,
            KeywordInterface keyword, String prompt, int max) {
        // Never pay multikicker, replicate, or any repeated keyword cost.
        return 0;
    }

    // ── X-Cost ────────────────────────────────────────────────────────
    // Rust default choose_x_value returns max_x (spend all available mana).

    @Override
    public Integer announceRequirements(SpellAbility ability, String announce) {
        if ("X".equals(announce)) {
            int maxX = ComputerUtilCost.getMaxXValue(ability, player, ability.isTrigger());
            return Math.max(maxX, 0);
        }
        return super.announceRequirements(ability, announce);
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

    // ── Type / Card Name / Number Selection ────────────────────────────
    // Rust defaults: first valid type, first valid name, min value.

    @Override
    public String chooseSomeType(String kindOfType, SpellAbility sa, Collection<String> validTypes, boolean isOptional) {
        if (validTypes == null || validTypes.isEmpty()) return "";
        List<String> sorted = new ArrayList<>(validTypes);
        Collections.sort(sorted);
        return sorted.get(0);
    }

    @Override
    public String chooseCardName(SpellAbility sa, Predicate<ICardFace> cpp, String valid, String message) {
        // Delegate to parent AI — no simple "first" since candidates come from full card DB.
        return super.chooseCardName(sa, cpp, valid, message);
    }

    @Override
    public String chooseCardName(SpellAbility sa, List<ICardFace> faces, String message) {
        if (faces == null || faces.isEmpty()) return "";
        List<ICardFace> sorted = new ArrayList<>(faces);
        sorted.sort(Comparator.comparing(ICardFace::getName));
        return sorted.get(0).getName();
    }

    @Override
    public int chooseNumber(SpellAbility sa, String title, int min, int max) {
        return min;
    }

    @Override
    public int chooseNumber(SpellAbility sa, String title, List<Integer> values, Player relatedPlayer) {
        if (values == null || values.isEmpty()) return 0;
        return Collections.min(values);
    }

    @Override
    public int chooseNumberForCostReduction(final SpellAbility sa, final int min, final int max) {
        return min;
    }

    // ── Coin Flip ─────────────────────────────────────────────────────
    // Rust default flip_coin_call returns true (always call heads).

    @Override
    public boolean chooseFlipResult(SpellAbility sa, Player flipper, boolean[] results, boolean call) {
        return true; // always call heads
    }

    // ── Mulligan Bottom Selection ────────────────────────────────────
    // Rust default choose_cards_to_bottom returns first N cards.

    @Override
    public CardCollectionView tuckCardsViaMulligan(Player mulliganingPlayer, int cardsToReturn) {
        CardCollectionView hand = mulliganingPlayer.getCardsIn(ZoneType.Hand);
        CardCollection sorted = new CardCollection(hand);
        sorted.sort(Comparator.comparing(Card::getName));
        return new CardCollection(sorted.subList(0, Math.min(cardsToReturn, sorted.size())));
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
     * Conservative, side-effect-free mana gate.
     * Prevents selecting actions that are obviously unpayable with current untapped mana sources.
     */
    private boolean hasDeterministicMana(SpellAbility sa) {
        final Cost costs = sa.getPayCosts();
        if (costs == null || !costs.hasManaCost()) {
            return true;
        }

        final ManaCost manaCost = costs.getCostMana().getManaCostFor(sa);
        if (manaCost == null || manaCost.isNoCost()) {
            return false;
        }

        // Conservative rule for deterministic probing:
        // non-mana activations should not assume the source can also fund its own mana cost.
        final boolean excludesSource = costs.hasTapCost()
            || (sa.isAbility() && !sa.isManaAbility() && sa.getHostCard().isInPlay());
        final Card source = sa.getHostCard();
        final int sourceId = source.getId();
        int totalSources = 0;
        int w = 0, u = 0, b = 0, r = 0, g = 0, c = 0;

        for (Card card : player.getCardsIn(ZoneType.Battlefield)) {
            if (card.isTapped()) {
                continue;
            }
            // Summoning-sick creatures cannot activate {T} abilities (including mana).
            // Must match Rust's calculate_available_mana() check so castability probes
            // agree with actual payment and neither engine wastes RNG on uncastable spells.
            if (card.isSick()) {
                boolean allNeedTap = true;
                for (SpellAbility manaSa : card.getManaAbilities()) {
                    if (!manaSa.getPayCosts().hasTapCost()) {
                        allNeedTap = false;
                        break;
                    }
                }
                if (allNeedTap) {
                    continue;
                }
            }
            if (excludesSource && card.getId() == sourceId) {
                continue;
            }
            boolean canProduceW = false;
            boolean canProduceU = false;
            boolean canProduceB = false;
            boolean canProduceR = false;
            boolean canProduceG = false;
            boolean canProduceC = false;

            for (SpellAbility manaSa : card.getManaAbilities()) {
                if (manaSa.getManaPart() == null) {
                    continue;
                }
                final String produced = Objects.toString(manaSa.getManaPart().getOrigProduced(), "");
                final String upper = produced.toUpperCase(Locale.ROOT);
                if (upper.contains("W")) canProduceW = true;
                if (upper.contains("U")) canProduceU = true;
                if (upper.contains("B")) canProduceB = true;
                if (upper.contains("R")) canProduceR = true;
                if (upper.contains("G")) canProduceG = true;
                if (upper.contains("C")) canProduceC = true;
                if (upper.contains("ANY")) {
                    canProduceW = canProduceU = canProduceB = canProduceR = canProduceG = canProduceC = true;
                }
            }

            if (canProduceW || canProduceU || canProduceB || canProduceR || canProduceG || canProduceC) {
                totalSources++;
                if (canProduceW) w++;
                if (canProduceU) u++;
                if (canProduceB) b++;
                if (canProduceR) r++;
                if (canProduceG) g++;
                if (canProduceC) c++;
            }
        }

        // Also count mana already floating in the pool (from rituals like Dark Ritual).
        forge.game.mana.ManaPool pool = player.getManaPool();
        if (!pool.isEmpty()) {
            totalSources += pool.totalMana();
            w += pool.getAmountOfColor(MagicColor.WHITE);
            u += pool.getAmountOfColor(MagicColor.BLUE);
            b += pool.getAmountOfColor(MagicColor.BLACK);
            r += pool.getAmountOfColor(MagicColor.RED);
            g += pool.getAmountOfColor(MagicColor.GREEN);
            c += pool.getAmountOfColor((byte) 0); // colorless
        }

        if (totalSources < manaCost.getCMC()) {
            return false;
        }

        final int[] required = manaCost.getColorShardCounts(); // W U B R G C
        return w >= required[0]
            && u >= required[1]
            && b >= required[2]
            && r >= required[3]
            && g >= required[4]
            && c >= required[5];
    }

    /**
     * Set up deterministic targets for a spell ability.
     * Sorts all candidate targets alphabetically, then picks randomly.
     */
    private void setupDeterministicTargets(SpellAbility sa) {
        sa.resetTargets();

        // 1. Check for spell targets on the stack (counterspells, redirects, etc.)
        //    Rust's choose_target_spell returns the first (topmost) valid spell.
        List<SpellAbility> spellCandidates = new ArrayList<>();
        for (SpellAbilityStackInstance si : getGame().getStack()) {
            SpellAbility abilityOnStack = si.getSpellAbility();
            if (sa.canTargetSpellAbility(abilityOnStack)) {
                spellCandidates.add(abilityOnStack);
            }
        }
        if (!spellCandidates.isEmpty()) {
            // Pick the first (topmost) targetable spell — matches Rust default.
            sa.getTargets().add(spellCandidates.get(0));
            return;
        }

        // 2. Build unified candidate list: players first (by name), then cards (by name)
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
