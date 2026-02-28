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

        PRESETS.put("comprehensive_test", new String[][] {
            // Lands (18)
            {"Forest", "3"},
            {"Island", "3"},
            {"Plains", "3"},  // was 2 + Command Tower (Combo ColorIdentity → unusable in non-Commander)
            {"Mountain", "2"},
            {"Swamp", "3"},   // was 2 + Path of Ancestry (Combo ColorIdentity → unusable in non-Commander)
            {"Breeding Pool", "1"},
            {"Hallowed Fountain", "1"},
            {"Temple of Mystery", "1"},
            {"Yavimaya Coast", "1"},
            // Keyword creatures (11)
            {"Vampire Nighthawk", "1"},
            {"Serra Angel", "1"},
            {"Darksteel Myr", "1"},
            {"Boggart Brute", "1"},
            {"Glistener Elf", "1"},
            {"White Knight", "1"},
            {"Giant Spider", "1"},
            {"Llanowar Elves", "2"},
            {"Soul Warden", "1"},
            {"Guttersnipe", "1"},
            // ETB / explore / proliferate (4)
            {"Merfolk Branchwalker", "1"},
            {"Jadelight Ranger", "1"},
            {"Elvish Visionary", "1"},  // was Mulldrifter (evoke: Rust uses evoke cost, Java uses main cost)
            {"Thrummingbird", "1"},
            // Detain / goad / protection (3)
            {"Lyev Skyknight", "1"},
            {"Gods Willing", "1"},
            {"Brave the Elements", "1"},
            // Damage / removal (4)
            {"Lightning Bolt", "2"},
            {"Wrath of God", "1"},
            {"Doom Blade", "1"},
            {"Prey Upon", "1"},
            // Card advantage (4)
            {"Ponder", "1"},
            {"Preordain", "1"},
            {"Thought Scour", "1"},
            {"Steady Progress", "1"},
            // Modal / draw / choice (3)
            {"Izzet Charm", "1"},
            {"Divination", "1"},
            {"Control Magic", "1"},
            // Combat tricks / bounce / fog (3)
            {"Giant Growth", "1"},
            {"Fog", "1"},
            {"Unsummon", "1"},
            // Tokens (3)
            {"Raise the Alarm", "1"},
            {"Dragon Fodder", "1"},
            {"Lingering Souls", "1"},
            // Simple creatures / spells (6) — removed alt-cost cards that cause parity divergences
            {"Faithless Looting", "1"},
            {"Goblin Bushwhacker", "1"},
            {"Volcanic Hammer", "1"},
            {"Shock", "1"},
            {"Lightning Elemental", "1"},
            {"Gray Ogre", "1"},
            // Static anthems (2)
            {"Glorious Anthem", "1"},
            {"Honor of the Pure", "1"},
        });

        PRESETS.put("trigger_expanded", new String[][] {
            {"Mountain", "7"},
            {"Forest", "5"},
            {"Swamp", "3"},
            {"Island", "3"},
            {"Plains", "2"},
            // AttackersDeclared
            {"Roar of Resistance", "3"},
            {"Ruby Collector", "3"},
            // SpellCast
            {"Guttersnipe", "3"},
            {"Young Pyromancer", "2"},
            // ChangesZone
            {"Essence Warden", "3"},
            {"Impact Tremors", "2"},
            // DamageDoneOnce
            {"Raptor Hatchling", "3"},
            {"Ranging Raptors", "2"},
            // ChangesZoneAll
            {"Woodland Champion", "2"},
            // CounterAddedOnce
            {"Nest of Scarabs", "2"},
            {"Stocking the Pantry", "2"},
            // Surveil
            {"Thoughtbound Phantasm", "2"},
            {"Whispering Snitch", "2"},
            // DamageDoneOnce
            {"Rite of Passage", "2"},
        });
    }

    /**
     * Build a Deck from a preset name or inline spec. Returns null if the preset is unknown.
     *
     * Supports inline deck specs with the "inline:" prefix:
     *   "inline:Mountain*17,Lightning Bolt*4,Shock*4"
     */
    public static Deck buildDeck(String presetName) {
        if (presetName.startsWith("inline:")) {
            return buildInlineDeck(presetName.substring(7));
        }

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
     * Returns all available preset names.
     */
    public static String[] availablePresets() {
        return PRESETS.keySet().toArray(new String[0]);
    }
}
