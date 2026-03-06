use super::EffectContext;
use crate::spellability::SpellAbility;

/// `SP$ ChooseType` — the activating player chooses a creature type, card type, etc.
/// Stores the result in `source.chosen_type` for subsequent effects.
///
/// Mirrors Java's `ChooseTypeEffect.java`.
/// - `Type$` — the category of type to choose: "Creature", "Card", "Land", etc.
/// - `ValidTypes$` — optional comma-separated list of valid types (overrides auto-list).
///
/// # Card script examples
/// ```text
/// A:SP$ ChooseType | Type$ Creature
/// A:SP$ ChooseType | Type$ Card | ValidTypes$ Artifact,Creature,Enchantment,Land,Planeswalker
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let type_category = sa
        .params
        .get("Type")
        .cloned()
        .unwrap_or_else(|| "Creature".to_string());

    // Build the valid types list
    let valid_types: Vec<String> = if let Some(vt) = sa.params.get("ValidTypes") {
        vt.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        match type_category.as_str() {
            "Creature" => default_creature_types(),
            "Land" => vec![
                "Plains".into(),
                "Island".into(),
                "Swamp".into(),
                "Mountain".into(),
                "Forest".into(),
            ],
            _ => vec![
                "Artifact".into(),
                "Creature".into(),
                "Enchantment".into(),
                "Instant".into(),
                "Land".into(),
                "Planeswalker".into(),
                "Sorcery".into(),
            ],
        }
    };

    if valid_types.is_empty() {
        return;
    }

    let chosen =
        ctx.agents[controller.index()].choose_type(controller, &type_category, &valid_types);

    if let Some(chosen_type) = chosen {
        if let Some(source_id) = sa.source {
            let source = ctx.game.card_mut(source_id);
            source.chosen_type = Some(chosen_type);
            source.chosen_type_controller = Some(controller);
            source.chosen_type_revealed = false;
        }
    }
}

/// Default creature types for ChooseType (common types used in MTG).
pub fn default_creature_types() -> Vec<String> {
    vec![
        "Advisor",
        "Angel",
        "Ape",
        "Archer",
        "Assassin",
        "Avatar",
        "Beast",
        "Berserker",
        "Bird",
        "Boar",
        "Cat",
        "Centaur",
        "Cleric",
        "Construct",
        "Crab",
        "Demon",
        "Dinosaur",
        "Djinn",
        "Dog",
        "Dragon",
        "Drake",
        "Druid",
        "Dwarf",
        "Elemental",
        "Elephant",
        "Elf",
        "Faerie",
        "Fish",
        "Fox",
        "Frog",
        "Fungus",
        "Giant",
        "Gnome",
        "Goblin",
        "God",
        "Golem",
        "Griffin",
        "Horror",
        "Human",
        "Hydra",
        "Illusion",
        "Imp",
        "Insect",
        "Jellyfish",
        "Knight",
        "Lizard",
        "Merfolk",
        "Minotaur",
        "Monk",
        "Mutant",
        "Myr",
        "Ninja",
        "Noble",
        "Ogre",
        "Orc",
        "Otter",
        "Ox",
        "Pegasus",
        "Phoenix",
        "Pilot",
        "Pirate",
        "Plant",
        "Praetor",
        "Rat",
        "Rebel",
        "Rogue",
        "Salamander",
        "Samurai",
        "Scout",
        "Serpent",
        "Shade",
        "Shaman",
        "Shapeshifter",
        "Skeleton",
        "Sliver",
        "Snake",
        "Soldier",
        "Sphinx",
        "Spider",
        "Spirit",
        "Squirrel",
        "Thopter",
        "Treefolk",
        "Troll",
        "Turtle",
        "Unicorn",
        "Vampire",
        "Vedalken",
        "Viashino",
        "Wall",
        "Warrior",
        "Werewolf",
        "Wizard",
        "Wolf",
        "Wolverine",
        "Wurm",
        "Zombie",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
