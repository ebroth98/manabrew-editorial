package forge.harness;

import forge.StaticData;
import forge.card.CardDb;
import forge.deck.CardPool;
import forge.deck.Deck;
import forge.deck.DeckSection;
import forge.item.PaperCard;
import forge.model.FModel;

import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Builds the same preset decks used by the Rust parity runner.
 * Card lists must match exactly for snapshot comparison.
 */
public final class PresetDecks {
    private PresetDecks() {}

    private static final Map<String, String[][]> PRESETS = new LinkedHashMap<>();

    static {
        PRESETS.put("red_burn", new String[][] {
            {"Mountain", "17"},
            {"Lightning Bolt", "4"},
            {"Shock", "4"},
            {"Gray Ogre", "3"},
            {"Hill Giant", "3"},
            {"Guttersnipe", "3"},
        });

        PRESETS.put("green_stompy", new String[][] {
            {"Forest", "17"},
            {"Giant Growth", "4"},
            {"Grizzly Bears", "3"},
            {"Centaur Courser", "2"},
            {"Garruk's Companion", "3"},
            {"Giant Spider", "2"},
            {"Wall of Ice", "2"},
            {"Craw Wurm", "2"},
        });

        PRESETS.put("white_aggro", new String[][] {
            {"Plains", "17"},
            {"Savannah Lions", "4"},
            {"White Knight", "3"},
            {"Serra Angel", "3"},
            {"Soul Warden", "3"},
        });

        PRESETS.put("black_control", new String[][] {
            {"Swamp", "17"},
            {"Doom Blade", "4"},
            {"Dark Ritual", "2"},
            {"Hypnotic Specter", "3"},
            {"Sengir Vampire", "2"},
        });
    }

    /**
     * Build a Deck from a preset name. Returns null if the preset is unknown.
     */
    public static Deck buildDeck(String presetName) {
        String[][] cards = PRESETS.get(presetName);
        if (cards == null) {
            return null;
        }

        Deck deck = new Deck(presetName);
        CardPool main = deck.getOrCreate(DeckSection.Main);
        CardDb cardDb = FModel.getMagicDb().getCommonCards();

        for (String[] entry : cards) {
            String name = entry[0];
            int count = Integer.parseInt(entry[1]);

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
     * Returns all available preset names.
     */
    public static String[] availablePresets() {
        return PRESETS.keySet().toArray(new String[0]);
    }
}
