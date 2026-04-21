package forge.harness;

import com.google.common.collect.Lists;
import forge.LobbyPlayer;
import forge.game.Game;
import forge.game.GameEntity;
import forge.game.card.Card;
import forge.game.card.CardCollection;
import forge.game.combat.Combat;
import forge.game.player.Player;
import forge.game.spellability.SpellAbility;
import forge.game.zone.ZoneType;

import java.util.ArrayList;
import java.util.List;

public final class OpenMagicInteractiveController extends DeterministicController {
    private final Game game;
    private final OpenMagicInteractiveSession session;

    public OpenMagicInteractiveController(
            final Game game,
            final Player player,
            final LobbyPlayer lobbyPlayer,
            final OpenMagicInteractiveSession session
    ) {
        super(game, player, lobbyPlayer, new CountingRandom(0), false, true);
        this.game = game;
        this.session = session;
    }

    @Override
    public boolean mulliganKeepHand(final Player mulliganingPlayer, final int cardsToReturn) {
        final boolean keep = session.awaitMulliganDecision(
                SnapshotExtractor.playerIndex(game, player),
                cardsToReturn);
        if (keep && cardsToReturn > 0) {
            final CardCollection hand = new CardCollection(player.getCardsIn(ZoneType.Hand));
            final CardCollection selected = session.awaitMulliganPutBack(
                    SnapshotExtractor.playerIndex(game, player),
                    hand,
                    cardsToReturn);
            for (final forge.game.card.Card card : selected) {
                game.getAction().moveToLibrary(card, -1, null);
            }
            onCallback("choose_cards_to_bottom", selected.toString(), String.valueOf(hand.size()),
                    String.valueOf(cardsToReturn));
        }
        onCallback("mulligan_decision", Boolean.toString(keep),
                String.valueOf(player.getCardsIn(ZoneType.Hand).size()),
                String.valueOf(cardsToReturn));
        return keep;
    }

    @Override
    public CardCollection tuckCardsViaMulligan(final Player mulliganingPlayer, final int cardsToReturn) {
        return new CardCollection();
    }

    @Override
    public void declareAttackers(final Player attacker, final Combat combat) {
        combat.clearAttackers();
        final List<Card> legalAttackers = ChoiceSpace.sortNative(
                CombatChoiceSpace.legalAttackers(attacker, combat),
                ParityOrder.cardComparator());
        final CardCollection selected = session.awaitAttackers(
                SnapshotExtractor.playerIndex(game, attacker),
                combat,
                legalAttackers);
        for (final Card card : selected) {
            List<GameEntity> defenders = CombatChoiceSpace.legalDefendersForAttacker(card, combat);
            defenders = ParityOrder.sortDefenders(defenders);
            if (!defenders.isEmpty()) {
                combat.addAttacker(card, defenders.get(0));
            }
        }
        onCallback("choose_attackers", selected.toString(), String.valueOf(legalAttackers.size()),
                String.valueOf(combat.getDefenders().size()));
    }

    @Override
    public List<SpellAbility> chooseSpellAbilityToPlay() {
        final List<SpellAbility> all = ChoiceSpace.sortNative(
                new ArrayList<>(ActionSpace.getPossibleActions(player)),
                ParityOrder.actionComparator());
        final SpellAbility selected = session.awaitPriorityAction(
                SnapshotExtractor.playerIndex(game, player), all);
        if (selected == null) {
            onCallback("choose_action", "PassPriority");
            return null;
        }
        onCallback("choose_action", selected.toString());
        return Lists.newArrayList(selected);
    }

    @Override
    public CardCollection chooseCardsToDiscardFrom(
            final Player playerDiscard,
            final SpellAbility sa,
            final CardCollection validCards,
            final int min,
            final int max
    ) {
        final CardCollection selected = session.awaitCardChoice(
                "choose_discard",
                SnapshotExtractor.playerIndex(game, player),
                validCards,
                min,
                max);
        onCallback("choose_discard", selected.toString(), String.valueOf(validCards.size()),
                String.valueOf(min), String.valueOf(max));
        return selected;
    }

    @Override
    public CardCollection chooseCardsToDiscardToMaximumHandSize(final int numDiscard) {
        final CardCollection hand = new CardCollection(player.getCardsIn(ZoneType.Hand));
        final CardCollection selected = session.awaitCardChoice(
                "choose_discard",
                SnapshotExtractor.playerIndex(game, player),
                hand,
                numDiscard,
                numDiscard);
        onCallback("choose_discard", selected.toString(), String.valueOf(hand.size()),
                String.valueOf(numDiscard), String.valueOf(numDiscard));
        return selected;
    }
}
