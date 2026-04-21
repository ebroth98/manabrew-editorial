package forge.harness;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import forge.game.Game;
import forge.game.GameEntity;
import forge.game.Match;
import forge.game.card.Card;
import forge.game.card.CardCollection;
import forge.game.card.CardCollectionView;
import forge.game.combat.Combat;
import forge.game.player.Player;
import forge.game.spellability.SpellAbility;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.LinkedBlockingQueue;

public final class OpenMagicInteractiveSession {
    private final String sessionId;
    private Match match;
    private Game game;
    private final BlockingQueue<JsonObject> actions = new LinkedBlockingQueue<>();
    private volatile String latestPromptJson;
    private volatile boolean closed;
    private volatile Thread gameThread;

    OpenMagicInteractiveSession(final String sessionId) {
        this.sessionId = Objects.requireNonNull(sessionId, "sessionId");
    }

    void attach(final Match match, final Game game) {
        this.match = Objects.requireNonNull(match, "match");
        this.game = Objects.requireNonNull(game, "game");
    }

    public String getSessionId() {
        return sessionId;
    }

    public Game getGame() {
        requireAttached();
        return game;
    }

    public void start() {
        requireAttached();
        gameThread = new Thread(() -> {
            try {
                match.startGame(game);
            } catch (RuntimeException error) {
                System.err.println("[open-magic] interactive game error: " + error.getMessage());
                error.printStackTrace(System.err);
            }
        }, "open-magic-forge-" + sessionId);
        gameThread.setDaemon(true);
        gameThread.start();
    }

    public void close() {
        closed = true;
        JsonObject action = new JsonObject();
        action.addProperty("kind", "pass");
        actions.offer(action);
        if (game != null && !game.isGameOver()) {
            game.setGameOver(forge.game.GameEndReason.Draw);
        }
    }

    public String getLatestPromptJson() {
        return latestPromptJson;
    }

    public String getSnapshotJson() {
        requireAttached();
        return InteractiveSnapshotExtractor.snapshotJson(game);
    }

    public String submitAction(final String actionJson) {
        if (closed) {
            throw new IllegalStateException("session is closed");
        }
        JsonObject action = JsonParser.parseString(actionJson).getAsJsonObject();
        actions.offer(action);
        return getSnapshotJson();
    }

    SpellAbility awaitPriorityAction(final int playerId, final List<SpellAbility> actionsForPrompt) {
        requireAttached();
        publishPriorityPrompt(playerId, actionsForPrompt);
        while (!closed && !game.isGameOver()) {
            final JsonObject action;
            try {
                action = actions.take();
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                return null;
            }
            final String kind = action.has("kind") ? action.get("kind").getAsString() : "";
            if ("pass".equals(kind) || "pass_priority".equals(kind)) {
                return null;
            }
            if ("choose_action".equals(kind)) {
                final int index = action.get("index").getAsInt();
                if (index < 0 || index >= actionsForPrompt.size()) {
                    throw new IllegalArgumentException("action index out of range: " + index);
                }
                return actionsForPrompt.get(index);
            }
            throw new UnsupportedOperationException("unsupported action kind: " + kind);
        }
        return null;
    }

    boolean awaitMulliganDecision(final int playerId, final int cardsToReturn) {
        requireAttached();
        final List<Card> cards = new ArrayList<Card>(
                game.getPlayers().get(playerId).getCardsIn(forge.game.zone.ZoneType.Hand));
        publishCardChoicePrompt("mulligan", playerId, cards, 0, 0, cardsToReturn);
        while (!closed && !game.isGameOver()) {
            final JsonObject action;
            try {
                action = actions.take();
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                return true;
            }
            final String actionKind = action.has("kind") ? action.get("kind").getAsString() : "";
            if ("mulligan_decision".equals(actionKind)) {
                return action.has("keep") && action.get("keep").getAsBoolean();
            }
            if ("pass".equals(actionKind) || "pass_priority".equals(actionKind)) {
                return true;
            }
            throw new UnsupportedOperationException("unsupported action kind: " + actionKind);
        }
        return true;
    }

    CardCollection awaitMulliganPutBack(final int playerId, final CardCollectionView hand, final int count) {
        requireAttached();
        if (count <= 0) {
            return new CardCollection();
        }
        final List<Card> cards = new ArrayList<Card>(hand);
        publishCardChoicePrompt("mulligan_put_back", playerId, cards, count, count, count);
        return awaitCardsFromPublishedPrompt(cards, count, count);
    }

    CardCollection awaitAttackers(
            final int playerId,
            final Combat combat,
            final List<Card> availableAttackers
    ) {
        requireAttached();
        publishAttackersPrompt(playerId, combat, availableAttackers);
        while (!closed && !game.isGameOver()) {
            final JsonObject action;
            try {
                action = actions.take();
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                return new CardCollection();
            }
            final String actionKind = action.has("kind") ? action.get("kind").getAsString() : "";
            if ("pass".equals(actionKind) || "pass_priority".equals(actionKind)) {
                return new CardCollection();
            }
            if (!"declare_attackers".equals(actionKind)) {
                throw new UnsupportedOperationException("unsupported action kind: " + actionKind);
            }
            final CardCollection selected = new CardCollection();
            if (action.has("assignments") && action.get("assignments").isJsonArray()) {
                for (JsonElement element : action.getAsJsonArray("assignments")) {
                    if (!element.isJsonObject()) {
                        continue;
                    }
                    final JsonObject assignment = element.getAsJsonObject();
                    final String attackerId = assignment.has("attackerId")
                            ? assignment.get("attackerId").getAsString()
                            : "";
                    final Card selectedCard = findCardByPublishedId(availableAttackers, attackerId);
                    if (selectedCard != null && !selected.contains(selectedCard)) {
                        selected.add(selectedCard);
                    }
                }
            }
            return selected;
        }
        return new CardCollection();
    }

    CardCollection awaitCardChoice(
            final String kind,
            final int playerId,
            final CardCollectionView validCards,
            final int min,
            final int max
    ) {
        requireAttached();
        final List<Card> cards = ParityOrder.sortCardsByNameThenId(new ArrayList<Card>(validCards));
        publishCardChoicePrompt(kind, playerId, cards, min, max);
        return awaitCardsFromPublishedPrompt(cards, min, max);
    }

    private CardCollection awaitCardsFromPublishedPrompt(
            final List<Card> cards,
            final int min,
            final int max
    ) {
        while (!closed && !game.isGameOver()) {
            final JsonObject action;
            try {
                action = actions.take();
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                return new CardCollection();
            }
            final String actionKind = action.has("kind") ? action.get("kind").getAsString() : "";
            if ("pass".equals(actionKind) || "pass_priority".equals(actionKind)) {
                return new CardCollection(cards.subList(0, Math.min(min, cards.size())));
            }
            if (!"choose_cards".equals(actionKind)) {
                throw new UnsupportedOperationException("unsupported action kind: " + actionKind);
            }
            final CardCollection selected = new CardCollection();
            if (action.has("card_ids") && action.get("card_ids").isJsonArray()) {
                for (JsonElement element : action.getAsJsonArray("card_ids")) {
                    final String cardId = element.getAsString();
                    final Card selectedCard = findCardByPublishedId(cards, cardId);
                    if (selectedCard != null) {
                        selected.add(selectedCard);
                    }
                }
            }
            if (selected.size() < min || selected.size() > max) {
                throw new IllegalArgumentException("selected card count out of range: " + selected.size());
            }
            return selected;
        }
        return new CardCollection();
    }

    private void publishPriorityPrompt(final int playerId, final List<SpellAbility> actionsForPrompt) {
        JsonObject prompt = new JsonObject();
        prompt.addProperty("kind", "priority");
        prompt.addProperty("sessionId", sessionId);
        prompt.addProperty("player", playerId);
        prompt.add("snapshot", JsonParser.parseString(InteractiveSnapshotExtractor.snapshotJson(game)));
        final List<String> labels = ActionSpace.buildMainActionLabels(actionsForPrompt);
        com.google.gson.JsonArray options = new com.google.gson.JsonArray();
        for (int i = 0; i < actionsForPrompt.size(); i++) {
            JsonObject option = new JsonObject();
            option.addProperty("index", i);
            option.addProperty("label", labels.get(i));
            options.add(option);
        }
        prompt.add("actions", options);
        latestPromptJson = prompt.toString();
    }

    private void publishCardChoicePrompt(
            final String kind,
            final int playerId,
            final List<Card> cards,
            final int min,
            final int max
    ) {
        JsonObject prompt = new JsonObject();
        prompt.addProperty("kind", kind);
        prompt.addProperty("sessionId", sessionId);
        prompt.addProperty("player", playerId);
        prompt.addProperty("min", min);
        prompt.addProperty("max", max);
        prompt.add("snapshot", JsonParser.parseString(InteractiveSnapshotExtractor.snapshotJson(game)));
        com.google.gson.JsonArray options = new com.google.gson.JsonArray();
        for (int i = 0; i < cards.size(); i++) {
            JsonObject option = new JsonObject();
            option.addProperty("index", i);
            option.addProperty("id", SnapshotExtractor.javaCardId(cards.get(i)));
            option.addProperty("label", cards.get(i).getName());
            options.add(option);
        }
        prompt.add("cards", options);
        latestPromptJson = prompt.toString();
    }

    private void publishCardChoicePrompt(
            final String kind,
            final int playerId,
            final List<Card> cards,
            final int min,
            final int max,
            final int count
    ) {
        JsonObject prompt = new JsonObject();
        prompt.addProperty("kind", kind);
        prompt.addProperty("sessionId", sessionId);
        prompt.addProperty("player", playerId);
        prompt.addProperty("min", min);
        prompt.addProperty("max", max);
        prompt.addProperty("count", count);
        prompt.add("snapshot", JsonParser.parseString(InteractiveSnapshotExtractor.snapshotJson(game)));
        com.google.gson.JsonArray options = new com.google.gson.JsonArray();
        for (int i = 0; i < cards.size(); i++) {
            JsonObject option = new JsonObject();
            option.addProperty("index", i);
            option.addProperty("id", SnapshotExtractor.javaCardId(cards.get(i)));
            option.addProperty("label", cards.get(i).getName());
            options.add(option);
        }
        prompt.add("cards", options);
        latestPromptJson = prompt.toString();
    }

    private void publishAttackersPrompt(
            final int playerId,
            final Combat combat,
            final List<Card> availableAttackers
    ) {
        JsonObject prompt = new JsonObject();
        prompt.addProperty("kind", "choose_attackers");
        prompt.addProperty("sessionId", sessionId);
        prompt.addProperty("player", playerId);
        prompt.add("snapshot", JsonParser.parseString(InteractiveSnapshotExtractor.snapshotJson(game)));
        com.google.gson.JsonArray attackers = new com.google.gson.JsonArray();
        for (int i = 0; i < availableAttackers.size(); i++) {
            JsonObject option = new JsonObject();
            option.addProperty("index", i);
            option.addProperty("id", SnapshotExtractor.javaCardId(availableAttackers.get(i)));
            option.addProperty("label", availableAttackers.get(i).getName());
            attackers.add(option);
        }
        prompt.add("attackers", attackers);

        com.google.gson.JsonArray defenders = new com.google.gson.JsonArray();
        for (final GameEntity defender : combat.getDefenders()) {
            JsonObject option = new JsonObject();
            option.addProperty("id", defenderId(defender));
            option.addProperty("label", defender.getName());
            defenders.add(option);
        }
        prompt.add("defenders", defenders);
        latestPromptJson = prompt.toString();
    }

    private String defenderId(final GameEntity defender) {
        if (defender instanceof Player) {
            return "player-" + SnapshotExtractor.playerIndex(game, (Player) defender);
        }
        return "defender-" + defender.getId();
    }

    private static Card findCardByPublishedId(final List<Card> cards, final String cardId) {
        final int parityId = parseJavaCardParityId(cardId);
        if (parityId >= 0) {
            for (final Card card : cards) {
                if (ParityCardMap.parityId(card) == parityId) {
                    return card;
                }
            }
        }
        final int index = parseJavaCardIndex(cardId);
        if (index >= 0 && index < cards.size()) {
            return cards.get(index);
        }
        return null;
    }

    private static int parseJavaCardParityId(final String cardId) {
        final String prefix = cardId.startsWith("engine-card-") ? "engine-card-" : "java-card-";
        if (!cardId.startsWith(prefix)) {
            return -1;
        }
        final String suffix = cardId.substring(prefix.length());
        if (suffix.contains("-")) {
            return -1;
        }
        try {
            return Integer.parseInt(suffix);
        } catch (NumberFormatException error) {
            return -1;
        }
    }

    private static int parseJavaCardIndex(final String cardId) {
        final int marker = cardId.lastIndexOf("-hand-");
        if (marker < 0) {
            return -1;
        }
        try {
            return Integer.parseInt(cardId.substring(marker + "-hand-".length()));
        } catch (NumberFormatException error) {
            return -1;
        }
    }

    private void requireAttached() {
        if (match == null || game == null) {
            throw new IllegalStateException("session is not attached to a Forge game");
        }
    }
}
