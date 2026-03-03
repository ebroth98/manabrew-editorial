package forge.harness;

import forge.card.CardDb;
import forge.deck.CardPool;
import forge.deck.Deck;
import forge.deck.DeckSection;
import forge.item.PaperCard;
import forge.model.FModel;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

/**
 * Builds preset decks from shared JSON files in the preset_decks/ directory.
 * Card lists are the single source of truth shared with the Rust parity runner.
 */
public final class PresetDecks {
    private PresetDecks() {}

    /** Default directory for preset deck JSON files (relative to CWD). */
    private static final String DEFAULT_DECKS_DIR = "preset_decks";

    private static String getDecksDir() {
        String dir = System.getProperty("preset.decks.dir");
        if (dir != null && !dir.isEmpty()) {
            return dir;
        }
        dir = System.getenv("PRESET_DECKS_DIR");
        if (dir != null && !dir.isEmpty()) {
            return dir;
        }
        return DEFAULT_DECKS_DIR;
    }

    /**
     * Build a Deck from a preset name or inline spec. Returns null if the preset is unknown.
     *
     * Supports inline deck specs with the "inline:" prefix:
     *   "inline:Mountain*17|Lightning Bolt*4|Shock*4"
     */
    public static Deck buildDeck(String presetName) {
        if (presetName.startsWith("inline:")) {
            return buildInlineDeck(presetName.substring(7));
        }

        String decksDir = getDecksDir();
        Path jsonPath = Paths.get(decksDir, presetName + ".json");
        if (!Files.exists(jsonPath)) {
            return null;
        }

        String contents;
        try {
            contents = Files.readString(jsonPath);
        } catch (IOException e) {
            System.err.println("[harness] WARNING: Failed to read " + jsonPath + ": " + e.getMessage());
            return null;
        }

        JsonObject root = JsonParser.parseString(contents).getAsJsonObject();
        JsonArray cards = root.getAsJsonArray("cards");

        Deck deck = new Deck(presetName);
        CardPool main = deck.getOrCreate(DeckSection.Main);
        CardDb cardDb = FModel.getMagicDb().getCommonCards();

        for (JsonElement elem : cards) {
            JsonObject entry = elem.getAsJsonObject();
            String name = entry.get("name").getAsString();
            int count = entry.get("count").getAsInt();

            PaperCard card = cardDb.getCard(name);
            if (card == null) {
                System.err.println("[harness] WARNING: Card not found: " + name);
                continue;
            }
            main.add(card, count);
        }

        return deck;
    }

    /**
     * Build a Deck from an inline spec string: "Name*Count|Name*Count|..."
     * Uses '|' as delimiter because MTG card names can contain commas.
     */
    private static Deck buildInlineDeck(String spec) {
        Deck deck = new Deck("inline");
        CardPool main = deck.getOrCreate(DeckSection.Main);
        CardDb cardDb = FModel.getMagicDb().getCommonCards();

        for (String entry : spec.split("\\|")) {
            entry = entry.trim();
            if (entry.isEmpty()) continue;

            int lastStar = entry.lastIndexOf('*');
            if (lastStar < 0) {
                System.err.println("[harness] WARNING: Invalid inline entry (no '*'): " + entry);
                continue;
            }

            String name = entry.substring(0, lastStar);
            int count;
            try {
                count = Integer.parseInt(entry.substring(lastStar + 1));
            } catch (NumberFormatException e) {
                System.err.println("[harness] WARNING: Invalid count in entry: " + entry);
                continue;
            }

            PaperCard card = cardDb.getCard(name);
            if (card == null) {
                System.err.println("[harness] WARNING: Card not found: " + name);
                continue;
            }
            main.add(card, count);
        }

        return deck;
    }

    /**
     * Returns all available preset names by scanning the preset_decks/ directory.
     */
    public static String[] availablePresets() {
        String decksDir = getDecksDir();
        File dir = new File(decksDir);
        if (!dir.isDirectory()) {
            return new String[0];
        }
        File[] files = dir.listFiles((d, name) -> name.endsWith(".json"));
        if (files == null) {
            return new String[0];
        }
        List<String> names = new ArrayList<>();
        for (File f : files) {
            String stem = f.getName().replace(".json", "");
            names.add(stem);
        }
        names.sort(String::compareTo);
        return names.toArray(new String[0]);
    }
}
