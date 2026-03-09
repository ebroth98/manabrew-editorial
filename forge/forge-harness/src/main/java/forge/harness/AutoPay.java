package forge.harness;

import forge.card.mana.ManaAtom;
import forge.card.mana.ManaCost;
import forge.card.mana.ManaCostShard;
import forge.game.card.Card;
import forge.game.card.CardCollectionView;
import forge.game.cost.Cost;
import forge.game.cost.CostPayment;
import forge.game.mana.Mana;
import forge.game.mana.ManaCostBeingPaid;
import forge.game.mana.ManaPool;
import forge.game.player.Player;
import forge.game.spellability.AbilityManaPart;
import forge.game.spellability.SpellAbility;
import forge.game.zone.ZoneType;

import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

/**
 * Deterministic, legality-first mana auto-payment for harness parity.
 *
 * <p>Design goals:
 * - Never depend on Java AI controller classes (`PlayerControllerAi`, `AiCostDecision`).
 * - Pay only legal costs using engine legality checks.
 * - Keep deterministic behavior: no random selection inside auto-pay.
 * - Pay required costs first; optional decisions are delegated to controller callbacks only
 *   when the engine asks for them while paying a selected ability cost.
 */
final class AutoPay {
    private final Player payer;
    private final DeterministicCostPlumbing costPlumbing;

    AutoPay(final Player payer, final DeterministicCostPlumbing costPlumbing) {
        this.payer = payer;
        this.costPlumbing = costPlumbing;
    }

    boolean payManaCost(final ManaCost toPay, final SpellAbility saBeingPaid, final boolean effect) {
        final ManaCostBeingPaid unpaid = new ManaCostBeingPaid(toPay);
        final ManaPool pool = payer.getManaPool();
        final List<Mana> spentFromPool = new ArrayList<>();

        // Consume already-floating mana first.
        if (pool.payManaCostFromPool(unpaid, saBeingPaid, false, spentFromPool)) {
            CostPayment.handleOfferings(saBeingPaid, false, true);
            return true;
        }

        int guard = 0;
        while (!unpaid.isPaid() && guard++ < 128) {
            final List<ManaAbilityCandidate> candidates = collectPlayableManaAbilities(saBeingPaid);
            if (candidates.isEmpty()) {
                break;
            }

            final ManaAbilityCandidate chosen = chooseCandidate(unpaid, candidates);
            if (chosen == null) {
                break;
            }

            if (!payAbilityActivationCosts(chosen.spellAbility, effect)) {
                // Candidate became unpayable after state changes; continue with remaining choices.
                candidates.remove(chosen);
                continue;
            }

            // Mana abilities resolve immediately in engine flow.
            payer.getGame().getStack().addAndUnfreeze(chosen.spellAbility);

            // Spend produced mana against the unpaid cost, then consume any matching floating mana.
            pool.payManaFromAbility(saBeingPaid, unpaid, chosen.spellAbility);
            pool.payManaCostFromPool(unpaid, saBeingPaid, false, spentFromPool);
        }

        final boolean paid = unpaid.isPaid();
        CostPayment.handleOfferings(saBeingPaid, false, paid);
        if (!paid) {
            pool.refundMana(spentFromPool);
            saBeingPaid.setSkip(true);
        }
        return paid;
    }

    private boolean payAbilityActivationCosts(final SpellAbility manaAbility, final boolean effect) {
        final Cost paymentCost = manaAbility.getPayCosts();
        if (paymentCost == null || paymentCost.getCostParts().isEmpty()) {
            return true;
        }
        return costPlumbing.payWithDeterministicDecision(paymentCost, manaAbility, effect);
    }

    private ManaAbilityCandidate chooseCandidate(
            final ManaCostBeingPaid unpaid,
            final List<ManaAbilityCandidate> candidates
    ) {
        for (final ManaCostShard shard : shardPriority(unpaid)) {
            final ManaAbilityCandidate match = chooseFirstPayingShard(candidates, shard);
            if (match != null) {
                configureExpressChoiceForShard(match.spellAbility, shard);
                return match;
            }
        }
        return null;
    }

    private List<ManaCostShard> shardPriority(final ManaCostBeingPaid unpaid) {
        final List<ManaCostShard> out = new ArrayList<>();
        final Set<ManaCostShard> seen = new LinkedHashSet<>();
        for (final ManaCostShard shard : unpaid.getUnpaidShards()) {
            if (shard == ManaCostShard.X) {
                continue;
            }
            if (seen.add(shard)) {
                out.add(shard);
            }
        }
        if (!seen.contains(ManaCostShard.GENERIC)) {
            out.add(ManaCostShard.GENERIC);
        }
        return out;
    }

    private ManaAbilityCandidate chooseFirstPayingShard(
            final List<ManaAbilityCandidate> candidates,
            final ManaCostShard shard
    ) {
        for (final ManaAbilityCandidate c : candidates) {
            if (canPayShard(c.spellAbility, shard)) {
                return c;
            }
        }
        return null;
    }

    private boolean canPayShard(final SpellAbility manaAbility, final ManaCostShard shard) {
        if (shard == ManaCostShard.X) {
            return false;
        }
        final AbilityManaPart manaPart = manaAbility.getManaPart();
        if (manaPart == null) {
            return false;
        }
        if (!manaPart.meetsManaRestrictions(manaAbility)) {
            return false;
        }
        if (shard == ManaCostShard.GENERIC) {
            return true;
        }
        final Card source = manaAbility.getHostCard();
        for (final byte atom : producedAtoms(manaAbility)) {
            if (!manaAbility.allowsPayingWithShard(source, atom)) {
                continue;
            }
            if (payer.getManaPool().canPayForShardWithColor(shard, atom)) {
                return true;
            }
        }
        return false;
    }

    private void configureExpressChoiceForShard(final SpellAbility manaAbility, final ManaCostShard shard) {
        final AbilityManaPart manaPart = manaAbility.getManaPart();
        if (manaPart == null) {
            return;
        }
        if (!manaPart.isAnyMana() && !manaPart.isComboMana()) {
            return;
        }
        final byte preferred = preferredColorForShard(shard, producedAtoms(manaAbility));
        if (preferred != 0) {
            manaPart.setExpressChoice(shortColor(preferred));
        }
    }

    private byte preferredColorForShard(final ManaCostShard shard, final List<Byte> produced) {
        for (final byte atom : produced) {
            if (payer.getManaPool().canPayForShardWithColor(shard, atom)) {
                return atom;
            }
        }
        return produced.isEmpty() ? 0 : produced.get(0);
    }

    private List<ManaAbilityCandidate> collectPlayableManaAbilities(final SpellAbility saBeingPaid) {
        final List<ManaAbilityCandidate> out = new ArrayList<>();
        final CardCollectionView battlefield = payer.getCardsIn(ZoneType.Battlefield);

        int sourceOrder = 0;
        for (final Card source : battlefield) {
            for (final SpellAbility manaAbility : source.getManaAbilities()) {
                if (!manaAbility.isManaAbility()) {
                    continue;
                }
                if (manaAbility.getPayCosts() != null && manaAbility.getPayCosts().hasManaCost()) {
                    // Keep auto-pay non-recursive and deterministic.
                    continue;
                }
                manaAbility.setActivatingPlayer(payer);
                if (!manaAbility.canPlay() || !manaAbility.checkRestrictions(payer)) {
                    continue;
                }
                if (manaAbility.getManaPart() == null) {
                    continue;
                }
                if (!manaAbility.getManaPart().meetsManaRestrictions(saBeingPaid)) {
                    continue;
                }
                if (manaAbility.getPayCosts() != null
                        && !CostPayment.canPayAdditionalCosts(manaAbility.getPayCosts(), manaAbility, false, payer)) {
                    continue;
                }
                out.add(new ManaAbilityCandidate(manaAbility, sourceOrder++));
            }
        }
        return out;
    }

    private List<Byte> producedAtoms(final SpellAbility manaAbility) {
        final List<Byte> out = new ArrayList<>();
        final AbilityManaPart manaPart = manaAbility.getManaPart();
        if (manaPart == null) {
            return out;
        }
        final String produced = manaPart.mana(manaAbility);
        for (final String token : produced.split(" ")) {
            final String t = token.trim();
            if (t.isEmpty()) {
                continue;
            }
            if ("Any".equalsIgnoreCase(t)) {
                addDistinct(out, (byte) ManaAtom.WHITE);
                addDistinct(out, (byte) ManaAtom.BLUE);
                addDistinct(out, (byte) ManaAtom.BLACK);
                addDistinct(out, (byte) ManaAtom.RED);
                addDistinct(out, (byte) ManaAtom.GREEN);
                continue;
            }
            final byte atom = ManaAtom.fromName(t);
            if (atom != 0) {
                addDistinct(out, atom);
            }
        }
        return out;
    }

    private static void addDistinct(final List<Byte> atoms, final byte atom) {
        if (!atoms.contains(atom)) {
            atoms.add(atom);
        }
    }

    private static String shortColor(final byte atom) {
        if (atom == ManaAtom.WHITE) return "W";
        if (atom == ManaAtom.BLUE) return "U";
        if (atom == ManaAtom.BLACK) return "B";
        if (atom == ManaAtom.RED) return "R";
        if (atom == ManaAtom.GREEN) return "G";
        return "C";
    }

    private static final class ManaAbilityCandidate {
        private final SpellAbility spellAbility;
        @SuppressWarnings("unused")
        private final int sourceOrder;

        private ManaAbilityCandidate(final SpellAbility spellAbility, final int sourceOrder) {
            this.spellAbility = spellAbility;
            this.sourceOrder = sourceOrder;
        }
    }
}
