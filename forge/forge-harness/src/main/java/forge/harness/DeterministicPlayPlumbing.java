package forge.harness;

import forge.ai.ComputerUtilCost;
import forge.game.Game;
import forge.game.GameActionUtil;
import forge.game.ability.AbilityUtils;
import forge.game.ability.ApiType;
import forge.game.ability.effects.CharmEffect;
import forge.game.card.Card;
import forge.game.cost.Cost;
import forge.game.cost.CostPayment;
import forge.game.player.Player;
import forge.game.spellability.Spell;
import forge.game.spellability.SpellAbility;
import forge.game.spellability.TargetChoices;
import forge.game.zone.Zone;
import forge.game.zone.ZoneType;

import java.util.List;

/**
 * Deterministic cast/payment plumbing extracted from {@link DeterministicController}.
 *
 * Source references mirrored with minimal changes:
 * - forge/forge-ai/src/main/java/forge/ai/ComputerUtil.java (playNoStack/playStack)
 * - forge/forge-ai/src/main/java/forge/ai/PlayerControllerAi.java
 */
final class DeterministicPlayPlumbing {
    private final DeterministicController controller;
    private final Player payer;
    private final DeterministicCostPlumbing costPlumbing;

    DeterministicPlayPlumbing(
            final DeterministicController controller,
            final Player payer,
            final DeterministicCostPlumbing costPlumbing
    ) {
        this.controller = controller;
        this.payer = payer;
        this.costPlumbing = costPlumbing;
    }

    boolean playNoStackDeterministic(final Player ai, SpellAbility sa, final Game game, final boolean effect) {
        sa.setActivatingPlayer(ai);
        if (!ComputerUtilCost.canPayCost(sa, ai, effect)) {
            return false;
        }

        final Card source = sa.getHostCard();
        if (!effect && sa.isSpell() && !source.isCopiedSpell()) {
            sa.setHostCard(game.getAction().moveToStack(source, sa));
            sa = GameActionUtil.addExtraKeywordCost(sa);
        }

        final Cost cost = sa.getPayCosts();
        if (costPlumbing.payWithDeterministicDecision(cost, sa, effect)) {
            AbilityUtils.resolve(sa);
            return true;
        }

        return false;
    }

    boolean playStackDeterministic(SpellAbility sa, final Player ai, final Game game) {
        sa.setActivatingPlayer(ai);
        if (!ComputerUtilCost.canPayCost(sa, ai, false)) {
            return false;
        }

        final Card source = sa.getHostCard();
        final Zone fromZone = game.getZoneOf(source);
        final int zonePosition = fromZone != null ? fromZone.getCards().indexOf(source) : 0;

        if (sa.isSpell() && !source.isCopiedSpell()) {
            sa.setHostCard(game.getAction().moveToStack(source, sa));
            sa = GameActionUtil.addExtraKeywordCost(sa);
        }

        final Cost cost = sa.getPayCosts();
        final CostPayment pay = new CostPayment(cost, sa);

        if (!sa.checkRestrictions(ai)) {
            GameActionUtil.rollbackAbility(sa, fromZone, zonePosition, pay, source);
            return false;
        }

        if (costPlumbing.payWithDeterministicDecision(cost, sa, false)) {
            game.getStack().add(sa);
            return true;
        }
        return false;
    }

    boolean handlePlayingSpellAbilityDeterministic(final Player ai, SpellAbility sa, final Game game) {
        final Card source = sa.getHostCard();
        final Card host = sa.getHostCard();
        final Zone hz = host.isCopiedSpell() ? null : host.getZone();
        source.setSplitStateToPlayAbility(sa);

        if (sa.isSpell() && !source.isCopiedSpell()) {
            sa = AbilityUtils.addSpliceEffects(sa);
            if (sa.getSplicedCards() != null && !sa.getSplicedCards().isEmpty()) {
                sa.resetTargets();
                if (!sa.setupTargets()) {
                    return false;
                }
            }
        }

        if (!sa.isCopied()) {
            sa.resetPaidHash();
            sa.setPaidLife(0);
        }

        if (sa.getApi() == ApiType.Charm && !CharmEffect.makeChoices(sa)) {
            return false;
        }

        if (sa.isSpell() && !source.isCopiedSpell()) {
            sa.setHostCard(game.getAction().moveToStack(source, sa));
        }

        sa = GameActionUtil.addExtraKeywordCost(sa);

        // Mirror HumanPlaySpellAbility pre-cost prerequisites:
        // targeting setup is evaluated directly, not gated by isTargetNumberValid().
        if (!sa.setupTargets()) {
            return false;
        }

        final Cost cost = sa.getPayCosts();
        game.getStack().freezeStack(sa);

        if (costPlumbing.payWithDeterministicDecision(cost, sa, false)) {
            game.getStack().addAndUnfreeze(sa);
            if (sa.getSplicedCards() != null && !sa.getSplicedCards().isEmpty()) {
                game.getAction().reveal(sa.getSplicedCards(), ai, true, "Computer reveals spliced cards from ");
            }
            return true;
        }

        System.out.println("[" + sa.getActivatingPlayer() + "] AI failed to play "
                + sa.getHostCard() + " [" + sa.getHostCard().getZone() + "]");
        sa.setSkip(true);
        if (host != null && hz != null && hz.is(ZoneType.Stack)) {
            final Card c = game.getAction().moveTo(hz.getZoneType(), host, null, null);
            for (SpellAbility csa : c.getSpellAbilities()) {
                csa.setSkip(true);
            }
        }
        return false;
    }

    boolean prepareSingleSaDeterministic(final Card host, final SpellAbility sa, final boolean isMandatory) {
        if (sa.getApi() == ApiType.Charm) {
            if (!CharmEffect.makeChoices(sa)) {
                return false;
            }
        }
        // Deterministic harness must not route trigger target setup through AI
        // heuristics (`AiController#doTrigger`). Mirror Java SpellAbility setup flow
        // so all target prompts are resolved by controller callbacks.
        return sa.setupTargets();
    }

    void orderAndPlaySimultaneousSa(List<SpellAbility> activePlayerSAs, final Game game) {
        for (final SpellAbility sa : activePlayerSAs) {
            if (sa.isTrigger() && !sa.isCopied()) {
                if (prepareSingleSaDeterministic(sa.getHostCard(), sa, true)) {
                    playStackDeterministic(sa, payer, game);
                }
            } else {
                if (sa.isCopied()) {
                    if (sa.isSpell()) {
                        if (!sa.getHostCard().isInZone(ZoneType.Stack)) {
                            sa.setHostCard(game.getAction().moveToStack(sa.getHostCard(), sa));
                        } else {
                            game.getStackZone().add(sa.getHostCard());
                        }
                    }

                    if (sa.isMayChooseNewTargets()) {
                        final TargetChoices tc = sa.getTargets();
                        if (!sa.setupTargets()) {
                            sa.setTargets(tc);
                        }
                    }
                }
                game.getStack().add(sa);
            }
        }
    }

    boolean playSaFromPlayEffect(SpellAbility tgtSA, final Game game) {
        final boolean optional = !tgtSA.getPayCosts().isMandatory();
        if (tgtSA instanceof Spell) {
            if (optional && !controller.chooseDeterministicBoolean("play_effect_optional", "DECLINE", "ACCEPT")) {
                return false;
            }
            return playStackDeterministic(tgtSA, payer, game);
        }
        return true;
    }
}
