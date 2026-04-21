package forge.harness;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import forge.game.Game;
import forge.game.card.Card;
import forge.game.card.CounterEnumType;
import forge.game.card.CounterType;
import forge.game.player.Player;
import forge.game.zone.ZoneType;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;

/**
 * Adds UI-only ordered card data to the parity snapshot used by interactive sessions.
 */
public final class InteractiveSnapshotExtractor {
    private InteractiveSnapshotExtractor() {}

    private static final Gson GSON = new GsonBuilder().create();

    public static String snapshotJson(final Game game) {
        return GSON.toJson(extractSnapshot(game));
    }

    @SuppressWarnings("unchecked")
    public static Map<String, Object> extractSnapshot(final Game game) {
        final Map<String, Object> snapshot = SnapshotExtractor.extractSnapshot(game);
        final List<Map<String, Object>> players = new ArrayList<>();
        final Object basePlayers = snapshot.get("players");
        int index = 0;
        for (final Player player : game.getRegisteredPlayers()) {
            final Map<String, Object> basePlayer = basePlayers instanceof List && index < ((List<?>) basePlayers).size()
                    ? new LinkedHashMap<>((Map<String, Object>) ((List<?>) basePlayers).get(index))
                    : new LinkedHashMap<>();
            players.add(snapshotInteractivePlayer(game, player, basePlayer));
            index++;
        }
        snapshot.put("players", players);
        return snapshot;
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> snapshotInteractivePlayer(
            final Game game,
            final Player player,
            final Map<String, Object> basePlayer
    ) {
        final Map<String, Object> out = new LinkedHashMap<>(basePlayer);
        out.put("battlefield_cards", snapshotBattlefieldCards(game, player));
        out.put("hand_cards", snapshotZoneCards(player.getCardsIn(ZoneType.Hand)));
        out.put("command_zone", player.getCardsIn(ZoneType.Command).stream()
                .map(card -> normalizeCardName(card.getName()))
                .collect(Collectors.toList()));
        out.put("command_zone_cards", snapshotZoneCards(player.getCardsIn(ZoneType.Command)));
        return out;
    }

    private static List<Map<String, Object>> snapshotBattlefieldCards(final Game game, final Player player) {
        final List<Map<String, Object>> out = new ArrayList<>();
        for (final Card card : player.getCardsIn(ZoneType.Battlefield)) {
            out.add(snapshotBattlefieldCard(game, card));
        }
        return out;
    }

    private static Map<String, Object> snapshotBattlefieldCard(final Game game, final Card card) {
        final Map<String, Object> out = snapshotZoneCard(card);
        out.put("tapped", card.isTapped());
        out.put("power", card.isCreature() ? card.getNetPower() : null);
        out.put("toughness", card.isCreature() ? card.getNetToughness() : null);
        out.put("damage", card.getDamage());
        out.put("summoning_sick", card.hasSickness());

        final Map<String, Integer> counters = new TreeMap<>();
        for (final Map.Entry<CounterType, Integer> entry : card.getCounters().entrySet()) {
            if (entry.getValue() > 0) {
                counters.put(counterTypeName(entry.getKey()), entry.getValue());
            }
        }
        out.put("counters", counters);
        out.put("controller", SnapshotExtractor.playerIndex(game, card.getController()));
        return out;
    }

    private static List<Map<String, Object>> snapshotZoneCards(final Iterable<Card> cards) {
        final List<Map<String, Object>> out = new ArrayList<>();
        for (final Card card : cards) {
            out.add(snapshotZoneCard(card));
        }
        return out;
    }

    private static Map<String, Object> snapshotZoneCard(final Card card) {
        final Map<String, Object> out = new LinkedHashMap<>();
        out.put("id", SnapshotExtractor.javaCardId(card));
        out.put("name", normalizeCardName(card.getName()));
        return out;
    }

    private static String normalizeCardName(final String name) {
        if (name != null && name.startsWith("Troll of Khazad-d") && name.endsWith("m")) {
            return "Troll of Khazad-d\u00fbm";
        }
        return name;
    }

    private static String counterTypeName(final CounterType counterType) {
        if (counterType instanceof CounterEnumType) {
            final CounterEnumType counterEnumType = (CounterEnumType) counterType;
            switch (counterEnumType) {
                case P1P1: return "+1/+1";
                case M1M1: return "-1/-1";
                default: return counterEnumType.name().toLowerCase();
            }
        }
        return counterType.getName().toLowerCase();
    }
}
